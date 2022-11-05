use clap::Command;
use clap::{Args, Parser, Subcommand, ValueEnum};
use serde::de::IntoDeserializer;
use serde::{Deserialize, Serialize};
use serde_yaml::{self, value};

use std;

use std::cmp::min;
use std::collections::BTreeMap;
use std::collections::{BTreeSet, HashMap};
use std::error::Error;
use std::ffi::OsStr;
use std::ffi::OsString;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

use bson::{self, Array, Serializer};
use fuzzy_trie::FuzzyTrie;
use rocksdb::{DBCommon, MergeOperands, Options, SingleThreaded, ThreadMode, DB};

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(about = "Offline dictionary", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    yaml {
        path: String,
        #[arg(required = false)]
        query: Option<String>,
    },
    stat{},
}

// Defs are merged into one single Def
fn def_merge(
    key: &[u8], // The word
    existing_val: Option<&[u8]>,
    operands: &MergeOperands, // List of new values
) -> Option<Vec<u8>> {
    let mut result: Vec<u8> = Vec::new();
    let mut ops = Vec::from_iter(operands);
    let mut def: Def;
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
        Some(bytes) => {
            let p = bson::from_slice::<Def>(existing_val.unwrap());
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
        match bson::from_slice::<Def>(bytes) {
            Ok(mut parsed) => {
                parsed.index = None; // remove it
                set.insert(parsed)
            }
            Err(_) => continue,
        };
    }

    wrapper.definitions = Some(set.into_iter().collect());

    Some(bson::to_raw_document_buf(&wrapper).unwrap().into_bytes())
}

// Updates word-def mappings, deduping dup defs
fn import_defs<'a>(defs: &'a Vec<Def>, db: &DB) -> FuzzyTrie<&'a str> {
    let mut trie = FuzzyTrie::new(2, false);
    for def in defs {
        db.merge(
            def.word.as_ref().unwrap().as_str(),
            bson::to_raw_document_buf(&def).unwrap().as_bytes(),
        );
        trie.insert(def.word.as_ref().unwrap().as_str())
            .insert(def.word.as_ref().unwrap().as_str()); // It does work with one key to multi values
    }

    trie
}

fn open_db() -> DB {
    let mut opts = Options::default();
    let path = "rocks_t";

    opts.create_if_missing(true);
    opts.set_merge_operator_associative("defs", def_merge);

    DB::open(&opts, path).unwrap()
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Cli::parse();
    match args.command {
        Commands::yaml { path, query } => {
            // Read yaml, put word defs in rocks, build a trie words -> words

            let mut file = File::open(path).expect("Unable to open file");
            let yaml_defs: Vec<Def> = serde_yaml::from_reader(file)?;

            let db = open_db();

            let trie = import_defs(&yaml_defs, &db);

            // match db.get(yaml_defs[0].word.as_ref().unwrap()) {
            //     Ok(Some(value)) => bson::from_slice::<Def>(value.as_slice()),
            //     Ok(None) => println!(""),
            //     Err(e) => println!("operational problem encountered: {}", e),
            // }
            // db.delete(yaml_defs[0].word.as_ref().unwrap().as_str())
            //     .unwrap();

            // let _ = DB::destroy(&Options::default(), path);

            // println!("{}", serde_yaml::to_string(&docs)?);

            let mut key: Vec<(u8, &str)> = Vec::new();
            if let Some(ref q) = query {
                trie.prefix_fuzzy_search(q, &mut key); // Values of, keys that are close, are returned
                let mut key_iter = key.into_iter();
                println!("Distance {:?}", key_iter);
                let mut arr: Vec<(u8, &str)> = key_iter.collect();
                arr.sort_by_key(|x| x.0);
                for d in db.multi_get(arr[..min(5, arr.len())].iter().map(|(d, str)| str)) {
                    if let Ok(Some(by)) = d {
                        // println!("{:?}", bson::from_slice::<Def>(by.as_slice()).unwrap())
                        println!(
                            "{}",
                            serde_yaml::to_string(
                                &bson::from_slice::<Def>(by.as_slice()).unwrap()
                            )?
                        )
                    }
                }
            }

            Ok(())
        }
        Commands::stat{} => {
            let db = open_db();

            if let Ok(Some(r)) = db.property_int_value("rocksdb.estimate-num-keys") {
                println!("Words: {}", r);
            }
            Ok(())
        }
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
    #[serde(default = "default_as_false")]
    _wrapper: bool,
}

fn default_as_false() -> bool {
    false
}

#[derive(Serialize, Deserialize, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
#[serde(untagged)]
enum example {
    obj(example_obj),
    str(Option<String>)
}

#[derive(Serialize, Deserialize, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
#[serde(untagged)]
enum pronunciation {
    vec(Vec<Option<String>>),
    str(Option<String>)
}

#[derive(Serialize, Deserialize, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
struct example_obj {
    #[serde(skip_serializing_if = "Option::is_none")]
    CN: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    EN: Option<String>,
}

