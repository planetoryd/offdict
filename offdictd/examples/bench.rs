use std::{
    borrow::{Borrow, BorrowMut},
    path::PathBuf,
    sync::{Arc, RwLock},
};

use regex::Regex;
use tokio::{self};

use offdictd::{def_bin::WrapperDef, fst_index::fstmmap, topk::TopkParam, *};
use topk::Strprox;

const REG: &str = r"(?<query>[^+]+)\s*([+](?<num>[0-9]+))?";

#[tokio::main]
async fn main() -> Result<()> {
    let conf = crate::config::get_config();
    let db_path = PathBuf::from(conf.data_path.clone());
    let fst = fstmmap::load_file(&fstmmap::path(&db_path))?;
    let strp = Strprox::load_file(&Strprox::path(&db_path))?;
    let lineparam = Regex::new(REG)?;
    loop {
        let li = readline().await.unwrap();
        let li = li.trim();
        if li.is_empty() {
            continue;
        } else {
            if let Some(caps) = lineparam.captures(li) {
                if let Some(q) = caps.name("query") {
                    let q = q.as_str().trim();
                    let n: usize = if let Some(n) = caps.name("num") {
                        n.as_str().parse()?
                    } else {
                        1
                    };
                    println!("fst: {:?}", fst.query(q, false)?);
                    println!("meta: {:?}", strp.query(q, TopkParam::new(n))?);
                }
            }
        }
    }

    Ok(())
}

#[test]
fn reg() -> Result<()> {
    let lineparam = Regex::new(REG)?;
    let c1 = lineparam.captures("word");
    dbg!(&c1);
    dbg!(c1.unwrap().name("num"));
    dbg!(lineparam.captures("word +4"));
    dbg!(lineparam.captures("word next word +4"));
    dbg!(lineparam.captures("word next word  +4"));
    Ok(())
}
