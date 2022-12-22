use std::{
    borrow::Borrow,
    cell::RefCell,
    path::PathBuf,
    sync::{Arc, RwLock},
};

// use ciborium::de::from_reader;

use clap::{Parser, Subcommand};

use percent_encoding;
use tokio::{self};
use warp::Filter;

use config::{Config, File, FileFormat, Map};
use offdictd::{def_bin::WrapperDef, *};

use glob;

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

#[allow(non_camel_case_types)]
#[derive(Debug, Subcommand)]
enum Commands {
    #[command(
        about = "Import definitions from an yaml file",
        arg_required_else_help = false
    )]
    yaml {
        #[arg(short = 'p', required = true)]
        path: String,
        #[arg(short = 'c', long)]
        check: bool,
        #[arg(short = 's', long)]
        save: bool,
    },
    #[command(about = "Stats")]
    stat {},
    #[command(about = "Fuzzy query (prefix)")]
    lookup {
        query: String,
    },
    // TODO: bincode import
    // #[command(about = "Convert an yaml file to cbor")]
    // cbor {
    //     // Converts a yaml to cbor and save it.
    //     #[arg(short = 'p')]
    //     path: String,
    //     #[arg(short = 'n')]
    //     name: String, // Name to be displayed
    // },
    reset {},
    #[command(about = "Build index, required after adding or removing words")]
    build {},
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
    let db_path = _db_path.to_str().unwrap();

    let db = Arc::new(RwLock::new(offdict::open_db(db_path.to_owned())));

    println!("config: {:?}", &conf);
    let _db_a = db.clone();
    {
        let mut db_w = db.write().unwrap();
        match args.command {
            Some(Commands::yaml { path, check, save }) => {
                if check {
                    let options = glob::MatchOptions {
                        case_sensitive: false,
                        ..Default::default()
                    };

                    for entry in glob::glob_with(&path, options)? {
                        let entr = entry?;
                        println!("checking {}", entr.to_str().unwrap());
                        Def::check_yaml(entr.to_str().unwrap(), save);
                    }

                    return Ok(());
                } else {
                    match db_w.import_glob(&path) {
                        Ok(()) => println!("imported"),
                        Err(e) => println!("{:?}", e),
                    }
                }
            }
            Some(Commands::stat {}) => {
                let s = db_w.stat();
                println!("Words in database: {}", s.words);
            }
            Some(Commands::lookup { query }) => {
                for d in db_w.search(&query, 1, true) {
                    let list: Vec<Def> = d.vec_human();
                    println!("{}", serde_yaml::to_string::<Vec<Def>>(&list)?)
                }
            }
            Some(Commands::reset {}) => {
                db_w.reset_db();
                println!("reset.");
            }
            Some(Commands::build {}) => {
                let c = db_w.build_fst_from_db();
                println!("built, {} words", c);
            }
            None => {}
        };
    }
    let rt = tokio::runtime::Runtime::new().unwrap();

    rt.block_on(async {
        let db_tok = db.clone();

        let lookup = warp::get()
            .and(warp::path("q"))
            .and(warp::path::param::<String>())
            .map(move |word: String| {
                let db_r = db_tok.read().unwrap();
                let word = percent_encoding::percent_decode_str(&word)
                    .decode_utf8()
                    .unwrap()
                    .to_string();
                warp::reply::json(&api_q(&db_r, &word))
            });

        let stat = warp::get()
            .and(warp::path("stat"))
            .map(|| warp::reply::json(&Stat { words: 0 }));

        tokio::join!(
            warp::serve(lookup.or(stat)).run(([127, 0, 0, 1], 3030)),
            repl(db.clone())
        );
    });

    Ok(())
}

#[derive(Serialize, Deserialize)]
struct Stat {
    words: u64,
}

async fn repl(db_: Arc<RwLock<offdict>>) {
    loop {
        let li = readline().await.unwrap();
        let li = li.trim();
        if li.is_empty() {
            continue;
        } else {
            let db = db_.read().unwrap();

            match respond(li, db.borrow()) {
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
}

fn respond(line: &str, db: &offdict) -> Result<bool, String> {
    let mut arr = db.search(line, 2, true);

    println!("{} results", arr.len());
    arr.truncate(2);
    for d in arr.into_iter() {
        println!(
            "{}",
            serde_yaml::to_string::<Vec<Def>>(&d.vec_human()).unwrap()
        );
    }

    Ok(false)
}

fn api_q(db: &offdict, query: &str) -> Map<String, Vec<Def>> {
    println!("\nq: {}", query);
    let arr = db.search(&query, 5, true);
    let mut m = Map::new();
    for wr in arr {
        m.insert(wr.word.clone(), wr.vec_human());
    }

    m
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
