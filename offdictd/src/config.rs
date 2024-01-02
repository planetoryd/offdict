use std::path::PathBuf;

use config::{Config, File, FileFormat};
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct OffdictConfig {
    pub data_path: PathBuf,
    pub hide_on_blur: bool,
}

pub fn get_config() -> OffdictConfig {
    let config = Config::builder()
        .set_default("data_path", ".")
        .unwrap()
        .set_default("hide_on_blur", false)
        .unwrap()
        .add_source(File::new("config", FileFormat::Json5).required(false))
        .build().unwrap();
    
    let conf: OffdictConfig = config.try_deserialize().unwrap();
    conf
}
