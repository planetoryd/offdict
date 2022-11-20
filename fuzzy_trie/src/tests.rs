use crate::Config;
use crate::FuzzyTrie;
use crate::LevenshteinConfig;
use std::fmt::Debug;

fn test_search<'a, T>(trie: &FuzzyTrie<T>, key: &str, expected_values: Vec<T>)
where
    T: Copy + Ord + Debug,
{
    let mut values: Vec<(u8, usize)> = Vec::new();
    trie.prefix_fuzzy_search(key, &mut values);
    let mut r = values
        .iter()
        .map(|(d, i)| *trie.values.get_by_left(i).unwrap())
        .collect::<Vec<T>>();
    r.sort();
    assert_eq!(r, expected_values);
}

fn test_search_d<'a, T>(trie: &FuzzyTrie<T>, key: &str)
where
    T: Copy + Ord + Debug,
{
    let mut res: Vec<(u8, usize)> = Vec::new();
    trie.prefix_fuzzy_search(key, &mut res);
    println!("{:?}", res)
    // assert_eq!(values, expected_values);
}

// #[test]
// fn test() {
//     let mut index = FuzzyTrie::new(2, false);
//     let s = "жфг";
//     index.insert(s).insert(234);
//     let mut v: Vec<usize> = Vec::new();
//     index.fuzzy_search(s, &mut v);
//     assert_eq!(Some(234), v.into_iter().next());
// }

// #[test]
// fn test_custom_config() {
//     let default = LevenshteinConfig {
//         distance: 2,
//         damerau: false,
//     };
//     let len4 = LevenshteinConfig {
//         distance: 1,
//         damerau: false,
//     };
//     let len3 = LevenshteinConfig {
//         distance: 0,
//         damerau: false,
//     };
//     let config = Config {
//         default,
//         other: vec![(len3, 3), (len4, 4)],
//     };
//     let mut index = FuzzyTrie::new_with_config(&config);

//     index.insert("key").insert(10);
//     index.insert("kry").insert(15);
//     index.insert("tick").insert(20);
//     index.insert("tack").insert(25);
//     index.insert("takk").insert(30);
//     index.insert("hello").insert(35);
//     index.insert("hollo").insert(40);
//     index.insert("holmo").insert(45);
//     index.insert("hommo").insert(50);

// }

#[test]
fn test_prefix() {
    let mut trie = FuzzyTrie::new(2, false);
    trie.insert("something").insert(1);
    trie.insert("something").insert(2);
    trie.insert("something else").insert(3);
    trie.insert("somewhere").insert(4);
    trie.insert("some time").insert(5);
    trie.insert("sometimes").insert(6);

    test_search(&trie, "s0me", vec![1, 2, 3, 4, 5, 6]);
    test_search(&trie, "s0ma", vec![1, 2, 3, 4, 5, 6]);
    test_search(&trie, "s0meth", vec![1, 2, 3, 4, 6]);
    test_search(&trie, "s0methin", vec![1, 2, 3]);
    test_search(&trie, "somatime", vec![5, 6]);
    test_search(&trie, "something wrong", vec![]);

    test_search_d(&trie, "something ");
}

#[test]
fn test_prefix_with_custom_config() {
    let default = LevenshteinConfig {
        distance: 2,
        damerau: false,
    };
    let len4 = LevenshteinConfig {
        distance: 1,
        damerau: false,
    };
    let len3 = LevenshteinConfig {
        distance: 0,
        damerau: false,
    };
    let config = Config {
        default,
        other: vec![(len3, 3), (len4, 4)],
    };
    let mut trie = FuzzyTrie::new_with_config(&config);

    trie.insert("something").insert(1);
    trie.insert("something").insert(2);
    trie.insert("something else").insert(3);
    trie.insert("somewhere").insert(4);
    trie.insert("some time").insert(5);
    trie.insert("sometimes").insert(6);

    test_search(&trie, "s0me", vec![1, 2, 3, 4, 5, 6]);
    test_search(&trie, "s0ma", vec![]);
    test_search(&trie, "s0m", vec![]);
    test_search(&trie, "s0meth", vec![1, 2, 3, 4, 6]);
    test_search(&trie, "s0methin", vec![1, 2, 3]);
    test_search(&trie, "somatime", vec![5, 6]);
    test_search(&trie, "something wrong", vec![]);
}
