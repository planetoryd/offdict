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
        #[arg(short = 'n', long)]
        name: Option<String>, // Name to be displayed
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

    let mut yaml_defs: Vec<Def> = vec![];

    println!("config: {:?}", &conf);
    let _db_a = db.clone();
    {
        let mut db_w = db.write().unwrap();
        match args.command {
            Some(Commands::yaml {
                path,
                name,
                check,
                save,
            }) => {
                if check {
                    Def::check_yaml(&path, save);
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

                match db_w.import_from_file(&path, &name1) {
                    Ok(()) => println!("imported"),
                    Err(e) => println!("{:?}", e),
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
    let arr = db.search(line, 1, true);

    println!("{} results", arr.len());
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
