use crate::Node;
use bimap::BiBTreeMap;
use timed::timed;

/// Trick to make possible using a value field as key
pub struct Inserter<'a, T: Ord> {
    values: &'a mut BiBTreeMap<usize, T>,
    to: &'a mut Vec<Node>,
}

impl<'a, T: Ord> Inserter<'a, T> {
    /// Consumes self and insert a value
    #[inline]
    pub fn insert(self, value: T) {
        if let Some(l) = self.values.get_by_right(&value) {
            self.to.push(Node::new_value_index(*l));
        } else {
            let index = self.values.len();
            self.values.insert(index, value);
            self.to.push(Node::new_value_index(index));
        }
    }

    // #[inline]
    // pub fn insert_unique(self, value: T)
    // where
    //     T: PartialEq,
    // {
    //     if self.values.contains(&value) {
    //         return;
    //     }
    //     self.values.push(value);
    //     let index = self.values.len() - 1;
    //     self.to.push(Node::new_value_index(index));
    // }

    pub(crate) fn new(values: &'a mut BiBTreeMap<usize, T>, to: &'a mut Vec<Node>) -> Self {
        Self { values, to }
    }
}
