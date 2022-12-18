#![allow(unused_variables)]

pub use serde::{Deserialize, Serialize};
pub use serde_yaml::{self};
pub use tokio::io::{AsyncReadExt, AsyncWriteExt};


use std::collections::{BTreeSet, HashMap};

use std::iter::FromIterator;

use std::{self, vec};

// use bson::{self, Array, Serializer};
pub use std::cmp::min;

pub use std::error::Error;

pub use std::fs::{create_dir_all, File};
pub use std::io::{Read, Write};
use std::path::Path;

// use lazy_static;
// use regex::Regex;

use timed::timed;

#[macro_use]
extern crate tantivy;

use std::cmp::Ordering::Equal;
use tantivy::collector::{Count, TopDocs};
use tantivy::query::{FuzzyTermQuery, QueryParser, RegexQuery, TermQuery};
use tantivy::schema::{*};
use tantivy::{DocAddress, Index, IndexWriter};
use tantivy::{IndexReader};

use debug_print::debug_println;

use serde_ignored;

pub type DefItemInDB = def_bin::WrapperDef;
pub type DefItem = def_bin::Def;
pub type DB = IndexReader;

pub struct stat {
    pub words: u64,
}

pub type candidate = (f32, DocAddress);
pub type candidates = Vec<candidate>;
pub struct offdict {
    // tantivy stuff
    reader: IndexReader,
    writer: IndexWriter,
    index: Index,
    query_parser: QueryParser,
    schema: Schema,
    path_tantivy: String,
}

impl offdict {
    pub fn serialize<T: Serialize>(v: &T) -> Result<Vec<u8>, Box<bincode::ErrorKind>> {
        bincode::serialize(v)
    }

    pub fn deserialize<'a, T: Deserialize<'a>>(v: &'a [u8]) -> Result<T, Box<bincode::ErrorKind>> {
        bincode::deserialize(v)
    }

    pub fn open_db(path: String) -> Self {
        let mut schema_builder = Schema::builder();
        // associate one word with one wrapperDef
        schema_builder.add_text_field("word", STRING);
        schema_builder.add_bytes_field("data", FAST | STORED);
        schema_builder.add_u64_field("version", FAST | STORED);

        let schema = schema_builder.build();
        let index;
        let pp = Path::new(&path);
        let word = schema.get_field("word").unwrap();
        if pp.is_dir() {
            index = Index::open_in_dir(&path).unwrap();
        } else {
            create_dir_all(pp).unwrap();
            index = Index::create_in_dir(&path, schema.clone()).unwrap();
        }

        offdict {
            path_tantivy: path,
            reader: index.reader().unwrap(),
            writer: index.writer(3_000_000).unwrap(),
            query_parser: QueryParser::for_index(&index, vec![word]),
            index: index,
            schema,
        }
    }

    pub fn reset_db(&mut self) {
        self.writer.delete_all_documents().unwrap();
        self.writer.commit().unwrap();
    }

    #[timed]
    pub fn candidates(&self, query: &str, top: usize, fuzzy: bool) -> candidates {
        let searcher = self.reader.searcher();
        let word = self.schema.get_field("word").unwrap();
        if fuzzy {
            let term = Term::from_field_text(word, query);
            let term_query = FuzzyTermQuery::new_prefix(term, 2, true);
            let (top_docs, count) = searcher
                .search(&term_query, &(TopDocs::with_limit(top), Count))
                .unwrap();

            top_docs
        } else {
            let term = Term::from_field_text(word, query);
            let term_query = TermQuery::new(term, IndexRecordOption::Basic);
            let (top_docs_t, count) = searcher
                .search(&term_query, &(TopDocs::with_limit(top), Count))
                .unwrap();
            debug_println!("TermQuery {}, count {}", query, count);
            if count >= top {
                top_docs_t
            } else {
                let q = query.to_owned() + ".*"; // regex seems to have poor scoring
                let term_query = RegexQuery::from_pattern(&q, word).unwrap();
                let (top_docs_r, count) = searcher
                    .search(&term_query, &(TopDocs::with_limit(top), Count))
                    .unwrap();

                debug_println!("RegexQuery {}, count {}", query, count);
                // the larger the more relevant

                let mut set: HashMap<DocAddress, f32> =
                    HashMap::from_iter(top_docs_r.into_iter().map(|(a, b)| (b, a)));
                top_docs_t.into_iter().for_each(|(s, d)| {
                    set.insert(d, s);
                });

                let mut v: candidates = set.into_iter().map(|(a, b)| (b, a)).collect();
                v.sort_by(|(a, _), (b, _)| b.partial_cmp(a).unwrap_or(Equal));

                dbg!(&v);
                v
            }
        }
    }

