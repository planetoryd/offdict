#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]
use config::{Config, File, FileFormat};
use offdictd::{self, *};
use rust_stemmers::{Algorithm, Stemmer};
use std::{
    borrow::Borrow,
    env,
    path::PathBuf,
    sync::Arc,
    sync::{Mutex, RwLock},
    thread,
};
use tauri::{
    self, window, App, ClipboardManager, GlobalShortcutManager, Manager, Window, WindowEvent,
};

use clipboard_master::{CallbackResult, ClipboardHandler, Master};

use tauri_plugin_positioner::{WindowExt, Position};
use std::io;

// struct Handler<'a> {
//     app: &'a mut App,
// }

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
    pub db: Option<DB>,
    pub trie: Option<FuzzyTrie<'a, String>>,
    pub trie_buf: Vec<u8>,
}

pub struct OffdictState<'a>(pub RwLock<InnerState<'a>>);

// Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
#[tauri::command]
fn candidates<'a>(
    state: tauri::State<'a, OffdictState>,
    query: &'a str,
) -> Result<Vec<String>, ()> {
    // offdictd::candidates(trie, query, num_max)
    let state_guard = state.0.read().unwrap();
    // Change field of state struct
    // Replace state struct; here you need to dereference the guard to get the pointer to the inner value (I think)
    // *state_guard = InnerState {
    //     db: None,
    //     trie: None,
    // };
    let strs = offdictd::candidates(state_guard.trie.as_ref().unwrap(), query, 5);
    println!("{:?}", strs);

    Ok(strs.into_iter().map(|x| x.clone()).collect()) // lets clone it bc idk how to solve it atm
}

#[tauri::command]
fn defs<'a>(state: tauri::State<'a, OffdictState>, query: &'a str) -> Result<Def, &'static str> {
    let state_guard = state.0.read().unwrap();

    let d = offdictd::search_single(
        state_guard.db.as_ref().unwrap(),
        state_guard.trie.as_ref().unwrap(),
        query,
    );

    match d {
        Some(de) => Ok(de),
        None => Err("not found"),
    }
}

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
                let mut window = app.get_window("main").unwrap();
                let w_on_ev = app.get_window("main").unwrap();
                let w_on_shortcut = app.get_window("main").unwrap();
                window.set_always_on_top(true);
                // window.set_focus().unwrap();
                window.move_window(Position::BottomRight).expect("cannot move window");

                let gtk_w = window.gtk_window().unwrap();

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
                let mut state_guard = state.0.write().unwrap();

                let mut db_path = PathBuf::from(conf.data_path.clone());
                db_path.push("rocks_t");
                let mut trie_path = PathBuf::from(conf.data_path.clone());
                trie_path.push("trie");

                state_guard.db = Some(open_db(db_path.to_str().unwrap()));

                state_guard.trie = Some(load_trie(
                    trie_path.to_str().unwrap(),
                    &mut state_guard.trie_buf,
                ));
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
        .manage(OffdictState(RwLock::new(InnerState {
            db: None,
            trie: None,
            trie_buf: Vec::new(),
        })))
        .invoke_handler(tauri::generate_handler![candidates, defs]);
    x.run(tauri::generate_context!())
        .expect("error while running tauri application");
}
