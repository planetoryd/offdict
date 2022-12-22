#![allow(unused_variables)]

use fst::SetBuilder;
pub use serde::{Deserialize, Serialize};
pub use serde_yaml::{self};

pub use tokio::io::{AsyncReadExt, AsyncWriteExt};

use std::collections::{self, BTreeMap, BTreeSet, HashMap};

use std::io::BufWriter;
use std::iter::FromIterator;

use std::str::FromStr;
use std::{self, fs, vec};

// use bson::{self, Array, Serializer};
pub use std::cmp::min;

pub use std::error::Error;

pub use std::fs::{create_dir_all, File};
pub use std::io::{Read, Write};
use std::path::{Path, PathBuf};

// use lazy_static;
// use regex::Regex;

use timed::timed;

use debug_print::debug_println;
use std::cmp::Ordering::Equal;

use jammdb;
use memmap2::Mmap;
use serde_ignored;

pub type DefItemInDB = def_bin::WrapperDef;
pub type DefItem = def_bin::Def;
pub type DB = jammdb::DB;

pub struct stat {
    pub words: usize,
}

pub type candidate = String;
pub type candidates = Vec<candidate>;
pub struct offdict {
    db: jammdb::DB,
    fst_set: Option<fst::Set<Mmap>>,
    data_path: PathBuf,
}

#[test]
pub fn test_guess_name() {
    let g1 = get_dictname_from_path("/hdd/OpenMdicts/简明英汉汉英词典.1.yaml".to_owned()).unwrap();
    dbg!(&g1);
    assert_eq!(g1, "简明英汉汉英词典".to_owned());
    let g2 =
        get_dictname_from_path("/hdd/OpenMdicts/汉语大词典(简体精排).34.yaml".to_owned()).unwrap();
    dbg!(&g2);
    let g3 = get_dictname_from_path(
        "/hdd/OpenMdicts/柯林斯COBUILD高阶英汉双解学习词典.4.yaml".to_owned(),
    )
    .unwrap();
    dbg!(&g3);
}

pub fn get_dictname_from_path(path: String) -> Option<String> {
    let pa = PathBuf::from(&path);
    let s = pa.file_stem().unwrap().to_str().unwrap().split_once(".");

    if s.is_none() {
        None
    } else {
        Some(s.unwrap().0.to_owned())
    }
}

impl offdict {
    pub fn serialize<T: Serialize>(v: &T) -> Result<Vec<u8>, Box<bincode::ErrorKind>> {
        bincode::serialize(v)
    }

    pub fn deserialize<'a, T: Deserialize<'a>>(v: &'a [u8]) -> Result<T, Box<bincode::ErrorKind>> {
        bincode::deserialize(v)
    }

    pub fn load_fst(pp: PathBuf) -> fst::Set<Mmap> {
        let mmap = unsafe { Mmap::map(&File::open(pp).unwrap()).unwrap() };
        let set = fst::Set::new(mmap).unwrap();
        set
    }

    pub fn open_db(path: String) -> Self {
        let db;
        let mut pp = PathBuf::from(path);
        let p2 = pp.clone();
        if !pp.is_dir() {
            create_dir_all(&pp).unwrap();
        }
        pp.push("dicts.db");
        let n = pp.to_str().unwrap();
        db = jammdb::DB::open(n).unwrap();
        pp.pop();
        pp.push("fst");

        if pp.exists() {
            offdict {
                db,
                fst_set: Some(offdict::load_fst(pp)),
                data_path: p2,
            }
        } else {
            offdict {
                db,
                fst_set: None,
                data_path: p2,
            }
        }
    }

    pub fn reset_db(&mut self) {
        let t = self.db.tx(true).unwrap();
        for b in t.buckets() {
            t.delete_bucket(b.0.name()).unwrap();
        }
        t.commit().unwrap();
    }

    #[timed]
    pub fn candidates(&self, query: &str, d: u32, sub: bool) -> candidates {
        if self.fst_set.is_some() {
            suggest::suggest(self.fst_set.as_ref().unwrap(), query, d, sub).unwrap()
        } else {
            vec![]
        }
    }

