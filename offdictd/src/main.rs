use std::path::PathBuf;

use ciborium::de::from_reader;

use clap::{Parser, Subcommand};

use percent_encoding;
use tokio;
use warp::Filter;

use config::{Config, File, FileFormat};
use offdictd::*;

#[derive(Debug, Deserialize)]
struct OffdictConfig {
    data_path: String,
}

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(about = "Offline dictionary", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Debug, Subcommand)]
enum Commands {
    #[command(
        about = "Import definitions from an yaml file to rocksdb and fuzzytrie",
        arg_required_else_help = false
    )]
    yaml {
        #[arg(short = 'p', required = true)]
        path: String,
        #[arg(short = 'n')]
        name: Option<String>, // Name to be displayed
        #[arg(short = 'c')]
        check: bool,
        #[arg(short = 's')]
        save: bool,
    },
    #[command(about = "Stats")]
    stat {},
    #[command(about = "Rebuild fuzzytrie from rocksdb")]
    trie {},
    #[command(about = "Fuzzy query (prefix)")]
    lookup { query: String },
    // #[command(about = "Convert an yaml file to cbor")]
    // cbor {
    //     // Converts a yaml to cbor and save it.
    //     #[arg(short = 'p')]
    //     path: String,
    //     #[arg(short = 'n')]
    //     name: String, // Name to be displayed
    // },
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Cli::parse();

    let config = Config::builder()
        .set_default("data_path", ".")
        .unwrap()
        .add_source(File::new("config", FileFormat::Json5))
        .build()
        .unwrap();

    let conf: OffdictConfig = config.try_deserialize().unwrap();

    let mut _db_path = PathBuf::from(conf.data_path.clone());
    _db_path.push("rocks_t");
    let db_path = _db_path.to_str().unwrap();
    let mut _trie_path = PathBuf::from(conf.data_path.clone());
    _trie_path.push("./trie");
    let trie_path = _trie_path.to_str().unwrap();

    let db = open_db(&db_path);
    let trie_buf: &'static mut Vec<u8> = Box::leak(Box::new(Vec::new()));
    let mut trie = FuzzyTrie::new(2, true);

    let yaml_defs: &'static mut Vec<Def> = Box::leak(Box::new(Vec::new()));

    println!("config: {:?}", &conf);
    match args.command {
        Some(Commands::yaml {
            path,
            name,
            check,
            save,
        }) => {
            // Read yaml, put word defs in rocks, build a trie words -> words
            if check {
                check_yaml(&path, save);
                return Ok(());
            }
            let pa = PathBuf::from(&path);
            let s = pa.file_stem().unwrap().to_str().unwrap().split_once(".");
            let name1;

            if name.is_none() {
                if s.is_none() {
                    println!("provide a name");
                    return Ok(());
                } else {
                    name1 = name.unwrap_or(s.unwrap().0.to_owned());
                }
            } else {
                name1 = name.unwrap()
            }

            let r = import_yaml(
                &db, &mut trie, trie_path, trie_buf, &path, yaml_defs, &name1,
            );
            match r {
                Ok(()) => println!("imported"),
                Err(e) => println!("{:?}", e),
            }
        }
        // Some(Commands::cbor { path, name }) => {
        //     let file = File::open(&path).expect("Unable to open file");
        //     *yaml_defs = serde_yaml::from_reader(file)?;
        //     // *yaml_defs = ciborium::de::from_reader(file)?;

        //     for def in yaml_defs.iter_mut() {
        //         (*def).dictName = Some(name.clone());
        //     }

        //     let wf = File::create(path.replace(".yaml", ".cbor"))?;
        //     ciborium::ser::into_writer(yaml_defs, wf)?;

        //     println!("written to cbor")
        //     // trie = load_trie(trie_path, trie_buf);
        //     // import_defs(cbor_defs, &db, &mut trie);

        // }
        // No we dont need cbor. just gzip
        Some(Commands::stat {}) => {
            trie = load_trie(trie_path, trie_buf);

            // if let Ok(Some(r)) = db.property_int_value("rocksdb.estimate-num-keys") {
            //     println!("Words: {}", r);
            // }
            // That's not accurate.

            println!("Words in database: {}", stat_db(&db));
            println!("Words in trie: {}", trie.into_values().len());
        }
        Some(Commands::trie {}) => {
            trie = FuzzyTrie::new(2, true);
            rebuild_trie(&db, &mut trie);
            save_trie(&trie_path, &trie);
            println!("trie rebuilt");
        }
        Some(Commands::lookup { query }) => {
            trie = load_trie(trie_path, trie_buf);

            let mut key: Vec<(u8, &String)> = Vec::new();

            trie.prefix_fuzzy_search(&query, &mut key); // Values of, keys that are close, are returned
            let mut arr: Vec<(u8, &String)> = key.into_iter().collect();
            arr.sort_by_key(|x| x.0);
            let arr2 = arr[..min(3, arr.len())].iter();

            println!("{:?}", arr2);

            for d in db.multi_get(arr2.map(|(_d, str)| str)) {
                if let Ok(Some(by)) = d {
                    // println!("{:?}", bson::from_slice::<Def>(by.as_slice()).unwrap())
                    println!(
                        "{}",
                        serde_yaml::to_string::<Def>(&from_reader::<Def, &[u8]>(&by)?)?
                    )
                }
            }
        }
        None => {
            trie = load_trie(trie_path, trie_buf);
        }
    };

    let db: &'static DB = Box::leak(Box::new(db));
    let trie: &'static FuzzyTrie<String> = Box::leak(Box::new(trie));

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let lookup = warp::get()
            .and(warp::path("q"))
            .and(warp::path::param::<String>())
            .map(|word: String| {
                let word = percent_encoding::percent_decode_str(&word)
                    .decode_utf8()
                    .unwrap()
                    .to_string();
                warp::reply::json(&api_q(db, trie, &word))
            });

        let stat = warp::get().and(warp::path("stat")).map(|| {
            warp::reply::json(&stat {
                words_rocks: db
                    .property_int_value("rocksdb.estimate-num-keys")
                    .unwrap()
                    .unwrap(),
                words_trie: trie.into_values().len(),
            })
        });

        tokio::join!(
            warp::serve(lookup.or(stat)).run(([127, 0, 0, 1], 3030)),
            repl(db, trie)
        );
    });

    Ok(())
}

