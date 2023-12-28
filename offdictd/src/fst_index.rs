use std::borrow::Borrow;
use std::collections::BTreeMap;
use std::error::Error;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::iter::FromIterator;
use std::{default, fmt};

use debug_print::debug_println;
use fst::automaton::{self, Levenshtein, Subsequence, Union};
use fst::set::Stream;
use fst::{Automaton, SetBuilder};
use fst::{IntoStreamer, Set, Streamer};

use timed;

use crate::*;
use bitmask_enum::bitmask;

#[bitmask]
#[derive(Default)]
enum Flags {
    Exact,
    ExactCaseIns,
    EqualLen,
    ExactPrefix,
    ExactPrefixCaseI,
    Leven1,
    Leven1Cap, // Leven1 with first letter capitalized ?
    Leven2,
    LevenFull,
}

#[derive(Default, Debug)]
struct Metrics {
    short: u32,
    flags: Flags,
    num: u32,
}

use std::cmp::Ordering;

fn collect_to_map<A: Automaton>(
    mut stream: Stream<A>,
    map: &mut BTreeMap<String, Metrics>,
    mut factor: Flags,
    query: &str,
    cap: i32,
) -> Result<()> {
    let mut curr = 0;
    while let Some(key) = stream.next() {
        let w = String::from_utf8(key.to_vec())?;
        map.insert(
            w.clone(),
            Metrics {
                short: 0,
                flags: (*map.get(&w).unwrap_or(&Metrics::default())).flags
                    | score(query, &w)
                    | factor,
                ..Default::default()
            },
        );
        curr += 1;
        if curr == cap {
            break;
        }
    }
    Ok(())
}

fn score(query: &str, curr: &str) -> Flags {
    if query == curr {
        Flags::Exact
    } else if query.to_lowercase() == curr.to_lowercase() {
        Flags::ExactCaseIns
    } else if query.len() == curr.len() {
        Flags::EqualLen
    } else {
        Flags::none()
    }
}

impl Into<f32> for &Metrics {
    fn into(self) -> f32 {
        if self.flags.contains(Flags::Exact) {
            0.0
        } else if self.flags.contains(Flags::ExactCaseIns) {
            0.1
        } else {
            let mut step1 = if self.flags.contains(Flags::EqualLen) {
                0.2
            } else if self.flags.contains(Flags::ExactPrefix) {
                0.2
            } else if self.flags.contains(Flags::ExactPrefixCaseI) {
                0.4
            } else if self.flags.contains(Flags::Leven1) {
                0.7
            } else {
                1.0
            };
            if self.flags.contains(Flags::LevenFull) {
                step1 -= 0.1;
            }
            step1 + (self.short / self.num) as f32 * 0.1
        }
    }
}

pub use fst::Set as fstset;
pub type fstmmap = fstset<Mmap>;

impl Indexer for fstmmap {
    const FILE_NAME: &'static str = "fst";
    type Param = bool;
    fn load_file(pp: &Path) -> Result<Self> {
        println!("loading FST index");
        let mmap = unsafe { Mmap::map(&File::open(pp).unwrap()).unwrap() };
        let set = fst::Set::new(mmap).unwrap();
        Ok(set)
    }
    #[timed]
    fn query(&self, q: &str, expensive: bool) -> Result<candidates> {
        let set = self;
        let len = q.chars().count();
        let mut map: BTreeMap<String, Metrics> = BTreeMap::new();
        let keys = if len <= 2 {
            let star = automaton::Str::new(q).starts_with(); // matches strings starting with q
            let stream = set.search(&star).into_stream();

            collect_to_map(stream, &mut map, Flags::ExactPrefix, q, 50)?;
        } else {
            let star = automaton::Str::new(q).starts_with(); // matches strings starting with q
            let stream = set.search(&star).into_stream();
            collect_to_map(stream, &mut map, Flags::ExactPrefix, q, 50)?;
            // Capping search results here leads to suboptimal results

            let low = q.to_lowercase();
            let star = automaton::Str::new(&low).starts_with();
            let stream = set.search(&star).into_stream();
            collect_to_map(stream, &mut map, Flags::ExactCaseIns, q, 50)?;

            let lev = Levenshtein::new(q, 1)?;
            let pre = lev.starts_with(); // also matches strings starting with q when d=0

            let stream = set.search(&pre).into_stream();
            collect_to_map(stream, &mut map, Flags::Leven1, q, 50)?;
        };

        // Fallbacks, levenshtein of distance 2
        if expensive {
            if map.is_empty() {
                let lev = Levenshtein::new(q, 2)?;
                let pre = lev.starts_with();

                let stream = set.search(&pre).into_stream();
                collect_to_map(stream, &mut map, Flags::Leven2, q, 50)?;
            }
            let lev = Levenshtein::new(q, 1)?;
            let stream = set.search(&lev).into_stream();
            collect_to_map(stream, &mut map, Flags::LevenFull, q, 50)?;
        }

        let mut ve: Vec<(String, Metrics)> = map.into_iter().collect();
        let le = ve.len() as u32;
        ve.sort_unstable_by_key(|x| x.0.len());
        for (a, b) in ve.iter_mut().enumerate() {
            b.1.short = a as u32;
            b.1.num = le;
        }
        ve.sort_by(|a, b| {
            let sa: f32 = a.1.borrow().into();
            let sb: f32 = b.1.borrow().into();
            sa.partial_cmp(&sb).unwrap_or(Ordering::Equal)
        });
        Ok(ve.into_iter().map(|x| x.0).collect())
    }
    fn build_all(sorted: impl IntoIterator<Item = String>, px: &Path) -> Result<()> {
        let mut w = BufWriter::new(File::create(px).unwrap());
        let mut bu = SetBuilder::new(&mut w).unwrap();
        sorted.into_iter().for_each(|k| {
            bu.insert(k).unwrap();
        });
        bu.finish()?;
        Ok(())
    }
    fn count(&self) -> usize {
        self.len()
    }
}

impl Diverge for offdict<fstmmap> {
    type Ix = fstmmap;
    fn search(&self, query: &str, num: usize, expensive: bool) -> Result<Vec<DefItemWrapped>> {
        let mut cands = self.candidates(query, expensive)?;
        cands.truncate(num);
        let mut res: Vec<DefItemWrapped> = vec![];
        for s in cands {
            res.push(self.retrieve(s).unwrap());
        }
        Ok(res)
    }
}
