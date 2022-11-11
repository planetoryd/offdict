## Offline dictionary with open formats

Totally offline performant dictionary, built with rocksdb and fuzzy-trie.

- A format I camp up with deliberately when cleaning up some dictionary files
- CLI is provided at minimum
    - [x] API server
    - [x] REPL
    - [ ] GUI (Native)
- Performant: Rocksdb for word-defition mapping, and fuzzy-trie is saved to disk

I personally have some Chinese-English dictionary source files. I cleaned up the data into open formats, and this program is specifically for that.

API: 
- `127.0.0.1:3030/q/some_word_to_lookup`
- `/stat/`

Source files that work with it: https://github.com/planetoryd/OpenMdicts

Import them with `offdict yaml -p 'path/OpenMdicts/<name>.yaml'`

### Build

- `cargo build --release`
