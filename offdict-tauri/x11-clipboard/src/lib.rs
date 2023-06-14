extern crate x11rb;

pub mod error;
mod run;

pub use x11rb::protocol::xproto::{Atom, Window};
pub use x11rb::rust_connection::RustConnection;

use error::Error;
use std::collections::HashMap;
use std::sync::mpsc::{channel, Sender};
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::{Duration, Instant};
use x11rb::connection::{Connection, RequestConnection};
use x11rb::errors::ConnectError;
use x11rb::protocol::xproto::{
    AtomEnum, ConnectionExt, CreateWindowAux, EventMask, Property, WindowClass,
};
use x11rb::protocol::{xfixes, Event};
use x11rb::{COPY_DEPTH_FROM_PARENT, CURRENT_TIME};

pub const INCR_CHUNK_SIZE: usize = 4000;
const POLL_DURATION: u64 = 50;
type SetMap = Arc<RwLock<HashMap<Atom, (Atom, Vec<u8>)>>>;

#[derive(Clone, Debug)]
pub struct Atoms {
    pub primary: Atom,
    pub clipboard: Atom,
    pub property: Atom,
    pub targets: Atom,
    pub string: Atom,
    pub utf8_string: Atom,
    pub incr: Atom,
}

impl Atoms {
    fn intern_all(conn: &RustConnection) -> Result<Atoms, Error> {
        let clipboard = conn.intern_atom(false, b"CLIPBOARD")?;
        let property = conn.intern_atom(false, b"THIS_CLIPBOARD_OUT")?;
        let targets = conn.intern_atom(false, b"TARGETS")?;
        let utf8_string = conn.intern_atom(false, b"UTF8_STRING")?;
        let incr = conn.intern_atom(false, b"INCR")?;
        Ok(Atoms {
            primary: Atom::from(AtomEnum::PRIMARY),
            clipboard: clipboard.reply()?.atom,
            property: property.reply()?.atom,
            targets: targets.reply()?.atom,
            string: Atom::from(AtomEnum::STRING),
            utf8_string: utf8_string.reply()?.atom,
            incr: incr.reply()?.atom,
        })
    }
}

/// X11 Clipboard
pub struct Clipboard {
    pub getter: Context,
    pub setter: Arc<Context>,
    setmap: SetMap,
    send: Sender<Atom>,
}

pub struct Context {
    pub connection: RustConnection,
    pub screen: usize,
    pub window: Window,
    pub atoms: Atoms,
}

#[inline]
fn get_atom(connection: &RustConnection, name: &str) -> Result<Atom, Error> {
    let intern_atom = connection.intern_atom(false, name.as_bytes())?;
    let reply = intern_atom.reply().map_err(Error::XcbReply)?;
    Ok(reply.atom)
}

impl Context {
    pub fn new(displayname: Option<&str>) -> Result<Self, Error> {
        let (connection, screen) = RustConnection::connect(displayname)?;
        let window = connection.generate_id()?;
        {
            let screen = connection
                .setup()
                .roots
                .get(screen)
                .ok_or(Error::XcbConnect(ConnectError::InvalidScreen))?;
            connection
                .create_window(
                    COPY_DEPTH_FROM_PARENT,
                    window,
                    screen.root,
                    0,
                    0,
                    1,
                    1,
                    0,
                    WindowClass::INPUT_OUTPUT,
                    screen.root_visual,
                    &CreateWindowAux::new()
                        .event_mask(EventMask::STRUCTURE_NOTIFY | EventMask::PROPERTY_CHANGE),
                )?
                .check()?;
        }

        let atoms = Atoms::intern_all(&connection)?;

        Ok(Context {
            connection,
            screen,
            window,
            atoms,
        })
    }

    pub fn get_atom(&self, name: &str) -> Result<Atom, Error> {
        get_atom(&self.connection, name)
    }
}

