#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]
use config::{Config, File, FileFormat};
use offdictd::{self, *};
use rust_stemmers::{Algorithm, Stemmer};
use std::{
    env, fs,
    path::PathBuf,
    sync::Arc,
    sync::{RwLock},
    thread,
};
use tauri::{
    self, api::dialog, ClipboardManager, GlobalShortcutManager, Manager, Window,
    WindowEvent,
};
use timed::timed;
use clipboard_master::{CallbackResult, ClipboardHandler, Master};

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
        let r = self.en_stemmer.stem(cleanup_clipboard_input(&k));

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

pub struct InnerState<'a> {
    pub db: RwLock<Option<DB>>,
    pub trie: RwLock<Option<FuzzyTrie<'a, String>>>,
    pub trie_buf: RwLock<Vec<u8>>,
    pub yaml_defs: RwLock<Vec<Def>>,
    // pub importing: bool,
}

pub struct OffdictState<'a>(pub Arc<InnerState<'a>>);

// Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
#[timed]
#[tauri::command]
fn candidates<'a>(
    state: tauri::State<'a, OffdictState>,
    query: &'a str,
) -> Result<Vec<String>, ()> {
    let state_guard = state.0.trie.read().unwrap();

    let strs = offdictd::get_candidates(state_guard.as_ref().unwrap(), query, 5);
    println!("{:?}", strs);

    Ok(strs.into_iter().map(|x| x.clone()).collect()) // lets clone it bc idk how to solve it atm
}

#[timed]
#[tauri::command]
fn defs<'a>(state: tauri::State<'a, OffdictState>, query: &'a str) -> Result<Def, &'static str> {
    // let state_guard = state.0.read().unwrap();
    let db_ = state.0.db.read();
    let db = db_.as_ref().unwrap().as_ref().unwrap();
    let trie_ = state.0.trie.read();
    let trie = trie_.as_ref().unwrap().as_ref().unwrap();
    let d = offdictd::search_single(db, trie, query);

    match d {
        Some(de) => Ok(de),
        None => Err("not found"),
    }
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
                    let db_ = v.db.read();
                    let db = db_.as_ref().unwrap().as_ref().unwrap();
                    let mut trie_ = v.trie.write();
                    let trie = trie_.as_mut().unwrap().as_mut().unwrap();
                    let mut trie_buf_ = v.trie_buf.write();
                    let trie_buf = <Vec<u8> as AsMut<Vec<u8>>>::as_mut(trie_buf_.as_mut().unwrap());
                    let mut yaml_defs_ = v.yaml_defs.write();
                    let yaml_defs: &mut Vec<Def> = yaml_defs_.as_mut().unwrap().as_mut();
                    println!("folder picked, {}", folder.display());
                    let paths = fs::read_dir(folder).unwrap();
                    for path in paths {
                        let p: String = path.as_ref().unwrap().path().to_str().unwrap().to_string();
                        if p.ends_with(".yaml") {
                            println!("importing yaml, {}", &p);
                            let pre = path.unwrap().file_name();
                            let name = pre.to_str().unwrap().split_once(".").unwrap();
                            let pa;
                            // let yaml_defs;
                            unsafe {
                                w.as_ref().unwrap().emit("importing", &p);
                                pa = &trie_pa;
                            }

                            import_yaml_opened(
                                db,
                                trie,
                                trie_buf,
                                yaml_defs,
                                &p,
                                pa,
                                &name.0.to_string(),
                            )
                            .unwrap();

                            unsafe {
                                w.as_ref().unwrap().emit("imported", &p);
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
static mut trie_pa: String = String::new();

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

                window.set_always_on_top(true);
                // window.set_focus().unwrap();
                window
                    .move_window(Position::BottomRight)
                    .expect("cannot move window");

                let _gtk_w = window.gtk_window().unwrap();

                if conf.hide_on_blur {
                    window.on_window_event(move |e| match e {
                        WindowEvent::Focused(b) => {
                            if !b {
                                w_on_ev.hide();
                                let pos = w_on_ev.outer_position().unwrap();
                                w_on_ev.set_position(pos);
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
                    });

                let state: tauri::State<OffdictState> = app.state();

                let v = state.0.clone();
                let mut db = v.db.write().unwrap();
                let mut trie = v.trie.write().unwrap();
                let mut trie_buf = v.trie_buf.write().unwrap();

                let mut db_path = PathBuf::from(conf.data_path.clone());
                db_path.push("rocks_t");
                let mut trie_path = PathBuf::from(conf.data_path.clone());
                trie_path.push("trie");
                unsafe {
                    trie_pa = trie_path.to_str().unwrap().to_string();
                }

                *db = Some(open_db(db_path.to_str().unwrap()));

                *trie = Some(load_trie(trie_path.to_str().unwrap(), &mut trie_buf));
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
            trie: RwLock::new(None),
            trie_buf: RwLock::new(Vec::new()),
            yaml_defs: RwLock::new(vec![]),
        })))
        .invoke_handler(tauri::generate_handler![candidates, defs, import]);
    x.run(tauri::generate_context!())
        .expect("error while running tauri application");
}
