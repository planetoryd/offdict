pub use serde::{Deserialize, Serialize};
pub use serde_yaml::{self};
pub use tokio::io::{AsyncReadExt, AsyncWriteExt};

use std::{self};

// use bson::{self, Array, Serializer};
pub use std::cmp::min;

pub use std::error::Error;

pub use std::fs::File;
pub use std::io::{Read, Write};

use crate::def_bin;

#[allow(non_camel_case_types)]
#[allow(non_snake_case)]
#[serde_with::skip_serializing_none]
#[derive(Serialize, Deserialize, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct Def {
    pub definitions: Option<Vec<Def>>,
    // Hierarchical definitions. Definitions from different dictionaries, and in the same dictionary there is multiple definitions
    // Merging the definitions can be considered.
    pub groups: Option<Vec<Def>>, // Alias for definitions
    pub etymology: Option<Vec<String>>,
    pub EN: Option<String>,
    pub pronunciation: Option<pronunciation>,
    pub title: Option<String>,
    pub info: Option<String>,
    pub r#type: Option<String>,
    pub index: Option<u32>,
    pub word: Option<String>,
    pub CN: Option<String>,
    pub t1: Option<String>,
    pub t2: Option<String>,
    pub examples: Option<Vec<example>>,
    pub tip: Option<Vec<tip>>,
    pub related: Option<Vec<String>>,
    pub dictName: Option<String>,
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
        }
    }
}

pub fn is_false(p: &bool) -> bool {
    !p.clone()
}

pub fn default_as_false() -> bool {
    false
}

#[allow(non_camel_case_types)]
#[derive(Serialize, Deserialize, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Clone)]
#[serde(untagged)]
pub enum example {
    obj(example_obj),
    str(String),
    none,
}

#[allow(non_camel_case_types)]
#[derive(Serialize, Deserialize, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Clone)]
#[serde(untagged)]
pub enum pronunciation {
    vec(Vec<Option<String>>),
    str(String),
    none,
}

#[allow(non_snake_case)]
#[allow(non_camel_case_types)]
#[derive(Serialize, Deserialize, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Clone)]
#[serde_with::skip_serializing_none]
pub struct example_obj {
    CN: Option<String>,
    EN: Option<String>,
}

#[allow(non_camel_case_types)]
#[derive(Serialize, Deserialize, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Clone)]
#[serde_with::skip_serializing_none]
#[serde(untagged)]
pub enum tip {
    obj(tip_obj),
    str(String),
    vec_str(Vec<String>),
    none,
}
#[allow(non_snake_case)]
#[allow(non_camel_case_types)]
#[derive(Serialize, Deserialize, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Clone)]
#[serde_with::skip_serializing_none]
pub struct tip_obj {
    CN: Option<String>,
    EN: Option<String>,
}

impl Def {
    pub fn for_machine(self) -> def_bin::Def {
        self.into()
    }
}
