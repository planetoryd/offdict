#[derive(Clone, Debug)]
/// Config for FuzzyTrie
/// 
/// It is used to provide different
/// Levenshtein configs depending on the length
pub struct Config {
    /// Default config for searches that 
    /// do not match other cases
    pub default: LevenshteinConfig,
    /// Pairs of (`config`, `len`)
    /// `len` is the max length of key in chars that `config` will be applied to.
    /// 
    /// If the explanation is chaotic
    /// then see the `choose_dfa_builder` method on `FuzzyTrie`
    /// for additional information
    pub other: Vec<(LevenshteinConfig, usize)>,
}

/// Config for Levenstein automata
#[derive(Copy, Clone, Debug)]
pub struct LevenshteinConfig {
    /// Max distance
    pub distance: u8,
    /// Indicates whether it Damerau–Levenshtein or not
    pub damerau: bool,
}