    #[timed]
    pub fn retrieve(&self, cand: candidate) -> Option<DefItemInDB> {
        let t = self.db.tx(false).unwrap();
        let b = t.get_bucket(cand.as_bytes());
        if b.is_ok() {
            let b = b.unwrap();
            let r = def_bin::WrapperDef {
                items: BTreeMap::from_iter(b.kv_pairs().map(|x| {
                    (
                        String::from_utf8(x.key().to_vec()).unwrap(),
                        Self::deserialize(x.value()).unwrap(),
                    )
                })),
                word: cand,
            };

            Some(r)
        } else {
            None
        }
    }

    #[timed]
    pub fn search(&self, query: &str, num: usize, fuzzy: bool) -> Vec<DefItemInDB> {
        let mut cands = self.candidates(query, 2, fuzzy);
        cands.truncate(num);
        let mut res: Vec<DefItemInDB> = vec![];
        let t = self.db.tx(false).unwrap();

        for s in cands {
            res.push(self.retrieve(s).unwrap());
            // debug_println!("{}", self.schema.to_json(&retrieved_doc));
        }
        res
    }

    pub fn export_all_yaml(&self, path: &str) {
        let file = File::create(path).expect("Unable to open file");
        // let mut defs: Vec<DefItemInDB> = vec![];
        let mut flat: Vec<DefItem> = vec![];
        let t = self.db.tx(false).unwrap();
        for b in t.buckets() {
            let bb = t.get_bucket(b.0.name()).unwrap();
            for x in bb.kv_pairs() {
                flat.push(Self::deserialize(x.value()).unwrap())
            }
        }
        t.commit().unwrap();

        serde_yaml::to_writer(file, &flat).unwrap();
    }

    pub fn import_glob(&mut self, path: &str) -> Result<(), Box<dyn Error>> {
        let options = glob::MatchOptions {
            case_sensitive: false,
            ..Default::default()
        };

        let mut pendin: Vec<(String, String)> = vec![];

        for entry in glob::glob_with(path, options)? {
            let ee = entry?;
            let entr = ee.to_str().unwrap();
            let dict_name = get_dictname_from_path(entr.to_owned());
            if let Some(s) = dict_name {
                println!("import {} as {}", entr, s.as_str());
                pendin.push((entr.to_owned(), s));
            } else {
                return Err(format!("Error getting dict name for {}", entr).into());
            }
        }

        pendin.into_iter().for_each(|(p, e)| {
            self.import_from_file(p.as_str(), e.as_str()).unwrap();
        });

        Ok(())
    }

    pub fn import_from_file(&mut self, path: &str, dict_name: &str) -> Result<(), Box<dyn Error>> {
        let ds = Def::load_yaml(&path, &dict_name)?;
        debug_println!("loaded {} Defs", ds.len());
        self.import_defs(ds)?;

        Ok(())
    }

    #[timed]
    pub fn import_defs(&mut self, defs: Vec<DefItem>) -> Result<(), Box<dyn Error>> {
        let ws: Vec<DefItemInDB> = defs.into_iter().map(|d| d.into()).collect();
        self.import_wrapped(ws);
        Ok(())
    }

    pub fn import_wrapped(&mut self, wrapped: Vec<DefItemInDB>) {
        let mut t = self.db.tx(true).unwrap();

        for mut incoming_w in wrapped {
            let b = t.get_or_create_bucket(incoming_w.word.as_bytes()).unwrap();
            for (k, v) in incoming_w.items {
                let by = Self::serialize(&v).unwrap();
                b.put(k, by).unwrap();
            }
        }

        t.commit().unwrap();
    }

    pub fn stat(&self) -> stat {
        let t = self.db.tx(false).unwrap();

        stat {
            words: t.buckets().count(),
        }
    }

    #[timed]
    pub fn build_fst_from_db(&mut self) -> usize {
        let mut px = self.data_path.clone();
        px.push("fst");
        let mut w = BufWriter::new(File::create(&px).unwrap());
        let mut bu = SetBuilder::new(w).unwrap();

        let t = self.db.tx(false).unwrap();
        let mut c = 0 as usize;
        for b in t.buckets() {
            bu.insert(b.0.name()).unwrap();
            c += 1;
        }

        bu.finish().unwrap();
        self.fst_set = Some(Self::load_fst(px));

        c
    }
}

