use config::{Config, File};
use serde_derive::Deserialize;
use std::error::Error;

#[derive(Deserialize, Clone)]
pub struct BotConfig {
    pub http_port: i32,
    pub block_engine_urls: Vec<String>,
    pub proxy: Vec<String>,
}

pub fn load_config() -> Result<BotConfig, Box<dyn Error>> {
    let settings = Config::builder()
        .add_source(File::with_name("config"))
        .build()?;

    let config: BotConfig = settings.try_deserialize()?;

    Ok(config)
}
