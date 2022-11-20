pub use serde::{Deserialize, Serialize};
pub use serde_yaml::{self};
pub use tokio::io::{AsyncReadExt, AsyncWriteExt};

use std::{self, vec};

// use bson::{self, Array, Serializer};

pub use fuzzy_trie::FuzzyTrie;
pub use rocksdb::{MergeOperands, Options, DB};
pub use std::cmp::min;

pub use std::collections::BTreeSet;
pub use std::error::Error;

pub use std::fs::File;
pub use std::io::{Read, Write};

pub use ciborium::{de::from_reader, ser::into_writer};

use lazy_static;
use regex::Regex;

use timed::timed;

use serde_ignored;
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
    pub definitions: Option<Vec<Def>>,
    // Hierarchical definitions. Definitions from different dictionaries, and in the same dictionary there is multiple definitions
    // Merging the definitions can be considered.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub groups: Option<Vec<Def>>, // Alias for definitions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub etymology: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub EN: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pronunciation: Option<pronunciation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub info: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub word: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub CN: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub t1: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub t2: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub examples: Option<Vec<example>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tip: Option<Vec<tip>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub related: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dictName: Option<String>,
    #[serde(default = "default_as_false", skip_serializing_if = "is_false")]
    pub _wrapper: bool,
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

#[derive(Serialize, Deserialize, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
#[serde(untagged)]
pub enum tip {
    obj(tip_obj),
    str(Option<String>),
    vec_str(Vec<String>),
}

#[derive(Serialize, Deserialize, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct tip_obj {
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

#[timed]
// exact word
pub fn search_single(db: &DB, trie: &FuzzyTrie<String>, word: &str) -> Option<Def> {
    if let Ok(Some(by)) = db.get(word.as_bytes()) {
        Some(from_reader::<Def, &[u8]>(&by).unwrap())
    } else {
        None
    }
}

#[timed]
pub fn import_yaml_opened<'a>(
    db: &DB,
    trie: &mut FuzzyTrie<'a, String>,
    trie_buf: &mut Vec<u8>,
    yaml_defs: &mut Vec<Def>,
    path: &str,
    trie_path: &str,
    name: &String,
) -> Result<(), Box<dyn Error>> {
    let file = File::open(path).expect("Unable to open file");
    *yaml_defs = serde_yaml::from_reader(file)?;
    for def in yaml_defs.iter_mut() {
        (*def).dictName = Some(name.clone());
    }
    import_defs(yaml_defs, db, trie);

    save_trie(trie_path, trie).unwrap();

    Ok(())
}

pub fn import_yaml<'a>(
    db: &DB,
    trie: &mut FuzzyTrie<'a, String>,
    trie_path: &str,
    trie_buf: &'a mut Vec<u8>,
    path: &str,
    yaml_defs: &'a mut Vec<Def>,
    name: &String,
) -> Result<(), Box<dyn Error>> {
    let file = File::open(path).expect("Unable to open file");
    *yaml_defs = serde_yaml::from_reader(file)?;

    for def in yaml_defs.iter_mut() {
        (*def).dictName = Some(name.clone());
    }
    *trie = load_trie(trie_path, trie_buf);
    import_defs(yaml_defs, db, trie);

    // match db.get(yaml_defs[0].word.as_ref().unwrap()) {
    //     Ok(Some(value)) => bson::from_slice::<Def>(value.as_slice()),
    //     Ok(None) => println!(""),
    //     Err(e) => println!("operational problem encountered: {}", e),
    // }
    // db.delete(yaml_defs[0].word.as_ref().unwrap().as_str())
    //     .unwrap();

    // let _ = DB::destroy(&Options::default(), path);

    // println!("{}", serde_yaml::to_string(&docs)?);

    save_trie(trie_path, trie);
    Ok(())
}

pub fn check_yaml(path: &str, save: bool) {
    let file = File::open(path).expect("Unable to open file");
    let d = serde_yaml::Deserializer::from_reader(file);
    let mut unused = BTreeSet::new();

    let p: Vec<Def> = serde_ignored::deserialize(d, |path| {
        unused.insert(path.to_string());
    })
    .unwrap();

    if save {
        let consumed = File::create(path.replace(".yaml", ".x.yaml")).unwrap();

        let par = p.into_iter().map(normalize_def).collect::<Vec<Def>>();
        serde_yaml::to_writer(consumed, &par);
    }
    println!("{:?}", unused);
}

pub fn contain_non_english(text: &str) -> bool {
    lazy_static::lazy_static! {
        static ref RE: Regex = Regex::new(r"\p{Han}").unwrap();
    }
    RE.is_match(text)
}

