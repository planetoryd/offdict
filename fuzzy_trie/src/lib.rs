#![warn(clippy::missing_inline_in_public_items)]
#![warn(clippy::missing_const_for_fn)]
#![warn(missing_docs)]

//! Key-value collection to make fuzzy searches

mod branch;
mod collector;
mod config;
mod inserter;
#[cfg(test)]
mod tests;

use bimap::BiBTreeMap;
use branch::Node;
pub use collector::Collector;
pub use config::*;
pub use inserter::Inserter;
use levenshtein_automata::LevenshteinAutomatonBuilder;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::slice::Iter;
use std::{collections::BTreeSet, marker};

/// FuzzyTrie is a trie with a LevensteinAutomata to make fuzzy searches
///
/// # Example
///
/// ```
/// use fuzzy_trie::FuzzyTrie;
///
/// let mut trie = FuzzyTrie::new(2, false);
/// trie.insert("vanilla").insert("vanilla item");
/// trie.insert("hello").insert("hello item");
/// trie.insert("helo").insert("helo item");
/// trie.insert("vanllo").insert("vanllo item");
///
///
/// let mut hello = Vec::new();
/// trie.fuzzy_search("hello", &mut hello);
/// let mut hello_iter = hello.into_iter();
///
/// assert_eq!(hello_iter.next(), Some((0, &"hello item")));
/// assert_eq!(hello_iter.next(), Some((1, &"helo item")));
/// assert_eq!(hello_iter.next(), None);
///
///
/// let mut vanila = Vec::new();
/// trie.fuzzy_search("vanilla", &mut vanila);
/// let mut vanila_iter = vanila.into_iter();
///
/// assert_eq!(vanila_iter.next(), Some((0, &"vanilla item")));
/// assert_eq!(vanila_iter.next(), Some((2, &"vanllo item")));
/// assert_eq!(vanila_iter.next(), None);
/// ```
///
///

#[derive(Serialize, Deserialize, Debug)]
pub struct FuzzyTrie<'b, T: 'b + Ord> {
    pub values: BiBTreeMap<usize, T>,
    root: Node,
    dfa_builders: Vec<(LevenshteinAutomatonBuilder, usize)>,
    default_dfa_builder: LevenshteinAutomatonBuilder,
    _marker: marker::PhantomData<&'b T>,
}

impl<'b, T: Ord> FuzzyTrie<'b, T> {
    /// Creates new fuzzy trie with
    /// given distance and dameru params
    #[inline]
    pub fn new(distance: u8, damerau: bool) -> Self {
        let default = LevenshteinConfig { distance, damerau };
        let config = Config {
            default,
            other: Vec::default(),
        };
        Self::new_with_config(&config)
    }

    /// Creates new fuzzy trie
    /// from given config
    #[inline]
    pub fn new_with_config(config: &Config) -> Self {
        let default_dfa_builder =
            LevenshteinAutomatonBuilder::new(config.default.distance, config.default.damerau);
        let mut dfa_builders: Vec<_> = config
            .other
            .iter()
            .map(|(cfg, len)| {
                (
                    LevenshteinAutomatonBuilder::new(cfg.distance, cfg.damerau),
                    *len,
                )
            })
            .collect();
        dfa_builders.sort_by_key(|(_, l)| *l);
        let values: BiBTreeMap<usize, T> = BiBTreeMap::new();
        let root = Node::new_branch('\0');
        Self {
            values,
            root,
            dfa_builders: dfa_builders,
            default_dfa_builder,
            _marker: marker::PhantomData,
        }
    }

    fn choose_dfa_builder(&self, len: usize) -> &LevenshteinAutomatonBuilder {
        for (builder, l) in self.dfa_builders.iter() {
            if len <= *l {
                return builder;
            }
        }
        return &self.default_dfa_builder;
    }

    /// Inserts value to trie
    /// Returns inserter, to make possible using the value field as a key
    /// See `Inserter` for additional information
    #[inline]
    pub fn insert<'a>(&'a mut self, key: &'a str) -> Inserter<'a, T> {
        self.root.insert(&mut self.values, key)
    }

    /// Makes fuzzy search with given key and puts result to out collector
    /// See `Collector` for additional information
    #[inline]
    pub fn fuzzy_search<'a, C: Collector>(&'a self, key: &'a str, out: &mut C) {
        let branches = match &self.root {
            Node::Branch(_, branches) => branches,
            _ => unreachable!(),
        };
        let dfa = self.choose_dfa_builder(key.chars().count()).build_dfa(key);
        for br in branches {
            br.fuzzy_search(&self.values, &dfa, dfa.initial_state(), out);
        }
    }

    /// Makes fuzzy search on prefix with given key and puts result to out collector
    /// See `Collector` for additional information
    #[inline]
    #[timed::timed]
    pub fn prefix_fuzzy_search<'a, C: Collector>(&'a self, key: &'a str, out: &mut C) {
        let branches = match &self.root {
            Node::Branch(_, branches) => branches,
            _ => unreachable!(),
        };
        let dfa = self
            .choose_dfa_builder(key.chars().count())
            .build_prefix_dfa(key);
        for br in branches {
            if br.fuzzy_search(&self.values, &dfa, dfa.initial_state(), out) {
                return;
            }
        }
    }

    /// Len of inner values vector
    #[inline]
    pub fn len(&self) -> usize {
        self.values.len()
    }
}
