use std::str::FromStr;

use config::{Config, ConfigError, File};
use serde::Deserialize;
use tracing::Level;
use zk_por_tracing::TraceConfig;

#[derive(Debug, Clone, Deserialize)]
pub struct ConfigLog {
    pub file_name_prefix: String,
    pub dir: String,
    pub level: String,
    pub flame: bool,
    pub console: bool,
}

impl From<ConfigLog> for TraceConfig {
    fn from(log_cfg: ConfigLog) -> Self {
        TraceConfig {
            prefix: log_cfg.file_name_prefix,
            dir: log_cfg.dir,
            level: Level::from_str(&log_cfg.level).unwrap(),
            console: log_cfg.console,
            flame: log_cfg.flame,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct ConfigProver {
    pub round_no: usize,
    pub batch_size: usize,
    pub tokens: Vec<String>,
    pub user_data_path: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ConfigDb {
    pub level_db_user_path: String,
    pub level_db_gmst_path: String,
}

impl ConfigDb {
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
