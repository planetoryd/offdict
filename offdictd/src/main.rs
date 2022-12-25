use std::{
    borrow::{Borrow, BorrowMut},
    cell::RefCell,
    path::PathBuf,
    sync::{Arc, RwLock},
};

// use ciborium::de::from_reader;

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



fn main() -> Result<(), Box<dyn Error>> {
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
    tui(db.write().unwrap().borrow_mut()).unwrap();
    let rt = tokio::runtime::Runtime::new().unwrap();

    rt.block_on(async {


        tokio::join!(
            serve(db.clone()),
            repl(db.clone())
        );
    });

    Ok(())
}





// fn api_lookup(res:Vec<Def>)

