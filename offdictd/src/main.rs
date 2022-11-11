use ciborium::{de::from_reader, ser::into_writer};

use clap::{Parser, Subcommand};


use percent_encoding;
use tokio;
use warp::Filter;

use offdictd::*;

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
        arg_required_else_help = true
    )]
    yaml {
        #[arg(short = 'p')]
        path: String,
    },
    #[command(about = "Stats")]
    stat {},
    #[command(about = "Rebuild fuzzytrie from rocksdb")]
    trie {},
    #[command(about = "Fuzzy query (prefix)")]
    lookup { query: String },
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Cli::parse();
    let db_path = "rocks_t";
    let trie_path = "./trie";
    
    let db = open_db(&db_path);
    let trie_buf: &'static mut Vec<u8> = Box::leak(Box::new(Vec::new()));
    let mut trie = FuzzyTrie::new(2, true);

    let yaml_defs: &'static mut Vec<Def> = Box::leak(Box::new(Vec::new()));

    match args.command {
        Some(Commands::yaml { path }) => {
            // Read yaml, put word defs in rocks, build a trie words -> words

            let file = File::open(path).expect("Unable to open file");
            *yaml_defs = serde_yaml::from_reader(file)?;

            trie = load_trie(trie_path, trie_buf);
            import_defs(yaml_defs, &db, &mut trie);

            // match db.get(yaml_defs[0].word.as_ref().unwrap()) {
            //     Ok(Some(value)) => bson::from_slice::<Def>(value.as_slice()),
            //     Ok(None) => println!(""),
            //     Err(e) => println!("operational problem encountered: {}", e),
            // }
            // db.delete(yaml_defs[0].word.as_ref().unwrap().as_str())
            //     .unwrap();

            // let _ = DB::destroy(&Options::default(), path);

            // println!("{}", serde_yaml::to_string(&docs)?);

            save_trie(trie_path, &trie);
            println!("imported");
        }
        Some(Commands::stat {}) => {
            trie = load_trie(trie_path, trie_buf);

            if let Ok(Some(r)) = db.property_int_value("rocksdb.estimate-num-keys") {
                println!("Words: {}", r);
            }
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

