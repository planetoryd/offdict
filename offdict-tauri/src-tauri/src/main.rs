#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]
use clipboard_master::{CallbackResult, ClipboardHandler, Master};

use gdkx11::gdk::{ffi::GDK_CURRENT_TIME, ModifierType};
use gtk::{
    gdk,
    traits::{GtkWindowExt, WidgetExt},
    ApplicationWindow,
};
use gtk::{prelude::*, HeaderBar};
// use lazy_regex;
use lazy_regex::{lazy_regex, Lazy};
use offdictd::{self, def_bin::WrapperDef, *};
use rust_stemmers::{Algorithm, Stemmer};
use std::io;
use std::{
    borrow::Cow, collections::BTreeSet, env, fs, iter::FromIterator, path::PathBuf, sync::Arc,
    sync::RwLock, thread, time::Instant,
};
use tauri::{
    self, api::dialog, regex::Regex, utils::debug_eprintln, ClipboardManager,
    GlobalShortcutManager, Manager, PhysicalPosition, Window, WindowEvent,
};
use tauri_plugin_positioner::{Position, WindowExt};
use timed::timed;

struct Handler<T: ClipboardManager> {
    app: Window,
    clip: T,
    last: String,
    en_stemmer: Stemmer,
}

use gtk::glib;

static mut pos: Option<PhysicalPosition<i32>> = None;
pub static re1: Lazy<Regex> = lazy_regex!(r"[;\[\]{}<>#@$%^&*/\\:]");
pub static re2: Lazy<Regex> = lazy_regex!(r"[;\[\]{}<>#@$%^&*/\\:,.?!。，]");

impl<T: ClipboardManager> ClipboardHandler for Handler<T> {
    fn on_clipboard_change(&mut self, mut k: String) -> CallbackResult {
        println!("clip_m: {}", k);

        if k.is_empty()  {
            k = self.clip.read_text().unwrap().unwrap_or_default();
        }

        if k == self.last {
            return CallbackResult::Next;
        }
        self.last = k.clone();
        // Clean up raw clipboard content, do fuzzy search, skip if no result, and stem, repeat.
        // let r = self.en_stemmer.stem(cleanup_clipboard_input(&k));
        if denied_clip(&k) {
            return CallbackResult::Next;
        }

        let r: Cow<str> = Cow::Owned(cleanup_clipboard_input(k.clone()));

        println!("clip: {}", r.as_ref());

        if r.is_empty() {
            return CallbackResult::Next;
        }
        // self.app.emit("clip", r.as_ref()).unwrap();
        // self.app.unminimize().unwrap();
        // self.app.show().unwrap();
        // let win = self.app.gtk_window().unwrap();
        // win.animation
        // win.present();
        // if !self.app.is_visible().unwrap() {
        //     self.app.show().unwrap();
        // }
        if onInput(&k, false) {
            // doesn't pop up when no results
            restore_pos(&self.app);
            self.app.set_always_on_top(true).unwrap();
            self.app.show().unwrap();
        }

        // doesnt really work on kde, only sets it glowy zxcsaz
        // self.app.set_focus().expect("cannot focus window");
        // https://stackoverflow.com/questions/66510406/gtk-rs-how-to-update-view-from-another-thread
        glib::idle_add(move || unsafe {
            ENTRY.as_ref().unwrap().set_text(&r);
            glib::source::Continue(false)
        });

        CallbackResult::Next
    }

    fn on_clipboard_error(&mut self, error: io::Error) -> CallbackResult {
        println!("clip, error: {}", error);
        CallbackResult::Next
    }
}

fn cleanup_clipboard_input(s: String) -> String {
    let trimmed = s.trim().to_owned();
    let removed = re2.replace_all(&trimmed, "");
    removed.into_owned()
}

fn denied_clip(k: &str) -> bool {
    k.len() > 25 || re1.is_match(&k)
}

#[test]
fn test_clip() {
    assert!(denied_clip("io::Error"));
    assert!(!denied_clip("self.app.show"));
    assert!(!denied_clip("Concretely,"));
    assert_eq!(
        cleanup_clipboard_input("   c,   ".to_owned()),
        "c".to_owned()
    );
}

#[derive(Debug, Deserialize, Clone)]
struct OffdictConfig {
    data_path: String,
    hide_on_blur: bool,
}

pub type InnerState = RwLock<offdict>;

pub struct OffdictState(pub Arc<InnerState>);

// Learn more about Tauri commands at https://tauri.app/v1/guides/features/command

