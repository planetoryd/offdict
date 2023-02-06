## Offline dictionary with open formats

Totally offline performant dictionary, with prefix fuzzy autocompleting search, live, and clipboard watching (lookup by selection on Linux).

- Linux only, currently

<img src="./img/screenshot.png" width="50%">
<img src="./img/screenshot-dark.png" width="50%">

- A format I camp up with deliberately when cleaning up some dictionary files
    - [x] API server
    - [x] REPL
    - [x] GUI (tauri)

I personally have some Chinese-English dictionary source files. I cleaned up the data into open formats, and this program is specifically for that.

API: 
- `127.0.0.1:3030/q/some_word_to_lookup`
- `/stat/`

```sh
apt install libxcb-shape0-dev libxcb-xfixes0-dev # required for building clipboard-master
```

## Usage

- Input anywhere to start live search.
- Copy anything and it would pop up.
- Press ⬅️ or ➡️ for scrolling
- Tap `Enter` to perform an extensive & expensive search
- It loads `config.json5` in the working directory. 
- To import dictionaries
    1. Just grab a `data` folder from somewhere (might have vulnerability)
    2. or, import with `./offdict yaml -p '*.yaml'` (supports wildcard `*`)
    3. Then build the index `./offdict build`

## notes 

- faster fuzzy autocompletion algorithm. https://github.com/qinbill/IncNgTrie ?

## Known issues

- IBus seems to write into the PRIMARY of X11 (when I am using Wayland), which was intended to be user selection
- Wayland can't set window on top https://github.com/tauri-apps/tauri/issues/3117 You have to set it manually (temporarily or permanently through system settings)
- Wayland global shortcut https://github.com/tauri-apps/tauri/issues/3578
- [ ] Clipboard should be monitored separate from primary. Sometimes `primary` is not respected. We have to fall back to clipboard, explicit copying.
- IME doesnt work in the input on Wayland 