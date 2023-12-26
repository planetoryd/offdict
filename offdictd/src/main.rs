use std::{
    borrow::{Borrow, BorrowMut},
    path::PathBuf,
    sync::{Arc, RwLock},
};

use tokio::{self};

use offdictd::{def_bin::WrapperDef, fst_index::fstmmap, *};

fn main() -> Result<()> {
    let conf = crate::config::get_config();

    let mut _db_path = PathBuf::from(conf.data_path.clone());
    let db_path = _db_path.to_str().unwrap();

    let db = Arc::new(RwLock::new(offdict::<fstmmap>::open_db(db_path.to_owned())));

    println!("config: {:?}", &conf);
    let _db_a = db.clone();
    tui(db.write().unwrap().borrow_mut()).unwrap();
    let rt = tokio::runtime::Runtime::new().unwrap();

    rt.block_on(async {
        tokio::try_join!(serve(db.clone()), repl(db.clone()))
    })?;

    Ok(())
}

// fn api_lookup(res:Vec<Def>)