#[timed]
#[tauri::command]
fn input<'a>(
    state: tauri::State<'a, OffdictState>,
    query: &'a str,
    expensive: bool,
) -> Result<(), &'static str> {
    onInput(query, expensive);
    Ok(())
}

#[tauri::command]
fn import<'a>(state: tauri::State<'a, OffdictState>) {
    println!("import");
    // state.0.importing = true;
    let v = state.0.clone();

    dialog::FileDialogBuilder::default()
        .add_filter("tar.gz/yaml", &["tar.gz", "yaml"])
        .pick_folder(move |folder| match folder {
            Some(folder) => {
                thread::spawn(move || {
                    // let mut db_ = v.db.write().unwrap();
                    let mut db = v.write().unwrap();

                    println!("folder picked, {}", folder.display());
                    let paths = fs::read_dir(folder).unwrap();
                    for path in paths {
                        let p: String = path.as_ref().unwrap().path().to_str().unwrap().to_string();
                        if p.ends_with(".yaml") {
                            println!("importing yaml, {}", &p);
                            let pre = path.unwrap().file_name();
                            let name = pre.to_str().unwrap().split_once(".").unwrap();

                            unsafe {
                                w.as_ref().unwrap().emit("importing", &p).unwrap();
                            }

                            db.import_from_file(&p, &name.0.to_string()).unwrap();

                            unsafe {
                                w.as_ref().unwrap().emit("imported", &p).unwrap();
                            }
                        } else if p.ends_with(".yaml.gz") {
                            println!("yaml.gz , {}", p)
                        }
                    }
                });
            }
            None => (),
        });
}

static mut ENTRY: Option<gtk::Entry> = None;
static mut w: Option<Window> = None;
static mut state_: Option<Arc<InnerState>> = None;

const CSS: &[u8] = b"
headerbar entry,
headerbar spinbutton,
headerbar button,
headerbar separator {
    margin-top: 0px; /* same as headerbar side padding for nicer proportions */
    margin-bottom: 0px;
}


headerbar {
    min-height: 0;
    padding-left: 2px; /* same as childrens vertical margins for nicer proportions */
    padding-right: 2px;
    margin: 0px; /* same as headerbar side padding for nicer proportions */
    padding: 0px;
}

.inputheader entry {
    margin: 10px
}

.inputheader button {
	margin: 10 10px 10px 0px;
}
.inputheader {
	padding-left: 25px;
}
               ";

fn save_pos(win: &Window) {
    unsafe {
        pos = Some(win.outer_position().unwrap());
    }
}

fn restore_pos(win: &Window) {
    // unsafe {
    //     win.set_position(pos.unwrap()).unwrap();
    // }
    if !win.is_visible().unwrap() {
        win.move_window(Position::BottomRight)
            .expect("cannot move window");
    }
}

fn plain_header() -> HeaderBar {
    let header = gtk::HeaderBar::builder()
        .opacity(1.0)
        .title("Offdict")
        .visible(true)
        .build();

    header
}

fn input_header(win: Window) -> HeaderBar {
    let header = gtk::HeaderBar::builder()
        .opacity(1.0)
        .visible(true)
        .hexpand(true)
        .build();
    let bo = gtk::Box::builder().visible(true).hexpand(true).build();
    let btn = gtk::Button::builder().visible(true).label("import").build();
    let en = unsafe {
        ENTRY = Some(gtk::Entry::builder().visible(true).build());
        ENTRY.as_ref().unwrap()
    };
    bo.pack_start(en, true, true, 0);
    en.connect_changed(|e| {
        dbg!(e.text());
        onInput(e.text().as_str(), false);
    });
    en.connect_key_press_event(move |e, k| {
        if k.keyval() == gdk::keys::constants::Return {
            println!("expensive search");
            onInput(e.text().as_str(), true);
        } else if k.keyval() == gdk::keys::constants::Escape {
            save_pos(&win);
            win.hide().unwrap();
        }
        Inhibit::default()
    });
    bo.pack_end(&btn, false, false, 0);
    btn.connect_clicked(|b| {
        println!("import btn");
    });
    header.set_custom_title(Some(&bo));
    header.style_context().add_class("inputheader");

    header
}

#[derive(Serialize, Clone)]
struct set_input {
    inputWord: String,
    extensive: bool,
}

// has results ?
fn onInput(s: &str, expensive: bool) -> bool {
    let db_ = unsafe { state_.as_ref().unwrap().read() };

    let db = db_.as_ref().unwrap();

    let mut d = db.search(s, 5, expensive);
    let mut def_list = offdictd::flatten_human(d);
    unsafe {
        let si = set_input {
            inputWord: s.to_owned(),
            extensive: expensive,
        };
        w.as_ref().unwrap().emit("set_input", si).unwrap();
    }

    if def_list.is_empty() {
        false
    } else {
        unsafe {
            w.as_ref().unwrap().emit("def_list", &def_list).unwrap();
        }
        true
    }
}

