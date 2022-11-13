#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]
use config::{Config, File, FileFormat};
use offdictd::{self, *};
use std::{borrow::Borrow, env, path::PathBuf, sync::Arc, sync::RwLock};
use tauri::Manager;

#[derive(Debug, Deserialize)]
struct OffdictConfig {
    data_path: String,
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
    tauri::Builder::default()
        .setup(|app| {
            println!("{}", env::current_dir().unwrap().to_str().unwrap());
            let config = Config::builder()
                .set_default("data_path", ".")
                .unwrap()
                .add_source(File::new("config", FileFormat::Json5))
                .build()
                .unwrap();

            let conf: OffdictConfig = config.try_deserialize().unwrap();
            println!("{:?}", conf);
            let window = app.get_window("main").unwrap();
            // window.emit("error", 1);
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
            Ok(())
        })
        .manage(OffdictState(RwLock::new(InnerState {
            db: None,
            trie: None,
            trie_buf: Vec::new(),
        })))
        .invoke_handler(tauri::generate_handler![candidates, defs])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
