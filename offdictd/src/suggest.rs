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

// Different strategies for short strings

fn collect_stream<A: Automaton>(mut stream: Stream<A>) -> Result<Vec<String>, Box<dyn Error>> {
    let mut keys = vec![];

    let cap = 50;
    let mut curr = 0;
    while let Some(key) = stream.next() {
        keys.push(String::from_utf8(key.to_vec())?);
        curr += 1;
        if curr == cap {
            break;
        }
    }
    Ok(keys)
}

fn score(query: &str, curr: &str) -> u8 {
    if query == curr {
        0
    } else {
        255
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

    let keys = if len <= 2 {
        let star = automaton::Str::new(q).starts_with(); // matches strings starting with q
        let mut stream = set.search(&star).into_stream();

        collect_stream(stream)?
    } else {
        let star = automaton::Str::new(q).starts_with(); // matches strings starting with q
        let mut stream = set.search(&star).into_stream();

        let pre1 = collect_stream(stream)?;
        let map1 = BTreeMap::from_iter(pre1.into_iter().map(|x| (x, 25)));

        let lev = Levenshtein::new(q, 1)?;
        let pre = lev.starts_with(); // also matches strings starting with q when d=0

        let mut stream = set.search(&pre).into_stream();

        let mut arr = collect_stream(stream)?;
        let mut map2 = BTreeMap::from_iter(arr.into_iter().map(|x| {
            let s = score(q, &x);
            (x, s)
        }));

        for (w, s) in map2.iter_mut() {
            *s = *map1.get(w).unwrap_or(&255);
        }
        map2.extend(map1.into_iter());

        let mut ax: Vec<(String, u8)> = map2.into_iter().collect();
        ax.sort_by_key(|x|x.1);

        ax.into_iter().map(|x|x.0).collect()
    };

    //     // let sub = Subsequence::new(q);

    // let pattern = format!(r"(?i){}", q); // (?i) for case insensitive
    // let dfa = dense::Builder::new()
    //     .configure(Config::new().anchored(!sub))
    //     .build(pattern.as_ref())
    //     .unwrap();

    // // let u = dfa.union(sub);

    // let mut stream = set.search(&dfa).into_stream();

    // collect_stream(stream)?

    Ok(keys)
}
