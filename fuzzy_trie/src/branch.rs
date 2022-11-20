use bimap::BiBTreeMap;

use crate::Collector;
use crate::Inserter;
use levenshtein_automata::{Distance, DFA, SINK_STATE};

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub(crate) enum Node {
    // leaf and sub nodes
    Branch(char, Vec<Node>),
    ValueIndex(usize),
}

impl Node {
    pub(crate) const fn new_branch(leaf: char) -> Self {
        Self::Branch(leaf, Vec::new())
    }

    pub(crate) const fn new_value_index(index: usize) -> Self {
        Self::ValueIndex(index)
    }

    /// Find node with a given leaf index
    fn find_node(branch: &mut Vec<Node>, leaf: char) -> Option<usize> {
        for (i, node) in branch.iter_mut().enumerate() {
            if let Self::Branch(l, _) = node {
                if *l == leaf {
                    return Some(i);
                }
            }
        }
        None
    }

    fn next_node(&mut self, leaf: char) -> &mut Self {
        let branch = match self {
            Self::Branch(_, br) => br,
            Self::ValueIndex(_) => unreachable!(),
        };
        match Self::find_node(branch, leaf) {
            Some(node_index) => &mut branch[node_index],
            None => Self::insert_branch(branch, leaf),
        }
    }

    fn insert_branch(branch: &mut Vec<Node>, leaf: char) -> &mut Self {
        branch.push(Self::new_branch(leaf));
        branch.last_mut().unwrap()
    }

    fn insert_value<'a, T: Ord>(
        &'a mut self,
        values: &'a mut BiBTreeMap<usize, T>,
    ) -> Inserter<'a, T> {
        let branch = match self {
            Self::Branch(_, br) => br,
            Self::ValueIndex(_) => unreachable!(),
        };
        Inserter::new(values, branch)
    }

    pub(crate) fn insert<'a, T: Ord>(
        &'a mut self,
        values: &'a mut BiBTreeMap<usize, T>,
        key: &str,
    ) -> Inserter<'a, T> {
        let mut node = self;
        for c in key.chars() {
            node = node.next_node(c);
        }
        node.insert_value(values)
    }

    pub(crate) fn fuzzy_search<'a, T: Ord>(
        &'a self,
        values: &'a BiBTreeMap<usize, T>,
        dfa: &DFA,
        state: u32,
        out: &mut impl Collector,
    ) -> bool {
        let (leaf, branch) = match self {
            Self::Branch(leaf, branch) => (leaf, branch),
            Self::ValueIndex(i) => {
                match dfa.distance(state) {
                    Distance::Exact(d) => {
                        out.push(d, *i);
                        if d == 0 {
                            return true;
                        }
                    }
                    Distance::AtLeast(_) => (),
                };
                return false;
            }
        };
        let mut new_state = state;
        let mut char_buf = [0; 4];
        for b in leaf.encode_utf8(&mut char_buf).bytes() {
            new_state = dfa.transition(new_state, b);
        }
        if new_state == SINK_STATE {
            return false;
        }
        for node in branch {
            if node.fuzzy_search(values, dfa, new_state, out) {
                return true;
            }
        }
        return false;
    }
}
