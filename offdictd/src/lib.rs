#![allow(unused_variables)]
pub use serde::{Deserialize, Serialize};
pub use serde_yaml::{self};
pub use tokio::io::{AsyncReadExt, AsyncWriteExt};

use std::collections::BTreeSet;
use std::{self};

// use bson::{self, Array, Serializer};
pub use std::cmp::min;

pub use std::error::Error;

pub use std::fs::File;
pub use std::io::{Read, Write};

// use lazy_static;
// use regex::Regex;

use timed::timed;

use serde_ignored;

use fuzzy_rocks::{ DistanceFunction, Table, TableConfig};

pub fn search(db: &Table<DictTableConfig, true>, query: &str) -> Vec<Def> {
    let f = db.lookup_fuzzy(query, None).unwrap();

    f.map(|(r, d)| {
        let r = db.get(r);
        r.unwrap().1
    })
    .collect::<Vec<Def>>()
}

#[timed]
// exact word
pub fn search_single(db: &mut Table<DictTableConfig, true>, word: &str) -> Option<Def> {
    if let Ok(mut ids) = db.lookup_exact(word) {
        let id = ids.next().unwrap();
        let r = db.get_value(id).unwrap();
        Some(r)
    } else {
        None
    }
}

#[timed]
pub fn import_yaml<'a>(
    db: &mut Table<DictTableConfig, true>,
    yaml_defs: &mut Vec<Def>,
    path: &str,
    name: &String,
) -> Result<(), Box<dyn Error>> {
    let file = File::open(path).expect("Unable to open file");
    *yaml_defs = serde_yaml::from_reader(file)?;
    for def in yaml_defs.iter_mut() {
        (*def).dictName = Some(name.clone());
    }
    import_defs(yaml_defs, db);
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
        serde_yaml::to_writer(consumed, &par).unwrap();
    }
    println!("{:?}", unused);
}

#[timed]
// Updates word-def mappings, deduping dup defs
pub fn import_defs<'a>(defs: &Vec<Def>, db: &mut Table<DictTableConfig, true>) {
    // let mut trie: FuzzyTrie<&'a str> = FuzzyTrie::new(2, false);
    for def in defs {
        db.insert(def.word.as_ref().unwrap().as_str(), def).unwrap();
    }
}

pub fn stat_db(db: &Table<DictTableConfig, true>) -> u32 {
    0
}

pub struct DictTableConfig();

impl TableConfig for DictTableConfig {
    type KeyCharT = char;
    type DistanceT = u8;
    type ValueT = Def;
    const UTF8_KEYS: bool = true;
    const MAX_DELETES: usize = 2;
    const MEANINGFUL_KEY_LEN: usize = 12;
    const GROUP_VARIANT_OVERLAP_THRESHOLD: usize = 5;
    const DISTANCE_FUNCTION: DistanceFunction<Self::KeyCharT, Self::DistanceT> =
        Self::levenstein_distance;
}

pub fn open_db(path: &str) -> Table<DictTableConfig, true> {
    let table = Table::<DictTableConfig, true>::new(path, DictTableConfig()).unwrap();
    // table.reset().unwrap();

    table
}

fn normalize_def(mut d: Def) -> Def {
    if d.groups.is_some() {
        d.definitions = d.groups;
        d.groups = None;
    }
    if d._wrapper {
        unreachable!("_wrapper=true");
    }
    // d = recursive_path_shorten(d);
    d
}

// Defs are merged into one single Def
// fn def_merge(
//     key: &[u8], // The word
//     existing_val: Option<&[u8]>,
//     operands: &MergeOperands, // List of new values
// ) -> Option<Vec<u8>> {
//     let _result: Vec<u8> = Vec::new();
//     let ops = Vec::from_iter(operands);
//     let mut _def: Def;
//     let mut set = BTreeSet::<Def>::new();
//     let mut wrapper: Def = Def {
//         word: Some(std::str::from_utf8(key).unwrap().to_owned()),
//         definitions: None, // Definitions from different dicts
//         groups: None,
//         // Can depend on sub-definition
//         EN: None,
//         CN: None,
//         pronunciation: None,
//         examples: None,
//         etymology: None,
//         related: None,
//         // -
//         index: None, // deprecated
//         dictName: None,
//         info: None,
//         title: None,
//         r#type: None,
//         t1: None,
//         t2: None,
//         tip: None,
//         _wrapper: true,
//     };
//     // TODO: How to structure the def. Compare defs in existing value, and ops
//     // Create a wrapper def for all imported def
//     let x;
//     match existing_val {
//         Some(_bytes) => {
//             let p = from_reader::<Def, &[u8]>(existing_val.unwrap());
//             match p {
//                 Ok(mut d) => {
//                     if !d._wrapper {
//                         x = d;
//                         set.insert(x);
//                     } else {
//                         d.definitions.unwrap_or_default().into_iter().for_each(|k| {
//                             set.insert(k);
//                         });
//                         d.definitions = None;
//                         wrapper = d;
//                     }
//                 }
//                 _ => (),
//             }
//         }
//         None => (),
//     }

