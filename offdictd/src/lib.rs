#![allow(unused_variables)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![feature(async_closure)]

use anyhow::bail;
pub use anyhow::Result;
use fst::SetBuilder;
pub use serde::{Deserialize, Serialize};

pub use serde_yaml::{self};

pub use tokio::io::{AsyncReadExt, AsyncWriteExt};

use std::borrow::Borrow;
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

use timed::timed;

use debug_print::debug_println;
use std::cmp::Ordering::Equal;

use def_bin::DBKey;
use memmap2::Mmap;
use rocksdb::{BlockBasedOptions, Options, ReadOptions, SliceTransform, DB as rocks};
use serde_ignored;

pub type DefItemWrapped = def_bin::WrapperDef;
pub type DefItem = def_bin::Def;
pub type DB = rocks;

pub struct stat {
    pub words: usize,
}

pub type candidate = String;
pub type candidates = Vec<candidate>;
pub struct offdict<index: Indexer> {
    db: rocks,
    fst_set: Option<index>,
    data_path: PathBuf,
    pub set_input: Option<fn(&str, bool) -> Result<bool>>,
}

pub trait Indexer {
    const FILE_NAME: &'static str;
    fn load_file(pp: &Path) -> Self;
    fn query(&self, query: &str, expensive: bool) -> Result<candidates>;
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

impl<Ix: Indexer> offdict<Ix> {
    pub fn serialize<T: Serialize>(v: &T) -> Result<Vec<u8>, Box<bincode::ErrorKind>> {
        bincode::serialize(v)
    }

    pub fn deserialize<'a, T: Deserialize<'a>>(v: &'a [u8]) -> Result<T, Box<bincode::ErrorKind>> {
        bincode::deserialize(v)
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

        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.set_prefix_extractor(SliceTransform::create("pre", |bs| DBKey::slice(bs).0, None));
        let mut tableopts = BlockBasedOptions::default();
        tableopts.set_index_type(rocksdb::BlockBasedIndexType::HashSearch);
        opts.set_block_based_table_factory(&tableopts);

        db = rocks::open(&opts, n).unwrap();
        pp.pop();
        pp.push("fst");

        if pp.exists() {
            offdict {
                db,
                fst_set: Some(Ix::load_file(&pp)),
                data_path: p2,
                set_input: None,
            }
        } else {
            offdict {
                db,
                fst_set: None,
                data_path: p2,
                set_input: None,
            }
        }
    }

    pub fn reset_db(&mut self) {
        self.db.drop_cf("default").unwrap();
    }

    pub fn candidates(&self, query: &str, expensive: bool, sub: bool) -> Result<candidates> {
        if let Some(index) = &self.fst_set {
            index.query(query, expensive)
        } else {
            Ok(Default::default())
        }
    }

    pub fn retrieve(&self, cand: candidate) -> Option<DefItemWrapped> {
        let mut items: BTreeMap<String, def_bin::Def> = BTreeMap::new();
        for res in self.db.prefix_iterator(DBKey::from(cand.as_str(), "")) {
            if res.is_ok() {
                let (k, v) = res.unwrap();
                items.insert(
                    String::from_utf8(DBKey::slice(&k).1.to_vec()).unwrap(),
                    Self::deserialize(&v).unwrap(),
                );
            }
        }
        if items.len() > 0 {
            Some(def_bin::WrapperDef { items, word: cand })
        } else {
            None
        }
    }

    #[timed]
    pub fn search(&self, query: &str, num: usize, expensive: bool) -> Result<Vec<DefItemWrapped>> {
        let mut cands = self.candidates(query, expensive, false)?;
        cands.truncate(num);
        let mut res: Vec<DefItemWrapped> = vec![];

        for s in cands {
            res.push(self.retrieve(s).unwrap());
            // debug_println!("{}", self.schema.to_json(&retrieved_doc));
        }

        Ok(res)
    }

    pub fn export_all_yaml(&self, path: &str) {
        let file = File::create(path).expect("Unable to open file");
        // let mut defs: Vec<DefItemInDB> = vec![];
        let mut flat: Vec<DefItem> = vec![];
        for r in self.db.iterator(rocksdb::IteratorMode::Start) {
            if r.is_ok() {
                let (k, v) = r.unwrap();
                // let k: DBKey = Self::deserialize(&k).unwrap();
                flat.push(Self::deserialize(&v).unwrap());
            }
        }

        serde_yaml::to_writer(file, &flat).unwrap();
    }

    pub fn import_glob(&mut self, path: &str) -> Result<()> {
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
                bail!("Error getting dict name for {}", entr)
            }
        }

        pendin.into_iter().for_each(|(p, e)| {
            self.import_from_file(p.as_str(), e.as_str()).unwrap();
        });