    #[timed]
    pub fn retrieve(&self, cand: candidate) -> Option<DefItemInDB> {
        let searcher = self.reader.searcher();
        let retrieved_doc = searcher.doc(cand.1).unwrap();
        if retrieved_doc.is_empty() {
            debug_assert!(false);
        }
        let v = retrieved_doc
            .get_first(self.schema.get_field("data").unwrap())
            .unwrap();
        if let Value::Bytes(b) = v {
            let d: DefItemInDB = Self::deserialize(b).unwrap();
            Some(d)
        } else {
            None
        }
    }

    #[timed]
    pub fn search(&self, query: &str, top: usize, fuzzy: bool) -> Vec<DefItemInDB> {
        let cands = self.candidates(query, top, fuzzy);

        let searcher = self.reader.searcher();
        let word = self.schema.get_field("word").unwrap();

        let top_docs = cands;

        let mut res: Vec<DefItemInDB> = vec![];

        for (score, doc_address) in top_docs {
            let retrieved_doc = searcher.doc(doc_address).unwrap();
            if retrieved_doc.is_empty() {
                debug_assert!(false);
                continue;
            }
            let v = retrieved_doc
                .get_first(self.schema.get_field("data").unwrap())
                .unwrap();
            if let Value::Bytes(b) = v {
                let d: DefItemInDB = Self::deserialize(b).unwrap();

                // https://github.com/quickwit-oss/tantivy/issues/563
                // TODO: Use master branch instead of the fork when ready
                // The fork has distance scoring done right

                res.push(d);
            }
            // debug_println!("{}", self.schema.to_json(&retrieved_doc));
        }

        res
    }

    // Vec<def_edit::DefNew>
    pub fn export_all_yaml(&self, path: &str) {
        let file = File::create(path).expect("Unable to open file");
        let mut defs: Vec<DefItemInDB> = vec![];
        let searcher = self.reader.searcher();

        for segment_reader in searcher.segment_readers() {
            let s = segment_reader.get_store_reader().unwrap();

            for dd in segment_reader.doc_ids_alive() {
                let doc = s.get(dd).unwrap();
                let v = doc
                    .get_first(self.schema.get_field("data").unwrap())
                    .unwrap();
                if let Value::Bytes(b) = v {
                    let d = Self::deserialize(&b).unwrap();
                    defs.push(d);
                }
            }
        }

        let flat = flatten(defs);
        let flat: Vec<DefItem> = flat.into_iter().map(|d| d.into()).collect();
        serde_yaml::to_writer(file, &flat).unwrap();
    }

    pub fn import_from_file(&mut self, path: &str, dict_name: &str) -> Result<(), Box<dyn Error>> {
        let ds = Def::load_yaml(&path, &dict_name)?;
        debug_println!("loaded {} DefNew", ds.len());
        self.import_defs(ds)?;

        Ok(())
    }

    pub fn import_defs(&mut self, defs: Vec<DefItem>) -> Result<(), Box<dyn Error>> {
        let ws: Vec<DefItemInDB> = defs.into_iter().map(|d| d.into()).collect();
        self.import_wrapped(ws);
        Ok(())
    }

