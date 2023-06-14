use crate::{CallbackResult, ClipboardHandler, Master};
use std::io;

impl<H: ClipboardHandler> Master<H> {
    ///Starts Master by waiting for any change
    pub fn run(&mut self) -> io::Result<()> {
        let mut result = Ok(());
        let clipboard = x11_clipboard::Clipboard::new().unwrap();
        // TODO: Add wayland
        loop {
            if let Ok((curr, exp)) = clipboard.load_wait(
                vec![
                    clipboard.getter.atoms.primary,
                    clipboard.getter.atoms.clipboard,
                ],
                clipboard.getter.atoms.utf8_string, 
                clipboard.getter.atoms.property,
            ) {
                let curr = String::from_utf8_lossy(&curr);
                let curr = curr.trim_matches('\u{0}').trim();
                if curr.is_empty() {
                    continue;
                }
                match self
                    .handler
                    .on_clipboard_change(Some(curr.to_owned()), exp)
                {
                    CallbackResult::Next => (),
                    CallbackResult::Stop => break,
                    CallbackResult::StopWithError(error) => {
                        result = Err(error);
                        break;
                    }
                }
            }
        }

        // xcb::delete_property(
        //     &clipboard.getter.connection,
        //     clipboard.getter.window,
        //     clipboard.getter.atoms.property,
        // );

        result
    }
}
