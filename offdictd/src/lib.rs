
pub use serde::{Deserialize, Serialize};
pub use serde_yaml::{self};
pub use tokio::io::{AsyncReadExt, AsyncWriteExt};

use std::{self};

// use bson::{self, Array, Serializer};

pub use rocksdb::{MergeOperands, Options, DB};
pub use fuzzy_trie::FuzzyTrie;
pub use std::cmp::min;

pub use std::collections::BTreeSet;
pub use std::error::Error;

pub use std::fs::File;
pub use std::io::{Read, Write};

pub use ciborium::{de::from_reader, ser::into_writer};

impl Def {
    pub fn cli_pretty(&mut self) -> String {
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
pub enum example {
    obj(example_obj),
    str(Option<String>),
}

#[derive(Serialize, Deserialize, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
#[serde(untagged)]
pub enum pronunciation {
    vec(Vec<Option<String>>),
    str(Option<String>),
}

#[derive(Serialize, Deserialize, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct example_obj {
    #[serde(skip_serializing_if = "Option::is_none")]
    CN: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    EN: Option<String>,
}


pub fn search(db: &DB, trie: &FuzzyTrie<String>, query: &str) -> Vec<Def> {
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


// Updates word-def mappings, deduping dup defs
pub fn import_defs<'a>(defs: &'a Vec<Def>, db: &DB, trie: &mut FuzzyTrie<'a, String>) {
    // let mut trie: FuzzyTrie<&'a str> = FuzzyTrie::new(2, false);
    for def in defs {
        let mut v: Vec<u8> = Vec::new();
        into_writer(&def, &mut v);
        db.merge(def.word.as_ref().unwrap().as_str(), v);

        trie.insert(def.word.as_ref().unwrap())
            .insert_unique(def.word.clone().unwrap()); // It does work with one key to multi values
    }
}

pub fn rebuild_trie<'a>(db: &DB, trie: &mut FuzzyTrie<'a, String>) {
    let iter = db.iterator(rocksdb::IteratorMode::Start);
    for x in iter {
        if let Ok((key, _val)) = x {
            let stri = String::from_utf8(key.to_vec()).unwrap();
            trie.insert(stri.as_str()).insert_unique(stri.clone());
        }
    }
}

pub fn open_db(path: &str) -> DB {
    let mut opts = Options::default();

    opts.create_if_missing(true);
    opts.set_merge_operator_associative("defs", def_merge);

    DB::open(&opts, path).unwrap()
}

pub fn load_trie<'a>(path: &str, buf: &'a mut Vec<u8>) -> FuzzyTrie<'a, String> {
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

pub fn save_trie<'a>(path: &str, trie: &FuzzyTrie<'a, String>) -> Result<(), Box<dyn Error>> {
    // let doc = flexbuffers::to_vec(trie)?;

    // let y = serde_yaml::to_string(trie).unwrap();
    // let doc = y.as_bytes();

    let mut doc: Vec<u8> = Vec::new();
    into_writer(trie, &mut doc);

    let mut file = File::create(path).expect("Unable to open file");

    file.write_all(&doc);
    Ok(())
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

