use config::Value;
use regex::bytes::Regex;
pub use serde::{Deserialize, Serialize};
use serde_yaml::value::TaggedValue;
use serde_yaml::Mapping;
pub use serde_yaml::{self};
pub use tokio::io::{AsyncReadExt, AsyncWriteExt};

use std::{self, default};

// use bson::{self, Array, Serializer};
pub use std::cmp::min;

pub use std::error::Error;

pub use std::fs::File;
pub use std::io::{Read, Write};

use crate::def_bin;

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

#[derive(Serialize, Deserialize, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Clone, Default)]
#[serde_with::skip_serializing_none]
#[serde(untagged)]
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
#[serde_with::skip_serializing_none]
pub struct example_obj {
    CN: Option<String>,
    EN: Option<String>,
}

pub type tip = shorthand<tip_obj>;

#[derive(Serialize, Deserialize, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Clone)]
#[serde_with::skip_serializing_none]
pub struct tip_obj {
    CN: Option<String>,
    EN: Option<String>,
}

pub trait Emptyable {
    fn empty_(&self) -> bool;
}

impl<T: Emptyable> Emptyable for Vec<T> {
    fn empty_(&self) -> bool {
        self.is_empty() || self.iter().fold(true, |a, e| a && e.empty_())
    }
}

impl DefaultToBeEmpty for String {}
impl Emptyable for tip_obj {
    fn empty_(&self) -> bool {
        self.CN.empty_() && self.EN.empty_()
    }
}
impl Emptyable for example_obj {
    fn empty_(&self) -> bool {
        self.CN.empty_() && self.EN.empty_()
    }
}

impl<T: Emptyable> Emptyable for shorthand<T> {
    fn empty_(&self) -> bool {
        match self {
            Self::none => true,
            Self::obj(o) => o.empty_(),
            Self::str(s) => s.is_empty(),
            Self::vec(v) => v.is_empty() || v[0].empty_(),
        }
    }
}

impl<T: Emptyable> Emptyable for Option<T> {
    fn empty_(&self) -> bool {
        self.is_none() || self.as_ref().unwrap().empty_()
    }
}

trait DefaultToBeEmpty: Default + PartialEq {}

impl<T: DefaultToBeEmpty> Emptyable for T {
    fn empty_(&self) -> bool {
        &Self::default() == self
    }
}

#[test]
fn test_empty() {
    assert!(Option::<String>::None.empty_());
    assert!(!Some("aa".to_owned()).empty_());
    assert!(Some("".to_owned()).empty_());
    assert!(Some(Vec::<String>::new()).empty_());
    assert!(pronunciation::none.empty_());
    assert!(!pronunciation::str("aa".to_owned()).empty_());
    assert!(tip::none.empty_());
    assert!(!tip::str("aa".to_owned()).empty_());
    assert!(!tip::obj(tip_obj {
        CN: Some("aa".to_owned()),
        EN: None
    })
    .empty_());
    assert!(tip::obj(tip_obj {
        CN: Some("".to_owned()),
        EN: Some("".to_owned())
    })
    .empty_());

    let opstr = Some("".to_owned());

    let e1 = Def::default();
    assert!(e1.empty_());

    let e2 = Def {
        info: opstr.clone(),
        index: Some(89),
        dictName: opstr.clone(),
        ..Default::default()
    };
    assert!(e2.empty_());

    let e3 = Def {
        info: Some("aaa".to_owned()),
        ..Default::default()
    };
    assert!(!e3.empty_());

    let mut e4: Def = serde_yaml::from_str("
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
    ").unwrap();
    assert!(!e4.empty_());
    e4.definitions.as_mut().unwrap().remove(1);

    assert!(e4.empty_())
}

impl Emptyable for Def {
    fn empty_(&self) -> bool {
        let defs = if self.definitions.is_none() {
            &self.groups
        } else {
            &self.definitions
        };

        ({
            self.CN.is_none()
                && self.EN.empty_()
                && self.etymology.empty_()
                && self.examples.empty_()
                && self.info.empty_()
                && self.pronunciation.empty_()
                && self.related.empty_()
                && self.tip.empty_()
        }) && defs.empty_()
    }
}

impl Def {
    pub fn for_machine(self) -> def_bin::Def {
        self.into()
    }
    // Remove "" strings
    // Remove &nbsp stuff
    pub fn cleanup(mut self) -> Self {
        self.index = None;
        let mut obj = serde_yaml::to_value(self).unwrap();
        cleanup_value(&mut obj);

        serde_yaml::from_value(obj).unwrap()
    }
}

use lazy_regex::regex;

// #[test]
// fn cleanup() {
//     let piece = include_str!("../fixtures/prob.yaml");
//     let mut d: Vec<Def> = serde_yaml::from_str(piece).unwrap();
//     let n: Vec<Def> = d.into_iter().map(|x| x.cleanup()).collect();

//     let f = File::create("./fixtures/prob.x.yaml").unwrap();
//     serde_yaml::to_writer(f, &n).unwrap();
// }

fn cleanup_value(v: &mut serde_yaml::Value) {
    match v {
        serde_yaml::Value::String(s) => {
            if s.empty_() {
                *v = serde_yaml::Value::Null
            } else {
                let r = regex!("&nbsp;?");
                let re = r.replace_all(s.as_str(), "");
                *s = re.to_string();
                *s = s.trim().to_owned();
                if s.empty_() {
                    *v = serde_yaml::Value::Null
                }
            }
        }
        serde_yaml::Value::Mapping(m) => {
            for (k, v) in m.iter_mut() {
                cleanup_value(v)
            }
        }
        serde_yaml::Value::Sequence(s) => {
            if s.is_empty() {
                *v = serde_yaml::Value::Null;
            } else {
                s.iter_mut().for_each(|e| cleanup_value(e));
            }
        }
        serde_yaml::Value::Tagged(t) => cleanup_value(&mut t.value),
        _ => (),
    }
}
