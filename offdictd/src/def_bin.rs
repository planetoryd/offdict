use lazy_static::__Deref;
pub use serde::{Deserialize, Serialize};
pub use serde_yaml::{self};
pub use tokio::io::{AsyncReadExt, AsyncWriteExt};

use std::collections::BTreeMap;
use std::convert::TryInto;
use std::iter::FromIterator;
use std::mem::transmute;
use std::{self};

// use bson::{self, Array, Serializer};
pub use std::cmp::min;

pub use std::error::Error;

pub use std::fs::File;
pub use std::io::{Read, Write};

use crate::def;

pub mod DBKey {
    pub fn slice(b: &[u8]) -> (&[u8], &[u8]) {
        let c = &b[0..4];
        let len: u32 = u32::from_be_bytes(c.try_into().unwrap());
        (&b[4..(4 + len as usize)], &b[(4 + len as usize)..])
    }
    pub fn from(word: &str, dict: &str) -> Vec<u8> {
        let mut v: Vec<u8> = vec![];
        v.extend((word.len() as u32).to_be_bytes() as [u8; 4]);
        v.extend(word.as_bytes());
        v.extend(dict.as_bytes());
        debug_assert!(v.len() > 4);
        v
    }
}

impl Def {
    pub fn key(&self) -> Vec<u8> {
        DBKey::from(self.word.as_ref().unwrap(), self.dictName.as_ref().unwrap())
    }
}


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

#[derive(Serialize, Deserialize, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Clone, Default)]
pub enum shorthand<O> {
    obj(O),
    vec(Vec<Option<String>>),
    str(String),
    #[default]
    none,
}


pub type example = shorthand<example_obj>;


pub type pronunciation = shorthand<String>;


#[derive(Serialize, Deserialize, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct example_obj {
    CN: Option<String>,
    EN: Option<String>,
}


pub type tip = shorthand<tip_obj>;


#[derive(Serialize, Deserialize, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Clone)]
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
        let mut d: Self = unsafe { transmute(value) };
        d.normalize_def()
    }
}

#[derive(Serialize, Deserialize, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Default, Clone)]
pub struct WrapperDef {
    pub items: BTreeMap<String, Def>, // dictname to def
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
        self.items
            .into_values()
            .into_iter()
            .map(|x| x.into())
            .collect()
    }
}

#[test]
fn test_wrapper() {
    let d: def::Def =serde_yaml::from_str("
    definitions:
    - {}
    - EN: If there is a certain amount of something left, or if you have a certain amount of it left, it remains when the rest has gone or been used.
      info: 【搭配模式】：v-link ADJ 【搭配模式】：usu v-link PHR
      type: \"ADJ\t形容词\"
      CN: 剩余的；剩下的；余下的
      examples:
      - CN: 还有杜松子酒吗？
        EN: Is there any gin left?...
      - CN: 他还剩有大把的钱。
        EN: He's got plenty of money left...
      - CN: 他们还剩6场比赛要打。
        EN: They still have six games left to play.
      - CN: ', or if you have it'
        EN: left over
    index: 17775
    word: left
    dictName: wikibedia
    ").unwrap();
    let d2: Def = serde_yaml::from_str("
    definitions:
    - {}
    - EN: If there is a certain amount of something left, or if you have a certain amount of it left, it remains when the rest has gone or been used.
      info: 【搭配模式】：v-link ADJ 【搭配模式】：usu v-link PHR
      type: \"ADJ\t形容词\"
      CN: 剩余的；剩下的；余下的
    word: left
    dictName: wikipedia
    ").unwrap();
    let dbin: Def = d.into();
    let d2bin: Def = d2.into();
    let mut dw: WrapperDef = dbin.into();
    let dw2: WrapperDef = d2bin.into();
    let m = dw2.merge(&mut dw);
    dbg!(&m);
    dbg!(m.vec_human());
}
