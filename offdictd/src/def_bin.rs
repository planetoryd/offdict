pub use serde::{Deserialize, Serialize};
pub use serde_yaml::{self};
pub use tokio::io::{AsyncReadExt, AsyncWriteExt};

use std::collections::{BTreeMap};
use std::iter::FromIterator;
use std::mem::transmute;
use std::{self};

// use bson::{self, Array, Serializer};
pub use std::cmp::min;

pub use std::error::Error;

pub use std::fs::File;
pub use std::io::{Read, Write};



#[allow(non_camel_case_types)]
#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Clone, Default)]
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

#[allow(non_camel_case_types)]
#[derive(Serialize, Deserialize, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Default, Clone)]
pub enum example {
    obj(example_obj),
    str(String),
    #[default]
    none,
}

#[allow(non_camel_case_types)]
#[derive(Serialize, Deserialize, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Default, Clone)]
pub enum pronunciation {
    vec(Vec<Option<String>>),
    str(String),
    #[default]
    none,
}

#[allow(non_snake_case)]
#[allow(non_camel_case_types)]
#[derive(Serialize, Deserialize, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Default, Clone)]
pub struct example_obj {
    CN: Option<String>,
    EN: Option<String>,
}

#[allow(non_camel_case_types)]
#[derive(Serialize, Deserialize, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Default, Clone)]
pub enum tip {
    obj(tip_obj),
    str(String),
    vec_str(Vec<String>),
    #[default]
    none,
}
#[allow(non_snake_case)]
#[allow(non_camel_case_types)]
#[derive(Serialize, Deserialize, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Default, Clone)]
pub struct tip_obj {
    CN: Option<String>,
    EN: Option<String>,
}

impl From<super::Def> for Def {
    fn from(value: super::Def) -> Self {
        unsafe { transmute(value) }
    }
}

impl From<Def> for super::Def {
    fn from(value: Def) -> Self {
        unsafe { transmute(value) }
    }
}

#[derive(Serialize, Deserialize, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Default, Clone)]
pub struct WrapperDef {
    pub items: BTreeMap<String, Def>,
    pub word: String,
}

impl From<Def> for WrapperDef {
    fn from(value: Def) -> Self {
        Self {
            word: value.word.clone().unwrap(),
            items: BTreeMap::from_iter([(value.dictName.clone().unwrap(), value)]),
        }
    }

}

impl WrapperDef {
    pub fn merge(mut self, other: &mut Self) -> Self {
        self.items.append(&mut other.items);
        self
    }
    pub fn vec_human(self) -> Vec<super::Def> {
        self.items.into_values().into_iter().map(|x| x.into()).collect()
    }
}
