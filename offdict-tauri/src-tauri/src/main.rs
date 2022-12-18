#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]
use clipboard_master::{CallbackResult, ClipboardHandler, Master};
use config::{Config, File, FileFormat};
use offdictd::{self, def_bin::WrapperDef, *};
use rust_stemmers::{Algorithm, Stemmer};
use std::{borrow::Cow, env, fs, path::PathBuf, sync::Arc, sync::RwLock, thread};
use tauri::{
    self, api::dialog, ClipboardManager, GlobalShortcutManager, Manager, Window, WindowEvent,
};
use timed::timed;

use std::io;
use tauri_plugin_positioner::{Position, WindowExt};

struct Handler<T: ClipboardManager> {
    app: Window,
    clip: T,
    en_stemmer: Stemmer,
}

impl<T: ClipboardManager> ClipboardHandler for Handler<T> {
    fn on_clipboard_change(&mut self) -> CallbackResult {
        let k = self.clip.read_text().unwrap().unwrap();
        // Clean up raw clipboard content, do fuzzy search, skip if no result, and stem, repeat.
        // let r = self.en_stemmer.stem(cleanup_clipboard_input(&k));
        let r: Cow<str> = Cow::Borrowed(cleanup_clipboard_input(&k));

        println!("clip: {}", r.as_ref());
        self.app.emit("clip", r.as_ref()).unwrap();
        if !self.app.is_visible().unwrap() {
            self.app.show().unwrap();
        }
        self.app.set_always_on_top(true).unwrap();
        // doesnt really work on kde, only sets it glowy
        // self.app.set_focus().expect("cannot focus window");

        CallbackResult::Next
    }

    fn on_clipboard_error(&mut self, error: io::Error) -> CallbackResult {
        println!("clip, error: {}", error);
        CallbackResult::Next
    }
}

fn cleanup_clipboard_input(s: &str) -> &str {
    s
}

#[derive(Debug, Deserialize)]
struct OffdictConfig {
    data_path: String,
    hide_on_blur: bool,
}

pub struct InnerState {
    pub db: RwLock<Option<offdict>>,
}

pub struct OffdictState(pub Arc<InnerState>);

// Learn more about Tauri commands at https://tauri.app/v1/guides/features/command

#[timed]
#[tauri::command]
fn defs<'a>(
    state: tauri::State<'a, OffdictState>,
    query: &'a str,
    fuzzy: bool,
) -> Result<Vec<DefItem>, &'static str> {
    // let state_guard = state.0.read().unwrap();
    let db_ = state.0.db.read();
    let db = db_.as_ref().unwrap().as_ref().unwrap();

    let d = db.search(query, 5, fuzzy);

    Ok(offdictd::flatten(d))
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
                    let mut db_ = v.db.write().unwrap();
                    let db = db_.as_mut().unwrap();

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

static mut w: Option<Window> = None;

fn main() {
    let x = tauri::Builder::default()
        .setup(|app| {
            println!("{}", env::current_dir().unwrap().to_str().unwrap());

            if let Ok(config) = Config::builder()
                .set_default("data_path", ".")
                .unwrap()
                .set_default("hide_on_blur", false)
                .unwrap()
                .add_source(File::new("config", FileFormat::Json5))
                .build()
            {
                let conf: OffdictConfig = config.try_deserialize().unwrap();
                println!("{:?}", conf);
                let window = app.get_window("main").unwrap();
                let w_on_ev = app.get_window("main").unwrap();
                let w_on_shortcut = app.get_window("main").unwrap();

                unsafe {
                    w = Some(app.get_window("main").unwrap());
                }

                window.set_always_on_top(true).unwrap();
                // window.set_focus().unwrap();
                window
                    .move_window(Position::BottomRight)
                    .expect("cannot move window");

                let _gtk_w = window.gtk_window().unwrap();

                if conf.hide_on_blur {
                    window.on_window_event(move |e| match e {
                        WindowEvent::Focused(b) => {
                            if !b {
                                w_on_ev.hide().unwrap();
                                let pos = w_on_ev.outer_position().unwrap();
                                w_on_ev.set_position(pos).unwrap();
                            }
                        }
                        _ => (),
                    });
                }
                app.global_shortcut_manager()
                    .clone()
                    .register("ctrl+alt+c", move || {
                        w_on_shortcut.show().unwrap();
                        w_on_shortcut.set_focus().unwrap();
                        w_on_shortcut.set_always_on_top(true).unwrap();
                    })
                    .unwrap();

                let state: tauri::State<OffdictState> = app.state();

                let v = state.0.clone();
                let mut db = v.db.write().unwrap();

                let db_path = PathBuf::from(conf.data_path.clone());

                *db = Some(offdict::open_db(db_path.to_str().unwrap().to_owned()));
            } else {
                println!("Config not found in working directory");
                panic!();
            }

            let mut m = Master::new(Handler {
                app: app.get_window("main").unwrap(),
                clip: app.clipboard_manager(),
                en_stemmer: Stemmer::create(Algorithm::English),
            });

            thread::spawn(move || {
                println!("clipboard ..");
                m.run().unwrap()
            });
            Ok(())
        })
        .manage(OffdictState(Arc::new(InnerState {
            db: RwLock::new(None),
        })))
        .invoke_handler(tauri::generate_handler![defs, import]);
    x.run(tauri::generate_context!())
        .expect("error while running tauri application");
}
