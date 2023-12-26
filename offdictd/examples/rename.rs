use anyhow::Result;
use std::{
    collections::HashMap,
    fs::{read_dir, rename},
    os::unix::ffi::OsStrExt,
};

fn main() -> Result<()> {
    let map: HashMap<_, _> = HashMap::from_iter([
        (868, "汉语大词典(简体精排)"),
        (786, "21世纪英汉大词典"),
        (888, "柯林斯COBUILD高阶英汉双解学习词典"),
        (857, "简明英汉汉英词典"),
    ]);
    let mut m2 = HashMap::new();
    for dir in read_dir("/home/noume/dicts")? {
        let en = dir?;
        let na = en.file_name();
        let by = na.as_bytes()[1..6]
            .into_iter()
            .map(|k| *k as u64)
            .reduce(|a, e| a + e)
            .unwrap();
        let st = en.file_name().to_string_lossy().into_owned();
        println!("{:?}, {:?}", en.file_name().to_string_lossy(), by);
        let mut ma = st.match_indices(".yaml");
        let k = ma.next().unwrap();
        let num = &st[k.0 - 2..k.0].parse::<u32>();
        let num = num.to_owned().unwrap_or(st[k.0 - 1..k.0].parse::<u32>()?);
        if let Some(val) = map.get(&by) {
            let name = format!("{}.{}.yaml", val, num);
            println!("{}", name);
            let mut pa2 = en.path();
            pa2.set_file_name(name);
            m2.insert(en.path(), pa2);
        }
    }
    for (x, y) in m2 {
        rename(x, y)?;
    }
    Ok(())
}