    pub fn import_wrapped(&mut self, wrapped: Vec<DefItemInDB>) {
        let searcher = self.reader.searcher();
        let data = self.schema.get_field("data").unwrap();
        let word = self.schema.get_field("word").unwrap();
        for mut d in wrapped {
            let query = TermQuery::new(
                Term::from_field_text(word, d.word.as_ref()),
                IndexRecordOption::Basic,
            );
            let ve = searcher.search(&query, &TopDocs::with_limit(1)).unwrap();
            if ve.len() > 0 {
                let (s, doca) = ve[0];
                let doc = searcher.doc(doca).unwrap();
                let v = doc.get_first(data).unwrap();
                if let Value::Bytes(b) = v {
                    let mut it: DefItemInDB = Self::deserialize(b).unwrap();
                    // merge and override the in-db map with the new map
                    it = it.merge(&mut d);
                    self.writer
                        .delete_term(Term::from_field_text(word, d.word.as_str()));

                    let byt = bincode::serialize(&d).unwrap();
                    let k = byt.as_slice();
                    self.writer.add_document(doc!(word=>d.word,data=>k));
                }
            } else {
                self.writer.add_document(doc!(
                    word => d.word.clone(),
                    data => Self::serialize(&d).unwrap()
                ));
            }
        }
        self.writer.commit().unwrap();
    }

    pub fn stat(&self) -> stat {
        let searcher = self.reader.searcher();

        stat {
            words: searcher.num_docs(),
        }
    }
}

// Database, network, import/export, edit, how
// Checkout a dictionary for editing.
// Export as bincoded bytes
// Sync data from Locutus and update key/values
// or, build database on Locutus
// Locutus and the index might need different data structures, which is more ideal.
// so maybe not.

impl<'a> AnyDef<'a, Self> for Def {
    fn load_yaml(path: &str, name: &str) -> Result<Vec<DefItem>, Box<dyn Error>> {
        let file = File::open(path).expect("Unable to open file");
        let mut yaml_defs: Vec<Def> = serde_yaml::from_reader(file)?;
        for def in yaml_defs.iter_mut() {
            (*def).dictName = Some(name.to_owned());
        }

        let vec_d: Vec<DefItem> = yaml_defs.into_iter().map(|x| x.into()).collect();

        Ok(vec_d)
    }

    // Validates the old def sources are correctly parsed by serde, and converted
    fn check_yaml(path: &str, save: bool) {
        let file = File::open(path).expect("Unable to open file");
        let d = serde_yaml::Deserializer::from_reader(file);
        let mut unused = BTreeSet::new();

        let p: Vec<Self> = serde_ignored::deserialize(d, |path| {
            unused.insert(path.to_string());
        })
        .unwrap();

        if save {
            let consumed = File::create(path.replace(".yaml", ".1.yaml")).unwrap();

            let par = p.into_iter().collect::<Vec<Self>>();
            serde_yaml::to_writer(consumed, &par).unwrap();

            let converted = File::create(path.replace(".yaml", ".2.yaml")).unwrap();

            let con: Vec<DefItem> = par.into_iter().map(|x| x.into()).collect();
            serde_yaml::to_writer(converted, &con).unwrap();
        }
        println!("{:?}", unused);
    }
}

// To import DefNew from
pub trait AnyDef<'a, T: Deserialize<'a>> {
    fn load_yaml(path: &str, name: &str) -> Result<Vec<DefItem>, Box<dyn Error>>;

    fn check_yaml(path: &str, save: bool);
}

// store in database as wrapped
// unwrapped in sources
pub fn flatten(wr: Vec<DefItemInDB>) -> Vec<DefItem> {
    let mut res = Vec::new();
    for wrapper in wr.into_iter() {
        for (_, d) in wrapper.items.into_iter() {
            res.push(d)
        }
    }

    res
}

impl Def {
    fn normalize_def(mut self) -> Self {
        if self.groups.is_some() {
            self.definitions = self.groups;
            self.groups = None;
        }
        // d = recursive_path_shorten(d);
        self
    }

    fn normalize_def_ref(&mut self) {
        if self.groups.is_some() {
            std::mem::swap(&mut self.definitions, &mut self.groups);
            // let x = self.groups;
            self.groups = None;
        }
    }
}

pub use def::*;
pub mod def;
mod tests;

pub mod def_bin;