#[derive(Serialize, Deserialize)]
struct stat {
    words_trie: usize,
    words_rocks: u64,
}

async fn repl(db: &DB, trie: &FuzzyTrie<'_, String>) {
    loop {
        let li = readline().await.unwrap();
        let li = li.trim();
        if li.is_empty() {
            continue;
        }

        match respond(li, &db, &trie) {
            Ok(quit) => {
                if quit {
                    break;
                }
            }
            Err(err) => {
                write!(std::io::stdout(), "{}", err)
                    .map_err(|e| e.to_string())
                    .unwrap();
                std::io::stdout()
                    .flush()
                    .map_err(|e| e.to_string())
                    .unwrap();
            }
        }
    }
}

fn respond(
    line: &str,
    db: &DB,
    // trie_bf: &Vec<u8>,
    trie: &FuzzyTrie<String>,
) -> Result<bool, String> {
    let arr = search(&db, &trie, line);

    for d in arr.into_iter().map(|mut x| x.cli_pretty()) {
        println!("{}", d);
    }

    Ok(false)
}

fn api_q(
    db: &DB,
    // trie_bf: &Vec<u8>,
    trie: &FuzzyTrie<String>,
    query: &str,
) -> Vec<Def> {
    println!("\nq: {}", query);
    let arr = search(&db, &trie, query);

    arr
}

// fn api_lookup(res:Vec<Def>)

async fn readline() -> Result<String, Box<dyn Error>> {
    let mut out = tokio::io::stdout();
    out.write_all(b"@ ").await?;
    out.flush().await?;
    let mut buffer = Vec::new();
    tokio::io::stdin().read(&mut buffer).await?;
    let stdin = tokio::io::stdin();
    let reader = tokio::io::BufReader::new(stdin);
    let mut lines = tokio::io::AsyncBufReadExt::lines(reader);
    Ok(lines.next_line().await?.unwrap())
}
