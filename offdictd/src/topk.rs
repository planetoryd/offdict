use anyhow::{anyhow, Result};
use strprox::{Autocompleter, TreeStringT};

use crate::*;
use crate::{candidates, Indexer};

pub struct Strprox {
    pub file: Mmap,
}

#[derive(new)]
pub struct TopkParam {
    num: usize,
}

impl Indexer for Strprox {
    const FILE_NAME: &'static str = "strprox";
    type Param = TopkParam;
    type Brw = Autocompleter<'static>;
    fn build_all(words: impl IntoIterator<Item = String>, pp: &std::path::Path) -> Result<()> {
        let arr: Vec<_> = words
            .into_iter()
            .map(|k| TreeStringT::from_owned(k))
            .collect();
        let set = Autocompleter::new(arr.len(), arr);
        let mut fw = std::fs::File::create(pp)?;
        bincode::serialize_into(&mut fw, &set)?;
        Ok(())
    }
    fn load_file(pp: &std::path::Path) -> Result<Self> {
        let f = std::fs::File::open(pp)?;
        let sel = Self {
            file: unsafe { Mmap::map(&f) }?,
        };
        Ok(sel)
    }
    fn query(&self, query: &str, param: TopkParam, brw: &Self::Brw) -> Result<crate::candidates> {
        let topk = brw;
        let rx = topk.autocomplete(query, param.num);
        let cands: Vec<_> = rx.into_iter().map(|k| k.string).collect();
        Ok(cands)
    }
}

// impl Init for Strprox {
//     fn init(&'static mut self) -> Result<()> {
//         self.set = Some(bincode::deserialize(&self.file)?);
//         Ok(())
//     }
// }
