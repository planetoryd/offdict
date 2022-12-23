use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::iter::FromIterator;

use fst::automaton::{self, Levenshtein, Subsequence, Union};
use fst::set::Stream;
use fst::Automaton;
use fst::{IntoStreamer, Set, Streamer};
use regex_automata::dfa::dense::{self, Config};

use timed;

use bitmask_enum::bitmask;

#[bitmask]
enum Score {
    Exact,
    ExactCaseIns,
    EqualLen,
    ExactPrefix,
    ExactPrefixCaseI,
    Leven1,
    Leven1Cap, // Leven1 with first letter capitalized ?
}

use std::cmp::Ordering;

fn collect_to_map<A: Automaton>(
    mut stream: Stream<A>,
    map: &mut BTreeMap<String, Score>,
    mut factor: Score,
    query: &str,
) -> Result<(), Box<dyn Error>> {
    let cap = 50;
    let mut curr = 0;
    while let Some(key) = stream.next() {
        let w = String::from_utf8(key.to_vec())?;
        map.insert(
            w.clone(),
            *map.get(&w).unwrap_or(&Score::none()) | score(query, &w) | factor,
        );
        curr += 1;
        if curr == cap {
            break;
        }
    }
    Ok(())
}

fn score(query: &str, curr: &str) -> Score {
    if query == curr {
        Score::Exact
    } else if query.to_lowercase() == curr.to_lowercase() {
        Score::ExactCaseIns
    } else if query.len() == curr.len() {
        Score::EqualLen
    } else {
        Score::none()
    }
}

impl Into<f32> for Score {
    fn into(self) -> f32 {
        if self.contains(Score::Exact) {
            0.0
        } else if self.contains(Score::ExactCaseIns) {
            0.01
        } else {
            if self.contains(Score::EqualLen) {
                0.02
            } else if self.contains(Score::ExactPrefix) {
                0.03
            } else if self.contains(Score::ExactPrefixCaseI) {
                0.04
            } else if self.contains(Score::Leven1) {
                0.05
            } else {
                1.0
            }
        }
    }
}

#[timed::timed]
pub fn suggest<D: AsRef<[u8]>>(
    set: &Set<D>,
    q: &str,
    d: u32,
    sub: bool,
) -> Result<Vec<String>, Box<dyn Error>> {
    let len = q.chars().count();
    let mut map: BTreeMap<String, Score> = BTreeMap::new();

    let keys = if len <= 2 {
        let star = automaton::Str::new(q).starts_with(); // matches strings starting with q
        let mut stream = set.search(&star).into_stream();

        collect_to_map(stream, &mut map, Score::ExactPrefix, q)?;
    } else {
        let star = automaton::Str::new(q).starts_with(); // matches strings starting with q
        let mut stream = set.search(&star).into_stream();
        collect_to_map(stream, &mut map, Score::ExactPrefix, q)?;

        let low = q.to_lowercase();
        let star = automaton::Str::new(&low).starts_with();
        let mut stream = set.search(&star).into_stream();
        collect_to_map(stream, &mut map, Score::ExactCaseIns, q)?;

        let lev = Levenshtein::new(q, 1)?;
        let pre = lev.starts_with(); // also matches strings starting with q when d=0

        let mut stream = set.search(&pre).into_stream();
        collect_to_map(stream, &mut map, Score::Leven1, q)?;
    };

    let mut ve: Vec<(String, Score)> = map.into_iter().collect();
    ve.sort_by(|a, b| {
        let sa: f32 = a.1.into();
        let sb: f32 = b.1.into();
        sa.partial_cmp(&sb).unwrap_or(Ordering::Equal)
    });
    //     // let sub = Subsequence::new(q);

    // let pattern = format!(r"(?i){}", q); // (?i) for case insensitive
    // let dfa = dense::Builder::new()
    //     .configure(Config::new().anchored(!sub))
    //     .build(pattern.as_ref())
    //     .unwrap();

    // // let u = dfa.union(sub);

    // let mut stream = set.search(&dfa).into_stream();

    // collect_stream(stream)?

    Ok(ve.into_iter().map(|x| x.0).collect())
}
