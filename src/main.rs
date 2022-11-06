use ciborium::{de::from_reader, ser::into_writer};

use clap::{Parser, Subcommand};

use serde::{Deserialize, Serialize};
use serde_yaml::{self};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use std::{self};

// use bson::{self, Array, Serializer};

use fuzzy_trie::FuzzyTrie;
use rocksdb::{MergeOperands, Options, DB};
use std::cmp::min;

use std::collections::BTreeSet;
use std::error::Error;

use std::fs::File;
use std::io::{Read, Write};

use percent_encoding;
use tokio;
use warp::Filter;

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(about = "Offline dictionary", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Debug, Subcommand)]
enum Commands {
    #[command(
        about = "Import definitions from an yaml file to rocksdb and fuzzytrie",
        arg_required_else_help = true
    )]
    yaml {
        #[arg(short = 'p')]
        path: String,
    },
    #[command(about = "Stats")]
    stat {},
    #[command(about = "Rebuild fuzzytrie from rocksdb")]
    trie {},
    #[command(about = "Fuzzy query (prefix)")]
    lookup { query: String },
}

// Defs are merged into one single Def
fn def_merge(
    key: &[u8], // The word
    existing_val: Option<&[u8]>,
    operands: &MergeOperands, // List of new values
) -> Option<Vec<u8>> {
    let _result: Vec<u8> = Vec::new();
    let ops = Vec::from_iter(operands);
    let mut _def: Def;
    let mut set = BTreeSet::<Def>::new();
    let mut wrapper: Def = Def {
        word: Some(std::str::from_utf8(key).unwrap().to_owned()),
        definitions: None, // Definitions from different dicts
        // Can depend on sub-definition
        EN: None,
        CN: None,
        pronunciation: None,
        examples: None,
        // -
        index: None,
        title: None,
        r#type: None,
        t1: None,
        _wrapper: true,
    };
    // TODO: How to structure the def. Compare defs in existing value, and ops
    // Create a wrapper def for all imported def
    let x;
    match existing_val {
        Some(_bytes) => {
            let p = from_reader::<Def, &[u8]>(existing_val.unwrap());
            match p {
                Ok(mut d) => {
                    if !d._wrapper {
                        x = d;
                        set.insert(x);
                    } else {
                        d.definitions
                            .unwrap_or_default()
                            .into_iter()
                            .map(|k| set.insert(k));
                        d.definitions = None;
                        wrapper = d;
                    }
                }
                _ => (),
            }
        }
        None => (),
    }

    for bytes in ops {
        match from_reader::<Def, &[u8]>(bytes) {
            Ok(mut parsed) => {
                parsed.index = None; // remove it
                set.insert(parsed)
            }
            Err(_) => continue,
        };
    }

    wrapper.definitions = Some(set.into_iter().collect());

    let mut v: Vec<u8> = Vec::new();
    into_writer(&wrapper, &mut v);
    Some(v)
    // Some(flexbuffers::to_vec(&wrapper).unwrap())
}

// Updates word-def mappings, deduping dup defs
fn import_defs<'a>(defs: &'a Vec<Def>, db: &DB, trie: &mut FuzzyTrie<'a, String>) {
    // let mut trie: FuzzyTrie<&'a str> = FuzzyTrie::new(2, false);
    for def in defs {
        let mut v: Vec<u8> = Vec::new();
        into_writer(&def, &mut v);
        db.merge(def.word.as_ref().unwrap().as_str(), v);

        trie.insert(def.word.as_ref().unwrap())
            .insert_unique(def.word.clone().unwrap()); // It does work with one key to multi values
    }
}

fn rebuild_trie<'a>(db: &DB, trie: &mut FuzzyTrie<'a, String>) {
    let iter = db.iterator(rocksdb::IteratorMode::Start);
    for x in iter {
        if let Ok((key, _val)) = x {
            let stri = String::from_utf8(key.to_vec()).unwrap();
            trie.insert(stri.as_str()).insert_unique(stri.clone());
        }
    }
}

fn open_db(path: &str) -> DB {
    let mut opts = Options::default();

    opts.create_if_missing(true);
    opts.set_merge_operator_associative("defs", def_merge);

    DB::open(&opts, path).unwrap()
}