        Ok(())
    }

    pub fn import_from_file(&mut self, path: &str, dict_name: &str) -> Result<()> {
        let ds = Def::load_yaml(&path, &dict_name)?;
        debug_println!("loaded {} Defs", ds.len());
        self.import_defs(ds)?;

        Ok(())
    }

    #[timed]
    pub fn import_defs(&mut self, defs: Vec<DefItem>) -> Result<()> {
        let ws: Vec<DefItemWrapped> = defs.into_iter().map(|d| d.into()).collect();
        self.import_wrapped(ws);
        Ok(())
    }

    pub fn import_wrapped(&mut self, wrapped: Vec<DefItemWrapped>) {
        for incoming_w in wrapped {
            for (k, v) in incoming_w.items {
                self.db.put(v.key(), Self::serialize(&v).unwrap()).unwrap();
            }
        }
    }

    pub fn stat(&self) -> stat {
        let t = self.db.iterator(rocksdb::IteratorMode::Start).count();

        stat { words: t }
    }

    #[timed]
    pub fn build_fst_from_db(&mut self) -> usize {
        let mut px = self.data_path.clone();
        px.push(Ix::FILE_NAME);
        let mut w = BufWriter::new(File::create(&px).unwrap());
        let mut bu = SetBuilder::new(w).unwrap();

        let mut c = 0 as usize;
        let mut set: BTreeSet<String> = BTreeSet::new();
        for res in self.db.iterator(rocksdb::IteratorMode::Start) {
            if res.is_ok() {
                let (k, v) = res.unwrap();
                let word = String::from_utf8(DBKey::slice(&k).0.to_vec()).unwrap();
                set.insert(word);
                c += 1;
            }
        }

        debug_println!("word set len {}", set.len());
        let mut sorted: Vec<String> = set.into_iter().collect();
        sorted.sort();
        sorted.into_iter().for_each(|k| {
            bu.insert(k).unwrap();
        });

        bu.finish().unwrap();
        self.fst_set = Some(Ix::load_file(&px));

        c
    }
}

// Result of yaml checking
#[derive(Serialize)]
struct DefCheck {
    def: Def,
    has_non_empty_alternative: bool,
}

