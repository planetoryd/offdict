use crate::Node;


/// Trick to make possible using a value field as key
pub struct Inserter<'a, T> {
    values: &'a mut Vec<T>,
    to: &'a mut Vec<Node>,
}


impl<'a, T> Inserter<'a, T> {
    /// Consumes self and insert a value
    #[inline]
    pub fn insert(self, value: T) {
        self.values.push(value);
        let index = self.values.len() - 1;
        self.to.push(Node::new_value_index(index));
    }

    pub(crate) fn new(values: &'a mut Vec<T>, to: &'a mut Vec<Node>) -> Self {
        Self{values, to}
    }
}