impl Clipboard {
    /// Create Clipboard.
    pub fn new() -> Result<Self, Error> {
        let getter = Context::new(None)?;
        let setter = Arc::new(Context::new(None)?);
        let setter2 = Arc::clone(&setter);
        let setmap = Arc::new(RwLock::new(HashMap::new()));
        let setmap2 = Arc::clone(&setmap);

        let (sender, receiver) = channel();
        let max_length = setter.connection.maximum_request_bytes();
        thread::spawn(move || run::run(&setter2, &setmap2, max_length, &receiver));

        Ok(Clipboard {
            getter,
            setter,
            setmap,
            send: sender,
        })
    }

    fn process_event<T>(
        &self,
        buff: &mut Vec<u8>,
        selection: Vec<Atom>,
        target: Atom,
        property: Atom,
        timeout: T,
        use_xfixes: bool,
    ) -> Result<bool, Error>
    where
        T: Into<Option<Duration>>,
    {
        let mut is_incr = false;
        let timeout = timeout.into();
        let start_time = if timeout.is_some() {
            Some(Instant::now())
        } else {
            None
        };

        let mut explicit: bool = false;
        let mut ignore_primary = false;
        let mut ignore_all = false; 

        loop {
            if timeout
                .into_iter()
                .zip(start_time)
                .next()
                .map(|(timeout, time)| (Instant::now() - time) >= timeout)
                .unwrap_or(false)
            {
                return Err(Error::Timeout);
            }

            let event = match use_xfixes {
                true => self.getter.connection.wait_for_event()?,
                false => match self.getter.connection.poll_for_event()? {
                    Some(event) => event,
                    None => {
                        thread::park_timeout(Duration::from_millis(POLL_DURATION));
                        continue;
                    }
                },
            };

            match event {
                Event::XfixesSelectionNotify(event) if use_xfixes => {
                    // for sel in selection.clone() {
                    self.getter
                        .connection
                        .convert_selection(
                            self.getter.window,
                            event.selection,
                            target,
                            property,
                            event.timestamp,
                        )?
                        .check()?;
                    let name = String::from_utf8(
                        self.getter
                            .connection
                            .get_property(
                                false,
                                event.owner,
                                AtomEnum::WM_NAME,
                                AtomEnum::STRING,
                                0,
                                256,
                            )?
                            .reply()?
                            .value,
                    )
                    .unwrap_or_default();
                    ignore_primary = name == "Chromium clipboard";
                    ignore_all = name.is_empty(); // Ignore windows that have no name. They are probably some weird hacks.
                    println!("owner {}, {}; ignore_primary, {}, ignore_all, {}", name, event.owner, ignore_primary, ignore_all);

                }
                Event::SelectionNotify(event) => {
                    if !selection.contains(&event.selection) {
                        continue;
                    };
                    if event.selection == self.getter.atoms.clipboard {
                        explicit = true;
                    } else {
                        if ignore_primary {
                            continue;
                        }
                    }
                    if ignore_all {
                        continue;
                    }

                    // Note that setting the property argument to None indicates that the
                    // conversion requested could not be made.
                    if event.property == Atom::from(AtomEnum::NONE) {
                        break;
                    }

                    let reply = self
                        .getter
                        .connection
                        .get_property(
                            false,
                            self.getter.window,
                            event.property,
                            AtomEnum::NONE,
                            buff.len() as u32,
                            u32::MAX,
                        )?
                        .reply()?;

                    if reply.type_ == self.getter.atoms.incr {
                        if let Some(mut value) = reply.value32() {
                            if let Some(size) = value.next() {
                                buff.reserve(size as usize);
                            }
                        }
                        self.getter
                            .connection
                            .delete_property(self.getter.window, property)?
                            .check()?;
                        is_incr = true;
                        continue;
                    } else if reply.type_ != target {
                        return Err(Error::UnexpectedType(reply.type_));
                    }

                    buff.extend_from_slice(&reply.value);
                    break;
                }
                // TODO: One buffer per atom (event.sequence). It's unlikely to conflict tho
                Event::PropertyNotify(event) if is_incr => {
                    if event.state != Property::NEW_VALUE {
                        continue;
                    };

                    let cookie = self.getter.connection.get_property(
                        false,
                        self.getter.window,
                        property,
                        AtomEnum::NONE,
                        0,
                        0,
                    )?;

                    let length = cookie.reply()?.bytes_after;

                    let cookie = self.getter.connection.get_property(
                        true,
                        self.getter.window,
                        property,
                        AtomEnum::NONE,
                        0,
                        length,
                    )?;
                    let reply = cookie.reply()?;
                    if reply.type_ != target {
                        continue;
                    };

                    let value = reply.value;

                    if !value.is_empty() {
                        buff.extend_from_slice(&value);
                    } else {
                        break;
                    }
                }
                _ => {
                    // dbg!(&event);
                }
            }
        }
        Ok(explicit)
    }