impl<'a> AnyDef<'a, Self> for Def {
    fn load_yaml(path: &str, name: &str) -> Result<Vec<DefItem>> {
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
    fn load_yaml(path: &str, name: &str) -> Result<Vec<DefItem>>;

    fn check_yaml(path: &str, save: bool);
    fn check_yaml_defs(imported_Defs: Vec<Def>, save: bool, unused: BTreeSet<String>, path: &str);
}

// store in database as wrapped
// unwrapped in sources
pub fn flatten(wr: Vec<DefItemWrapped>) -> Vec<DefItem> {
    let mut res = Vec::new();
    for wrapper in wr.into_iter() {
        for (_, d) in wrapper.items.into_iter() {
            res.push(d)
        }
    }

    res
}

pub fn flatten_human(wr: Vec<DefItemWrapped>) -> Vec<Def> {
    let mut res = Vec::new();
    for wrapper in wr.into_iter() {
        res.extend(wrapper.vec_human())
    }
    res
}

impl Def {
    pub fn normalize_def(mut self) -> Self {
        if self.groups.is_some() {
            self.definitions = self.groups;
            self.groups = None;
        }
        if self.definitions.is_some() {
            self.definitions = Some(
                self.definitions
                    .unwrap()
                    .into_iter()
                    .map(|x| x.normalize_def())
                    .collect(),
            );
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

use clap::{Parser, Subcommand};
#[allow(non_camel_case_types)]
#[derive(Debug, Subcommand)]
enum Commands {
    #[command(
        about = "Import definitions from an yaml file",
        arg_required_else_help = false
    )]
    yaml {
        #[arg(short = 'p', required = true)]
        path: String,
        #[arg(short = 'c', long)]
        check: bool,
        #[arg(short = 's', long)]
        save: bool,
    },
    #[command(about = "Stats")]
    stat {},
    #[command(about = "Fuzzy query (prefix)")]
    lookup {
        query: String,
    },
    // TODO: bincode import
    // #[command(about = "Convert an yaml file to cbor")]
    // cbor {
    //     // Converts a yaml to cbor and save it.
    //     #[arg(short = 'p')]
    //     path: String,
    //     #[arg(short = 'n')]
    //     name: String, // Name to be displayed
    // },
    reset {},
    #[command(about = "Build index, required after adding or removing words")]
    build {},
}

#[derive(Parser, Debug)]
#[command(about = "Offline dictionary", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

// continue running ?
pub fn tui<Ix: Indexer>(db_w: &mut offdict<Ix>) -> Result<bool> {
    let args = Cli::parse();

    match args.command {
        Some(Commands::yaml { path, check, save }) => {
            if check {
                let options = glob::MatchOptions {
                    case_sensitive: false,
                    ..Default::default()
                };

                for entry in glob::glob_with(&path, options)? {
                    let entr = entry?;
                    println!("checking {}", entr.to_str().unwrap());
                    Def::check_yaml(entr.to_str().unwrap(), save);
                }

                return Ok(false);
            } else {
                match db_w.import_glob(&path) {
                    Ok(()) => println!("imported"),
                    Err(e) => println!("{:?}", e),
                }
            }
            Ok(false)
        }
        Some(Commands::stat {}) => {
            let s = db_w.stat();
            println!("Words in database: {}", s.words);
            Ok(false)
        }
        Some(Commands::lookup { query }) => {
            for d in db_w.search(&query, 1, true)? {
                let list: Vec<Def> = d.vec_human();
                println!("{}", serde_yaml::to_string::<Vec<Def>>(&list)?)
            }
            Ok(false)
        }
        Some(Commands::reset {}) => {
            db_w.reset_db();
            println!("reset.");
            Ok(false)
        }
        Some(Commands::build {}) => {
            let c = db_w.build_fst_from_db();
            println!("built, {} words", c);
            Ok(true)
        }
        None => Ok(true),
    }
}

use std::sync::{Arc, RwLock};
use tokio::{self};
use warp::Filter;

pub mod config;

#[derive(Serialize, Deserialize)]
pub struct Stat {
    words: u64,
}

#[derive(Deserialize, Default)]
pub struct ApiOpts {
    expensive: bool,
}

#[derive(Deserialize, Default, Serialize)]
pub struct SetRes {
    defs: bool, // true means result is empty
}

pub async fn serve<Ix: Indexer + Send + Sync + 'static>(
    db_tok: Arc<RwLock<offdict<Ix>>>,
) -> Result<()> {
    let db_t1 = db_tok.clone();
    let lookup = warp::get()
        .and(warp::path("q"))
        .and(warp::path::param::<String>())
        .and(
            warp::query::<ApiOpts>()
                .map(Some)
                .or_else(|_| async { Ok::<(Option<ApiOpts>,), std::convert::Infallible>((None,)) }),
        )
        .map(move |word: String, opts: Option<ApiOpts>| {
            let db_r = db_t1.read().unwrap();
            let word = percent_encoding::percent_decode_str(&word)
                .decode_utf8()
                .unwrap()
                .to_string();
            warp::reply::json(&api_q(&db_r, &word, opts.unwrap_or_default()).unwrap())
        });

    let stat = warp::get()
        .and(warp::path("stat"))
        .map(|| warp::reply::json(&Stat { words: 0 }));

    let set = warp::get()
        .and(warp::path("set"))
        .and(warp::path::param::<String>())
        .and(
            warp::query::<ApiOpts>()
                .map(Some)
                .or_else(|_| async { Ok::<(Option<ApiOpts>,), std::convert::Infallible>((None,)) }),
        )
        .map(move |word: String, opts: Option<ApiOpts>| {
            let db_r = db_tok.read().unwrap();
            let word = percent_encoding::percent_decode_str(&word)
                .decode_utf8()
                .unwrap()
                .to_string();
            let mut r: SetRes = SetRes { defs: false };
            if db_r.set_input.is_some() {
                r.defs = db_r.set_input.unwrap()(&word, false).unwrap();
            }
            warp::reply::json(&r)
        });

    Ok(warp::serve(lookup.or(stat).or(set))
        .run(([0, 0, 0, 0], 3030)) // XXX: this has to be hard coded, who cares
        .await)
}

pub async fn repl<Ix: Indexer>(db_: Arc<RwLock<offdict<Ix>>>) -> Result<()> {
    loop {
        let li = readline().await.unwrap();
        let li = li.trim();
        if li.is_empty() {
            continue;
        } else {
            let db = db_.read().unwrap();

            match respond(li, db.borrow()) {
                Ok(quit) => {
                    if quit {
                        break Ok(());
                    }
                }
                Err(err) => {
                    write!(std::io::stdout(), "{}", err)
                        .map_err(|e| e.to_string())
                        .unwrap();
                    std::io::stdout()
                        .flush()
                        .map_err(|e| e.to_string())
                        .unwrap();
                }
            }
        }
    }
}
async fn readline() -> Result<String> {
    let mut out = tokio::io::stdout();
    out.write_all(b"@ ").await?;
    out.flush().await?;
    let mut buffer = Vec::new();
    tokio::io::stdin().read(&mut buffer).await?;
    let stdin = tokio::io::stdin();
    let reader = tokio::io::BufReader::new(stdin);
    let mut lines = tokio::io::AsyncBufReadExt::lines(reader);
    Ok(lines.next_line().await?.unwrap())
}

pub fn api_q(db: &offdict<impl Indexer>, query: &str, opts: ApiOpts) -> Result<Vec<Def>> {
    println!("\nq: {}", query);

    let mut arr = db.search(query, 30, opts.expensive)?;
    let mut def_list = flatten_human(arr);

    Ok(def_list)
}

fn respond(line: &str, db: &offdict<impl Indexer>) -> Result<bool> {
    let mut arr = db.search(line, 2, true)?;

    println!("{} results", arr.len());
    arr.truncate(2);
    for d in arr.into_iter() {
        println!(
            "{}",
            serde_yaml::to_string::<Vec<Def>>(&d.vec_human()).unwrap()
        );
    }

    Ok(false)
}

pub use def::*;
pub mod def;
mod tests;

pub mod fst_index;

pub mod def_bin;
