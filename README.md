# Key-value collection to make fuzzy searches

FuzzyTrie is a trie with a LevensteinAutomata to make fuzzy searches

## Example

```rust
use fuzzy_trie::FuzzyTrie;

let mut trie = FuzzyTrie::new(2, false);
trie.insert("vanilla").insert("vanilla item");
trie.insert("hello").insert("hello item");
trie.insert("helo").insert("helo item");
trie.insert("vanllo").insert("vanllo item");


let mut hello = Vec::new();
trie.fuzzy_search("hello", &mut hello);
let mut hello_iter = hello.into_iter();

assert_eq!(hello_iter.next(), Some((0, &"hello item")));
assert_eq!(hello_iter.next(), Some((1, &"helo item")));
assert_eq!(hello_iter.next(), None);


let mut vanila = Vec::new();
trie.fuzzy_search("vanilla", &mut vanila);
let mut vanila_iter = vanila.into_iter();

assert_eq!(vanila_iter.next(), Some((0, &"vanilla item")));
assert_eq!(vanila_iter.next(), Some((2, &"vanllo item")));
assert_eq!(vanila_iter.next(), None);
```

**Some more examples are in `tests.rs`**