fn load_trie<'a>(path: &str, buf: &'a mut Vec<u8>) -> FuzzyTrie<'a, String> {
    let mut file = match File::open(path) {
        Ok(f) => f,
        Err(_e) => return FuzzyTrie::new(2, true),
    };
    file.read_to_end(buf);

    // Flexbuffers break the fuzzy trie. Dont use it
    // let trie: FuzzyTrie<'a, String> = flexbuffers::from_slice(buf).unwrap();
    // let trie: FuzzyTrie<'a, String> = serde_yaml::from_slice(buf).unwrap();
    let trie: FuzzyTrie<'a, String> = from_reader(buf.as_slice()).unwrap();

    trie
}

fn save_trie<'a>(path: &str, trie: &FuzzyTrie<'a, String>) -> Result<(), Box<dyn Error>> {
    // let doc = flexbuffers::to_vec(trie)?;

    // let y = serde_yaml::to_string(trie).unwrap();
    // let doc = y.as_bytes();

    let mut doc: Vec<u8> = Vec::new();
    into_writer(trie, &mut doc);

    let mut file = File::create(path).expect("Unable to open file");

    file.write_all(&doc);
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Cli::parse();
    let db_path = "rocks_t";
    let trie_path = "./trie";

    let db = open_db(&db_path);
    let trie_buf: &'static mut Vec<u8> = Box::leak(Box::new(Vec::new()));
    let mut trie = FuzzyTrie::new(2, true);

    let yaml_defs: &'static mut Vec<Def> = Box::leak(Box::new(Vec::new()));

    match args.command {
        Some(Commands::yaml { path }) => {
            // Read yaml, put word defs in rocks, build a trie words -> words

            let file = File::open(path).expect("Unable to open file");
            *yaml_defs = serde_yaml::from_reader(file)?;

            trie = load_trie(trie_path, trie_buf);
            import_defs(yaml_defs, &db, &mut trie);

            // match db.get(yaml_defs[0].word.as_ref().unwrap()) {
            //     Ok(Some(value)) => bson::from_slice::<Def>(value.as_slice()),
            //     Ok(None) => println!(""),
            //     Err(e) => println!("operational problem encountered: {}", e),
            // }
            // db.delete(yaml_defs[0].word.as_ref().unwrap().as_str())
            //     .unwrap();

            // let _ = DB::destroy(&Options::default(), path);

            // println!("{}", serde_yaml::to_string(&docs)?);

            save_trie(trie_path, &trie);
            println!("imported");
        }
        Some(Commands::stat {}) => {
            trie = load_trie(trie_path, trie_buf);

            if let Ok(Some(r)) = db.property_int_value("rocksdb.estimate-num-keys") {
                println!("Words: {}", r);
            }
            println!("Words in trie: {}", trie.into_values().len());
        }
        Some(Commands::trie {}) => {
            trie = FuzzyTrie::new(2, true);
            rebuild_trie(&db, &mut trie);
            save_trie(&trie_path, &trie);
            println!("trie rebuilt");
        }
        Some(Commands::lookup { query }) => {
            trie = load_trie(trie_path, trie_buf);

            let mut key: Vec<(u8, &String)> = Vec::new();

            trie.prefix_fuzzy_search(&query, &mut key); // Values of, keys that are close, are returned
            let mut arr: Vec<(u8, &String)> = key.into_iter().collect();
            arr.sort_by_key(|x| x.0);
            let arr2 = arr[..min(3, arr.len())].iter();

            println!("{:?}", arr2);

            for d in db.multi_get(arr2.map(|(_d, str)| str)) {
                if let Ok(Some(by)) = d {
                    // println!("{:?}", bson::from_slice::<Def>(by.as_slice()).unwrap())
                    println!(
                        "{}",
                        serde_yaml::to_string::<Def>(&from_reader::<Def, &[u8]>(&by)?)?
                    )
                }
            }
        }
        None => {
            trie = load_trie(trie_path, trie_buf);
        }
    };

    let db: &'static DB = Box::leak(Box::new(db));
    let trie: &'static FuzzyTrie<String> = Box::leak(Box::new(trie));

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let lookup = warp::get()
            .and(warp::path("q"))
            .and(warp::path::param::<String>())
            .map(|word: String| {
                let word = percent_encoding::percent_decode_str(&word)
                    .decode_utf8()
                    .unwrap()
                    .to_string();
                warp::reply::json(&api_q(db, trie, &word))
            });

        let stat = warp::get().and(warp::path("stat")).map(|| {
            warp::reply::json(&stat {
                words_rocks: db
                    .property_int_value("rocksdb.estimate-num-keys")
                    .unwrap()
                    .unwrap(),
                words_trie: trie.into_values().len(),
            })
        });

        tokio::join!(
            warp::serve(lookup.or(stat)).run(([127, 0, 0, 1], 3030)),
            repl(db, trie)
        );
    });

    Ok(())
}

