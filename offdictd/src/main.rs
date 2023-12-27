use std::{
    borrow::{Borrow, BorrowMut},
    path::PathBuf,
    sync::{Arc, RwLock},
};

use tokio::{self};

use offdictd::{def_bin::WrapperDef, *};
use topk::Strprox;

fn main() -> Result<()> {
    let conf = crate::config::get_config();
    println!("config: {:?}", &conf);

    let db_path = PathBuf::from(conf.data_path.clone());

    newix(db_path)
}

fn newix(db_path: PathBuf) -> Result<()> {
    static mut DB: Option<offdict<Strprox>> = None;

    let mut db = offdict::<Strprox>::open_db(db_path)?;

    process_cmd(&mut db)?;
    unsafe {
        DB = Some(db);
    }
    let db = unsafe { DB.as_mut() }.unwrap();
    let rt = tokio::runtime::Runtime::new().unwrap();

    rt.block_on(async move {
        if let Some(ref set) = db.set {
            db.set_brw = Some(bincode::deserialize(&set.file)?);
            tokio::try_join!(serve(db), repl(db))?;
        } else {
            println!("Run offdictd build to initialize the index");
        }
        anyhow::Ok(())
    })?;

    Ok(())
}
