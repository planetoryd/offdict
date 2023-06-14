/// Collector for searches
/// 
/// When trie makes a search it walks over the nodes
/// and puts values that match the given key into collector
pub trait Collector<'a, T> {
    /// Called when trie found a value that matches the key
    /// 
    /// distance - Levenshtein distance from
    /// key used to insert value to search key
    fn push(&mut self, distance: u8, value: &'a T);
}


impl<'a, T> Collector<'a, T> for Vec<(u8, &'a T)> {
    #[inline]
    fn push(&mut self, distance: u8, value: &'a T) {
        self.push((distance, value));
    }
}


impl<'a, T> Collector<'a, T> for Vec<&'a T> {
    #[inline]
    fn push(&mut self, _distance: u8, value: &'a T) {
        self.push(value);
    }
}


impl<'a, T: Copy> Collector<'a, T> for Vec<T> {
    #[inline]
    fn push(&mut self, _distance: u8, value: &'a T) {
        self.push(*value);
    }
}


impl<'a, T: Copy> Collector<'a, T> for Vec<(u8, T)> {
    #[inline]
    fn push(&mut self, distance: u8, value: &'a T) {
        self.push((distance, *value));
    }
}