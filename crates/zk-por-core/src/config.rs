use config::{Config, ConfigError, File};
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct ConfigLog {
    pub file_name_prefix: String,
    pub dir: String,
    pub level: String,
    pub flame: bool,
    pub console: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ConfigProver {
    pub round_no: u32,
    pub batch_size: u32,
    pub hyper_tree_size: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ConfigDb {
    pub level_db_path: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProverConfig {
    pub log: ConfigLog,
    pub prover: ConfigProver,
    pub db: ConfigDb,
}

impl ProverConfig {
    pub fn load(dir: &str) -> Result<Config, ConfigError> {
        let env = std::env::var("ENV").unwrap_or("default".into());
        Config::builder()
            // .add_source(File::with_name(&format!("{}/default", dir)))
            .add_source(File::with_name(&format!("{}/{}", dir, env)).required(false))
            .add_source(File::with_name(&format!("{}/local", dir)).required(false))
            .add_source(config::Environment::with_prefix("ZKPOR"))
            .build()
    }
    pub fn try_new() -> Result<Self, ConfigError> {
        let config = Self::load("config")?;
        config.try_deserialize()
    }
}
