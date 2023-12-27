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

    let db_path = PathBuf::from(conf.data_path.clone());
    let mut db = offdict::<Strprox>::open_db(db_path)?;
    println!("config: {:?}", &conf);

    process_cmd(&mut db)?;
    unsafe {
        DB = Some(db);
    }
    let db = unsafe { DB.as_mut() }.unwrap();
    let rt = tokio::runtime::Runtime::new().unwrap();

    rt.block_on(async move {
        db.set_brw = Some(bincode::deserialize(&db.set.as_ref().unwrap().file)?);
        tokio::try_join!(serve(db), repl(db))
    })?;

    Ok(())
}

pub type IxTy = Strprox;

static mut DB: Option<offdict<IxTy>> = None;

// fn api_lookup(res:Vec<Def>)
