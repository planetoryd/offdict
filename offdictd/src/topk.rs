use strprox::{Autocompleter, TreeStringT};

use crate::*;
use crate::{candidates, Indexer};

struct Strprox<'de> {
    file: Mmap,
    set: Option<Autocompleter<'de>>,
}

impl<'de> Indexer<'de> for Strprox<'de> {
    const FILE_NAME: &'static str = "strprox";
    fn build_all(
        words: impl IntoIterator<Item = String>,
        pp: &std::path::Path,
    ) -> anyhow::Result<()> {
        let arr: Vec<_> = words
            .into_iter()
            .map(|k| TreeStringT::from_owned(k))
            .collect();
        let set = Autocompleter::new(arr.len(), arr);
        let mut fw = std::fs::File::open(pp)?;
        bincode::serialize_into(&mut fw, &set)?;
        Ok(())
    }
    fn load_file(pp: &std::path::Path) -> Result<Self> {
        let f = std::fs::File::open(pp)?;
        let sel = Self {
            file: unsafe { Mmap::map(&f) }?,
            set: None,
        };
        Ok(sel)
    }
    fn query(&self, query: &str, expensive: bool) -> anyhow::Result<crate::candidates> {
        Ok(candidates::default())
    }
    fn init(&'de mut self) -> Result<()> {
        self.set = Some(bincode::deserialize(&self.file)?);
        Ok(())
    }
}