    /// load value.
    pub fn load<T>(
        &self,
        selection: Vec<Atom>,
        target: Atom,
        property: Atom,
        timeout: T,
    ) -> Result<Vec<u8>, Error>
    where
        T: Into<Option<Duration>>,
    {
        let mut buff = Vec::new();
        let timeout = timeout.into();

        for sel in selection.clone() {
            self.getter
                .connection
                .convert_selection(
                    self.getter.window,
                    sel,
                    target,
                    property,
                    CURRENT_TIME,
                    // FIXME ^
                    // Clients should not use CurrentTime for the time argument of a ConvertSelection request.
                    // Instead, they should use the timestamp of the event that caused the request to be made.
                )?
                .check()?;
        }

        self.process_event(&mut buff, selection, target, property, timeout, false)?;

        self.getter
            .connection
            .delete_property(self.getter.window, property)?
            .check()?;

        Ok(buff)
    }

    /// wait for a new value and load it
    pub fn load_wait(
        &self,
        selection: Vec<Atom>,
        target: Atom,
        property: Atom,
    ) -> Result<(Vec<u8>, bool), Error> {
        let mut buff = Vec::new();

        let screen = &self
            .getter
            .connection
            .setup()
            .roots
            .get(self.getter.screen)
            .ok_or(Error::XcbConnect(ConnectError::InvalidScreen))?;

        xfixes::query_version(&self.getter.connection, 5, 0)?;
        // Clear selection sources...
        xfixes::select_selection_input(
            &self.getter.connection,
            screen.root,
            self.getter.atoms.primary,
            xfixes::SelectionEventMask::default(),
        )?;
        xfixes::select_selection_input(
            &self.getter.connection,
            screen.root,
            self.getter.atoms.clipboard,
            xfixes::SelectionEventMask::default(),
        )?;
        // ...and set the one requested now
        for sel in selection.clone() {
            xfixes::select_selection_input(
                &self.getter.connection,
                screen.root,
                sel,
                xfixes::SelectionEventMask::SET_SELECTION_OWNER
                    | xfixes::SelectionEventMask::SELECTION_CLIENT_CLOSE
                    | xfixes::SelectionEventMask::SELECTION_WINDOW_DESTROY,
            )?
            .check()?;
        }
        let exp = self.process_event(&mut buff, selection, target, property, None, true)?;

        self.getter
            .connection
            .delete_property(self.getter.window, property)?
            .check()?;

        Ok((buff, exp))
    }

    /// store value.
    pub fn store<T: Into<Vec<u8>>>(
        &self,
        selection: Atom,
        target: Atom,
        value: T,
    ) -> Result<(), Error> {
        self.send.send(selection)?;
        self.setmap
            .write()
            .map_err(|_| Error::Lock)?
            .insert(selection, (target, value.into()));

        self.setter
            .connection
            .set_selection_owner(self.setter.window, selection, CURRENT_TIME)?
            .check()?;

        if self
            .setter
            .connection
            .get_selection_owner(selection)?
            .reply()
            .map(|reply| reply.owner == self.setter.window)
            .unwrap_or(false)
        {
            Ok(())
        } else {
            Err(Error::Owner)
        }
    }
}
