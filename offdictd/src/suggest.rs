use std::error::Error;
use std::fmt;
use std::fs::File;
use std::io::{BufRead, BufReader};

use fst::automaton::{self, Levenshtein, Subsequence, Union};
use fst::set::Stream;
use fst::Automaton;
use fst::{IntoStreamer, Set, Streamer};
use regex_automata::dfa::dense::{self, Config};

use timed;

// Different strategies for short strings

fn collect_stream<A: Automaton>(mut stream: Stream<A>) -> Result<Vec<String>, Box<dyn Error>> {
    let mut keys = vec![];

    let cap = 30;
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
    } else if len <= 4 {
        // let sub = Subsequence::new(q);

        let pattern = format!(r"(?i){}", q); // (?i) for case insensitive
        let dfa = dense::Builder::new()
            .configure(Config::new().anchored(!sub))
            .build(pattern.as_ref())
            .unwrap();

        // let u = dfa.union(sub);

        let mut stream = set.search(&dfa).into_stream();

        collect_stream(stream)?
    } else {
        let lev = Levenshtein::new(q, d)?;
        let pre = lev.starts_with(); // also matches strings starting with q when d=0

        let mut stream = set.search(&pre).into_stream();

        collect_stream(stream)?
    };

    Ok(keys)
}