fn main() {
    let conf = offdictd::config::get_config();
    let db_path = PathBuf::from(conf.data_path.clone());
    println!("{:?}", conf);
    let mut d = offdict::open_db(db_path.to_str().unwrap().to_owned());
    if !offdictd::tui(&mut d).unwrap() {
        return;
    }

    if cfg!(target_os = "linux") {

    } 

    let x = tauri::Builder::default()
        .setup(move |app| {
            println!("{}", env::current_dir().unwrap().to_str().unwrap());
            println!("{:?}", conf);
            let window = app.get_window("main").unwrap();
            let w_on_ev = app.get_window("main").unwrap();
            let w_on_shortcut = app.get_window("main").unwrap();
            let w_on_esc = app.get_window("main").unwrap();

            unsafe {
                w = Some(app.get_window("main").unwrap());
            }

            let win = window.gtk_window().unwrap();
            let header = input_header(w_on_esc);
            let provider = gtk::CssProvider::new();
            provider.load_from_data(&CSS).unwrap();

            gtk::StyleContext::add_provider_for_screen(
                &gdk::Screen::default().expect("Error initializing gtk css provider."),
                &provider,
                gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
            );
            // win.add(&header);
            win.connect_key_press_event(|wi, ek| unsafe {
                use gdk::keys::constants::*;
                let excl: BTreeSet<gdk::keys::Key> = BTreeSet::from_iter(
                    vec![
                        Return, Left, Right, Escape, Alt_L, Alt_R, Shift_L, Shift_R, Caps_Lock,
                        Tab, Up, Down, Super_L, Super_R, Home, End, Page_Down, Page_Up, Control_L,
                        Control_R,
                    ]
                    .into_iter(),
                );
                println!("{:?}", ek.keyval());
                if !excl.contains(&ek.keyval())
                    && !ENTRY.as_ref().unwrap().is_focus()
                    && !ek
                        .state()
                        .intersects(ModifierType::CONTROL_MASK | ModifierType::SUPER_MASK)
                // No modifier key present
                {
                    ENTRY.as_ref().unwrap().set_is_focus(true);
                }
                Inhibit(false)
            });
            win.set_titlebar(Some(&header));

            win.set_border_width(0);
            // window.set_decorations(true).unwrap();
            // window.set_always_on_top(true).unwrap();
            // window.set_focus().unwrap();
            window
                .move_window(Position::BottomRight)
                .expect("cannot move window");

            if conf.hide_on_blur {
                window.on_window_event(move |e| match e {
                    WindowEvent::Focused(b) => {
                        if !b {
                            save_pos(&w_on_ev);
                            w_on_ev.hide().unwrap();
                        }
                    }
                    _ => (),
                });
            }

            match app
                .global_shortcut_manager()
                .register("ctrl+alt+c", move || {
                    if w_on_shortcut.is_visible().unwrap() {
                        save_pos(&w_on_shortcut);
                        w_on_shortcut.hide().unwrap()
                    } else {
                        w_on_shortcut.set_always_on_top(true).unwrap();
                        restore_pos(&w_on_shortcut);
                        w_on_shortcut.show().unwrap();
                    }
                    // w_on_shortcut.set_focus().unwrap();
                    // w_on_shortcut.set_always_on_top(true).unwrap();
                }) {
                Ok(x) => {}
                Err(x) => {
                    println!("cannot reg global shortcut {:?}", x)
                }
            }

            let state: tauri::State<OffdictState> = app.state();
            let v = state.0.clone();
            unsafe {
                state_ = Some(state.0.clone());
            }

            let mut db = v.write().unwrap();

            // *db = d;

            tauri::async_runtime::spawn(serve(v.clone()));

            let mut m = Master::new(Handler {
                app: app.get_window("main").unwrap(),
                clip: app.clipboard_manager(),
                en_stemmer: Stemmer::create(Algorithm::English),
                last: "".to_owned(),
            });

            thread::spawn(move || {
                println!("clipboard ..");
                m.run().unwrap()
            });
            Ok(())
        })
        .manage(OffdictState(Arc::new(RwLock::new(d))))
        .invoke_handler(tauri::generate_handler![input, import]);

    x.run(tauri::generate_context!())
        .expect("error while running tauri application");
}
