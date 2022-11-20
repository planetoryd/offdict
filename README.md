## Offline dictionary with open formats

Totally offline performant dictionary, built with rocksdb and fuzzy-trie.

- A format I camp up with deliberately when cleaning up some dictionary files
- CLI is provided at minimum
    - [x] API server
    - [x] REPL
    - [x] GUI (tauri)
- Performant: Rocksdb for word-defition mapping, and fuzzy-trie is saved to disk

I personally have some Chinese-English dictionary source files. I cleaned up the data into open formats, and this program is specifically for that.

API: 
- `127.0.0.1:3030/q/some_word_to_lookup`
- `/stat/`

Source files that work with it: https://github.com/planetoryd/OpenMdicts

Import them with `offdict yaml -p 'path/OpenMdicts/<name>.yaml'`

- [x] GUI: Clipboard watcher
- [x] GUI: Import dictionaries from a folder of yaml
- [ ] Export dictionaries
- [ ] Decentralized sharing format ? IPLD ?
    - a protocol on revising dictionaries ?
- [ ] Better serialization for rocksdb and fuzzy trie (currently cbor) ? 

```sh
apt install libxcb-shape0-dev libxcb-xfixes0-dev # required for building clipboard-master
```

## Usage

- Input anywhere to start live search
- Press ⬆️ or ⬇️ for different words
- Press ⬅️ or ➡️ for scrolling
- It loads `config.json5` in the working directory
    - In the directory as specified by `data_path`, file `trie` and folder `rocks_t` should exist or will be created. 
- To import dictionaries
    1. Copy existing `trie` and `rocks_t` to the working directory
    2. Use GUI
    3. Use CLI