//     for bytes in ops {
//         match from_reader::<Def, &[u8]>(bytes) {
//             Ok(mut parsed) => {
//                 parsed.index = None; // remove it
//                 if parsed._wrapper {
//                     parsed
//                         .definitions
//                         .unwrap_or_default()
//                         .into_iter()
//                         .for_each(|k| {
//                             set.insert(k);
//                         });
//                 } else {
//                     set.insert(parsed);
//                 }
//             }
//             Err(_) => continue,
//         };
//     }

//     wrapper.definitions = Some(set.into_iter().map(normalize_def).collect());

//     let mut v: Vec<u8> = Vec::new();
//     into_writer(&wrapper, &mut v);
//     Some(v)
//     // Some(flexbuffers::to_vec(&wrapper).unwrap())
// }

// Notes on dictionary format
// Generally a dictionary is a Vec<Def>
// A self-sufficient dictionary is a Vec<Def> that tries to cover a topic
// The Vec<Def> can be serialized to Yaml, but I don't like it.
// Let's call self-sufficient dictionaries namespaces.

// A dictionary can be an IPLD block containing its description, revision, and all the CIDs that represent a Vec<Def> (we dont care about how the data is chunked)

impl Def {
    pub fn cli_pretty(&mut self) -> String {
        self._wrapper = false;
        let r = serde_yaml::to_string(self).unwrap();
        // self._wrapper = true;
        r
    }
}

// New format
#[derive(Serialize, Deserialize, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct DefNew {
    pub word: String,
}

#[allow(non_camel_case_types)]
#[allow(non_snake_case)]
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

impl Default for Def {
    fn default() -> Self {
        Def {
            word: None,
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
            _wrapper: false,
        }
    }
}

fn is_false(p: &bool) -> bool {
    !p.clone()
}

fn default_as_false() -> bool {
    false
}

#[allow(non_camel_case_types)]
#[derive(Serialize, Deserialize, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
#[serde(untagged)]
pub enum example {
    obj(example_obj),
    str(String),
    none,
}

#[allow(non_camel_case_types)]
#[derive(Serialize, Deserialize, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
#[serde(untagged)]
pub enum pronunciation {
    vec(Vec<Option<String>>),
    str(String),
    none,
}

#[allow(non_snake_case)]
#[allow(non_camel_case_types)]
#[derive(Serialize, Deserialize, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct example_obj {
    #[serde(skip_serializing_if = "Option::is_none")]
    CN: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    EN: Option<String>,
}

#[allow(non_camel_case_types)]
#[derive(Serialize, Deserialize, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
#[serde(untagged)]
pub enum tip {
    obj(tip_obj),
    str(String),
    vec_str(Vec<String>),
    none,
}
#[allow(non_snake_case)]
#[allow(non_camel_case_types)]
#[derive(Serialize, Deserialize, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct tip_obj {
    #[serde(skip_serializing_if = "Option::is_none")]
    CN: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    EN: Option<String>,
}
#[cfg(test)]
mod tests {
    use std::{fmt::Debug};

    use crate::DefNew;

    use super::Def;
    use bincode::{Options};
    use serde::{Deserialize, Serialize};
    pub use serde_yaml::{self};
    fn test_bincode<T: for<'a> Deserialize<'a> + Serialize + Debug + PartialEq>(value: T) {
        let record_coder = bincode::DefaultOptions::new()
            .with_varint_encoding()
            .with_little_endian();

        let value_bytes = record_coder.serialize(&value).unwrap();

        let value_d: T = record_coder.deserialize(&value_bytes).unwrap();
        assert_eq!(value, value_d);
    }

    #[test]
    #[should_panic] // The original struct is too weird ...
    fn bincode_orig_def() {
        let value: Def = Def::default();

        test_bincode(value);
    }

    #[test]
    fn bincode_def() {
        let value: DefNew = DefNew {
            word: "nice".to_owned(),
        };

        test_bincode(value);
    }

    #[test]
    fn yaml() {
        let value: Def = Def::default();

        let mut value_bytes = Vec::new();
        serde_yaml::to_writer(&mut value_bytes, &value).unwrap();

        let value_d: Def = serde_yaml::from_reader(value_bytes.as_slice()).unwrap();
        assert_eq!(value, value_d);
    }
}