#[derive(Serialize, Deserialize)]
struct stat {
    words_trie: usize,
    words_rocks: u64,
}

async fn repl(db: &DB, trie: &FuzzyTrie<'_, String>) {
    loop {
        let li = readline().await.unwrap();
        let li = li.trim();
        if li.is_empty() {
            continue;
        }

        match respond(li, &db, &trie) {
            Ok(quit) => {
                if quit {
                    break;
                }
            }
            Err(err) => {
                write!(std::io::stdout(), "{}", err)
                    .map_err(|e| e.to_string())
                    .unwrap();
                std::io::stdout()
                    .flush()
                    .map_err(|e| e.to_string())
                    .unwrap();
            }
        }
    }
}

fn respond(
    line: &str,
    db: &DB,
    // trie_bf: &Vec<u8>,
    trie: &FuzzyTrie<String>,
) -> Result<bool, String> {
    let arr = search(&db, &trie, line);

    for d in arr.into_iter().map(|mut x| x.cli_pretty()) {
        println!("{}", d);
    }

    Ok(false)
}

fn api_q(
    db: &DB,
    // trie_bf: &Vec<u8>,
    trie: &FuzzyTrie<String>,
    query: &str,
) -> Vec<Def> {
    println!("\nq: {}", query);
    let arr = search(&db, &trie, query);

    arr
}

fn search(db: &DB, trie: &FuzzyTrie<String>, query: &str) -> Vec<Def> {
    let mut key: Vec<(u8, &String)> = Vec::new();

    trie.prefix_fuzzy_search(query, &mut key); // Values of, keys that are close, are returned
    let mut arr: Vec<(u8, &String)> = key.into_iter().collect();
    arr.sort_by_key(|x| x.0);
    let arr2 = arr[..min(3, arr.len())].iter();

    let mut res: Vec<Def> = Vec::new();

    for d in db.multi_get(arr2.map(|(_d, str)| str)) {
        if let Ok(Some(by)) = d {
            // println!("{:?}", bson::from_slice::<Def>(by.as_slice()).unwrap())
            res.push(from_reader::<Def, &[u8]>(&by).unwrap());
        }
    }

    res
}

// fn api_lookup(res:Vec<Def>)

async fn readline() -> Result<String, Box<dyn Error>> {
    let mut out = tokio::io::stdout();
    out.write_all(b"@ ").await?;
    out.flush().await?;
    let mut buffer = Vec::new();
    tokio::io::stdin().read(&mut buffer).await?;
    let stdin = tokio::io::stdin();
    let reader = tokio::io::BufReader::new(stdin);
    let mut lines = tokio::io::AsyncBufReadExt::lines(reader);
    Ok(lines.next_line().await?.unwrap())
}

impl Def {
    fn cli_pretty(&mut self) -> String {
        self._wrapper = false;
        let r = serde_yaml::to_string(self).unwrap();
        // self._wrapper = true;
        r
    }
}

#[derive(Serialize, Deserialize, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Def {
    #[serde(skip_serializing_if = "Option::is_none")]
    definitions: Option<Vec<Def>>,
    // Hierarchical definitions. Definitions from different dictionaries, and in the same dictionary there is multiple definitions
    // Merging the definitions can be considered.
    #[serde(skip_serializing_if = "Option::is_none")]
    EN: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pronunciation: Option<pronunciation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    r#type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    index: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    word: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    CN: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    t1: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    examples: Option<Vec<example>>,
    #[serde(default = "default_as_false", skip_serializing_if = "is_false")]
    _wrapper: bool,
}

fn is_false(p: &bool) -> bool {
    !p.clone()
}

fn default_as_false() -> bool {
    false
}

#[derive(Serialize, Deserialize, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
#[serde(untagged)]
enum example {
    obj(example_obj),
    str(Option<String>),
}

#[derive(Serialize, Deserialize, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
#[serde(untagged)]
enum pronunciation {
    vec(Vec<Option<String>>),
    str(Option<String>),
}

#[derive(Serialize, Deserialize, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
struct example_obj {
    #[serde(skip_serializing_if = "Option::is_none")]
    CN: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    EN: Option<String>,
}