#[cfg(test)]
mod tests {
    use super::contain_non_english;
    #[test]
    fn check_chinese() {
        assert!(contain_non_english("以"));
        assert!(contain_non_english("'' 存在"));
        assert!(contain_non_english("'' 存在 be"));
        assert!(!contain_non_english("be"));
        assert!(!contain_non_english("' be"));
        assert!(!contain_non_english("be ᵐ"));
    }
}

pub fn candidates<'a>(
    trie: &'a FuzzyTrie<'a, String>,
    query: &'a str,
    num_max: usize,
) -> Vec<&'a String> {
    let mut key: Vec<(u8, &String)> = Vec::new();

    trie.prefix_fuzzy_search(query, &mut key);

    let mut arr: Vec<(u8, &String)> = key.into_iter().collect();
    arr.sort_by_key(|x| x.0); // x.0 is distance, from 0 to 2

    let arr2 = arr[..min(num_max, arr.len())].iter();
    println!("{:?}", arr.get(0));

    // Chinese words get a small distance that is 1.
    // Return [] if a Chinese word is matched to English

    if arr.len() > 0 {
        if contain_non_english(query) != contain_non_english(arr[0].1) {
            vec![]
        } else if arr[0].0 < 2 {
            arr2.map(|(_d, str)| *str).collect()
        } else {
            vec![]
        }
    } else {
        vec![]
    }
}

#[timed]
// Updates word-def mappings, deduping dup defs
pub fn import_defs<'a>(defs: &Vec<Def>, db: &DB, trie: &mut FuzzyTrie<'a, String>) {
    // let mut trie: FuzzyTrie<&'a str> = FuzzyTrie::new(2, false);
    for def in defs {
        let mut v: Vec<u8> = Vec::new();
        into_writer(&def, &mut v);
        db.merge(def.word.as_ref().unwrap().as_str(), v);
    }
    println!("- trie");
    for def in defs {
        trie.insert(def.word.as_ref().unwrap())
            .insert_unique(def.word.clone().unwrap()); // It does work with one key to multi values
    }
    db.flush();
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

pub fn stat_db(db: &DB) -> u32 {
    let iter = db.iterator(rocksdb::IteratorMode::Start);
    let mut n: u32 = 0;
    for x in iter {
        if let Ok((key, _val)) = x {
            n += 1;
        }
    }
    n
}

pub fn open_db(path: &str) -> DB {
    let mut opts = Options::default();

    opts.create_if_missing(true);
    opts.set_merge_operator_associative("defs", def_merge);

    DB::open(&opts, path).unwrap()
}

pub fn load_trie<'a>(path: &str, buf: &mut Vec<u8>) -> FuzzyTrie<'a, String> {
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

// fn recursive_path_shorten(mut d: Def) -> Def {
//     if (d.definitions.is_some() && d.definitions.unwrap().len() == 1) {
//         d.definitions = Some(d.definitions.unwrap()[0]);
//     }
//     recursive_path_shorten(d)
// }

fn normalize_def(mut d: Def) -> Def {
    if (d.groups.is_some()) {
        d.definitions = d.groups;
        d.groups = None;
    }
    if d._wrapper {
        unreachable!("_wrapper=true");
    }
    // d = recursive_path_shorten(d);
    d
}


#[timed]
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
        groups: None,
        // Can depend on sub-definition
        EN: None,
        CN: None,
        pronunciation: None,
        examples: None,
        etymology: None,
        related: None,
        // -
        index: None, // deprecated
        dictName: None,
        info: None,
        title: None,
        r#type: None,
        t1: None,
        t2: None,
        tip: None,
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
                        d.definitions.unwrap_or_default().into_iter().for_each(|k| {
                            set.insert(k);
                        });
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
                if parsed._wrapper {
                    parsed
                        .definitions
                        .unwrap_or_default()
                        .into_iter()
                        .for_each(|k| {
                            set.insert(k);
                        });
                } else {
                    set.insert(parsed);
                }
            }
            Err(_) => continue,
        };
    }

    wrapper.definitions = Some(set.into_iter().map(normalize_def).collect());

    let mut v: Vec<u8> = Vec::new();
    into_writer(&wrapper, &mut v);
    Some(v)
    // Some(flexbuffers::to_vec(&wrapper).unwrap())
}

// Notes on dictionary format
// Generally a dictionary is a Vec<Def>
// A self-sufficient dictionary is a Vec<Def> that tries to cover a topic
// The Vec<Def> can be serialized to Yaml, but I don't like it.
// Let's call self-sufficient dictionaries namespaces.

// A dictionary can be an IPLD block containing its description, revision, and all the CIDs that represent a Vec<Def> (we dont care about how the data is chunked)
