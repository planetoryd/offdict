use std::{
    borrow::{Borrow, BorrowMut},
    path::PathBuf,
    sync::{Arc, RwLock},
};

use tokio::{self};

use offdictd::{def_bin::WrapperDef, fst_index::fstmmap, *};
use topk::Strprox;

fn main() -> Result<()> {
    let conf = crate::config::get_config();
    println!("config: {:?}", &conf);

    let db_path = PathBuf::from(conf.data_path.clone());

    fstix(db_path)
}

fn fstix(db_path: PathBuf) -> Result<()> {
    static mut DB: Option<offdict<fstmmap>> = None;

    let mut db = offdict::<fstmmap>::open_db(db_path)?;
    process_cmd(&mut db)?;
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async move { tokio::try_join!(serve(&db), repl(&db)) })?;

    Ok(())
}