// Database, network, import/export, edit, how
// Checkout a dictionary for editing.
// Export as bincoded bytes
// Sync data from Locutus and update key/values
// or, build database on Locutus
// Locutus and the index might need different data structures, which is more ideal.
// so maybe not.

// Result of yaml checking
#[derive(Serialize)]
struct DefCheck {
    def: Def,
    has_non_empty_alternative: bool,
}

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

    fn check_yaml_defs(
        imported_Defs: Vec<Def>,
        save: bool,
        mut unused: BTreeSet<String>,
        path: &str,
    ) {
        let imported_words: BTreeSet<&str>;

        imported_words = BTreeSet::from_iter(
            imported_Defs
                .iter()
                .map(|x| x.word.as_ref().unwrap().as_str()),
        );

        // Outputs a cleaned up source, best effort
        if save {
            let empty_Defs: Vec<Def>;
            let mut non_empty_Defs: Vec<Def>;
            empty_Defs = imported_Defs
                .iter()
                .filter(|d| d.empty_())
                .map(|d| d.clone())
                .collect();
            non_empty_Defs = imported_Defs
                .iter()
                .filter(|d| !d.empty_())
                .map(|d| d.clone())
                .collect();

            let mut check_res: Vec<DefCheck> = vec![];
            let mut with_alt: usize = 0;
            for d in empty_Defs.iter() {
                let c = imported_words.contains::<str>(d.word.as_ref().unwrap().as_str());
                check_res.push(DefCheck {
                    def: d.clone(),
                    has_non_empty_alternative: c,
                });
                // Empty defs with an alternative can be removed without pity
                // Results that are not factually empty means the code has to be revised.
                if c {
                    with_alt += 1;
                }
            }

            let mut imported_words_m: collections::BTreeMap<String, bool> =
                collections::BTreeMap::new();

            let unique_Defs: Vec<Def> = non_empty_Defs
                .into_iter()
                .filter(|x| {
                    if *imported_words_m
                        .get::<str>(x.word.as_ref().unwrap().as_ref())
                        .unwrap_or(&false)
                    {
                        false
                    } else {
                        imported_words_m.insert(x.word.as_ref().unwrap().clone(), true);
                        true
                    }
                })
                .collect();

            println!(
                "Num empty {}, num total {}, empty but with alt {}",
                empty_Defs.len(),
                imported_Defs.len(),
                with_alt
            );

            let cleaned: Vec<Def> = unique_Defs.into_iter().map(|i| i.cleanup()).collect();
            let mut pb = PathBuf::from_str(path).unwrap();
            let fname = pb.file_name().unwrap().to_os_string();
            pb.pop();
            pb.push("checked");
            create_dir_all(&pb).unwrap();

            let f_processed = File::create(pb.join(fname.clone())).unwrap();
            let f_empty =
                File::create(pb.join(fname.to_str().unwrap().replace(".yaml", ".e.yaml"))).unwrap();

            serde_yaml::to_writer(f_empty, &check_res).unwrap();
            serde_yaml::to_writer(f_processed, &cleaned).unwrap();
        }
        println!("{:?}", unused);
    }

    // Validates the old def sources are correctly parsed by serde, and converted
    // Empty defs were caused by buggy .mdx parsers. They are reversed-engineered, anyway
    fn check_yaml(path: &str, save: bool) {
        let file = File::open(path).expect("Unable to open file");
        let d = serde_yaml::Deserializer::from_reader(file);
        let mut unused = BTreeSet::new();

        Self::check_yaml_defs(
            serde_ignored::deserialize(d, |path| {
                unused.insert(path.to_string());
            })
            .unwrap(),
            save,
            unused,
            path,
        )
    }
}

// To import DefNew from
pub trait AnyDef<'a, T: Deserialize<'a>> {
    fn load_yaml(path: &str, name: &str) -> Result<Vec<DefItem>, Box<dyn Error>>;

    fn check_yaml(path: &str, save: bool);
    fn check_yaml_defs(imported_Defs: Vec<Def>, save: bool, unused: BTreeSet<String>, path: &str);
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

pub mod suggest;

pub mod def_bin;
