use std::{
    borrow::{Borrow, BorrowMut},
    path::PathBuf,
    sync::{Arc, RwLock},
};

use tokio::{self};

use lazy_static;
use offdictd::{def_bin::WrapperDef, *};
use topk::Strprox;

fn main() -> Result<()> {
    let conf = crate::config::get_config();
    println!("config: {:?}", &conf);
    let db_path = PathBuf::from(conf.data_path.clone());

    newix(db_path)
}

fn newix(db_path: PathBuf) -> Result<()> {
    process_cmd(|| init_db(db_path.clone()))?;
    let db = init_db(db_path)?;
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async move {
        if let Some(ref set) = db.set {
            tokio::try_join!(serve(db), repl(db))?;
        } else {
            println!("Run offdictd build to initialize the index");
        }
        anyhow::Ok(())
    })?;
    Ok(())
}
