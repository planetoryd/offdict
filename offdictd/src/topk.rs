use anyhow::{anyhow, Result};
use strprox::{MetaAutocompleter, TreeStringT};
use yoke::{Yoke, Yokeable};

use crate::*;   
use crate::{candidates, Indexer};

pub struct Strprox {
    pub yoke: Yoke<MetaAutocompleter<'static>, Mmap>,
}

#[derive(new)]
pub struct TopkParam {
    num: usize,
}

impl Indexer for Strprox {
    const FILE_NAME: &'static str = "strprox";
    type Param = TopkParam;
    fn build_all(words: impl IntoIterator<Item = String>, pp: &std::path::Path) -> Result<()> {
        let arr: Vec<_> = words
            .into_iter()
            .map(|k| TreeStringT::from_owned(k))
            .collect();
        let set = MetaAutocompleter::new(arr.len(), arr);
        let mut fw = std::fs::File::create(pp)?;
        bincode::serialize_into(&mut fw, &set)?;
        Ok(())
    }
    fn load_file(pp: &std::path::Path) -> Result<Self> {
        println!("loading index from {:?}", pp);
        let f = std::fs::File::open(pp)?;
        let sel = Self {
            yoke: Yoke::try_attach_to_cart(unsafe { Mmap::map(&f) }?, |data| {
                bincode::deserialize(data)
            })?,
        };
        println!("index loaded");
        Ok(sel)
    }
    #[timed]
    fn query(&self, query: &str, param: TopkParam) -> Result<crate::candidates> {
        let topk = self.yoke.get();
        let rx = topk.threshold_topk(query, param.num, 2);
        let cands: Vec<_> = rx.into_iter().map(|k| k.string).collect();
        Ok(cands)
    }
    fn count(&self) -> usize {
        self.yoke.get().len()
    }
}
