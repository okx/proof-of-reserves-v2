use std::{
    io::{stdin, Read},
    path::PathBuf,
    str::FromStr,
};

#[cfg(feature = "async")]
use async_trait::async_trait;

use clap::{Parser, Subcommand};
use zk_por_cli::{
    checker::check_non_neg_user,
    constant::{DEFAULT_USER_PROOF_FILE_PATTERN, GLOBAL_PROOF_FILENAME},
    prover::prove,
    verifier::{verify_global, verify_user},
};
use zk_por_core::error::PoRError;

#[cfg(feature="cuda")]
use zeknox::init_cuda_degree_rs;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<ZkPorCommands>,
}

#[cfg(feature = "async")]
#[async_trait]
pub trait Execute {
    async fn execute(&self) -> std::result::Result<(), PoRError>;
}

#[cfg(not(feature = "async"))]
pub trait Execute {
    fn execute(&self) -> std::result::Result<(), PoRError>;
}

#[derive(Subcommand)]
pub enum ZkPorCommands {
    Prove {
        #[arg(short, long)]
        cfg_path: String, // path to config file
        #[arg(short, long)]
        output_path: String, // path to output file
    },
    CheckNonNegUser {
        #[arg(short, long)]
        cfg_path: String, // path to config file
    },

    VerifyGlobal {
        #[arg(short, long)]
        proof_path: String,
    },

    VerifyUser {
        #[arg(short, long)]
        global_proof_path: String,
        #[arg(short, long)]
        user_proof_path_pattern: String,
    },

    ShowCommitHash,
}

#[cfg(feature = "async")]
#[async_trait]
impl Execute for Option<ZkPorCommands> {
    async fn execute(&self) -> std::result::Result<(), PoRError> {
        match self {
            Some(ZkPorCommands::Prove { cfg_path, output_path }) => {
                let cfg = zk_por_core::config::ProverConfig::load(&cfg_path)
                    .map_err(|e| PoRError::ConfigError(e))?;
                let prover_cfg = cfg.try_deserialize().unwrap();
                let output_path = PathBuf::from_str(&output_path).unwrap();
                let _r = prove(prover_cfg, output_path).await.expect("Failed to prove");
                Ok(())
            }

            Some(ZkPorCommands::CheckNonNegUser { cfg_path }) => {
                let cfg = zk_por_core::config::ProverConfig::load(&cfg_path)
                    .map_err(|e| PoRError::ConfigError(e))?;
                let prover_cfg = cfg.try_deserialize().unwrap();
                check_non_neg_user(prover_cfg)
            }

            Some(ZkPorCommands::VerifyGlobal { proof_path: global_proof_path }) => {
                let global_proof_path = PathBuf::from_str(&global_proof_path).unwrap();
                verify_global(global_proof_path, true, true)
            }

            Some(ZkPorCommands::VerifyUser { global_proof_path, user_proof_path_pattern }) => {
                let global_proof_path = PathBuf::from_str(&global_proof_path).unwrap();
                verify_user(global_proof_path, user_proof_path_pattern, true)
            }

            Some(ZkPorCommands::ShowCommitHash) => {
                let commit_hash = option_env!("COMMIT_HASH").unwrap_or("n.a.");
                println!("\tCOMMIT_HASH: {}", commit_hash);
                Ok(())
            }

            None => {
                println!("============Validation started============");

                let exec_parent_path = std::env::current_exe()
                    .expect("fail to get current exe path")
                    .parent()
                    .unwrap()
                    .to_path_buf();

                // join the dir path and GLOBAL_PROOF_FILENAME
                let global_proof_path = exec_parent_path.join(GLOBAL_PROOF_FILENAME);

                let user_proof_path_pattern = exec_parent_path
                    .join(DEFAULT_USER_PROOF_FILE_PATTERN)
                    .to_str()
                    .unwrap()
                    .to_string();

                let global_result = verify_global(global_proof_path.clone(), false, false);
                let user_result = verify_user(global_proof_path, &user_proof_path_pattern, false);

                if global_result.is_ok() {
                    println!("Total sum and non-negative constraint validation passed");
                } else {
                    println!("Total sum and non-negative constraint validation failed");
                }

                if user_result.is_ok() {
                    println!("Inclusion constraint validation passed");
                } else {
                    println!("Inclusion constraint validation failed");
                }
                println!("============Validation finished============");

                if global_result.is_err() {
                    global_result
                } else if user_result.is_err() {
                    user_result
                } else {
                    Ok(())
                }
            }
        }
    }
}

#[cfg(not(feature = "async"))]
impl Execute for Option<ZkPorCommands> {
    fn execute(&self) -> std::result::Result<(), PoRError> {
        match self {
            Some(ZkPorCommands::Prove { cfg_path, output_path }) => {
                let cfg = zk_por_core::config::ProverConfig::load(&cfg_path)
                    .map_err(|e| PoRError::ConfigError(e))?;
                let prover_cfg = cfg.try_deserialize().unwrap();
                let output_path = PathBuf::from_str(&output_path).unwrap();
                prove(prover_cfg, output_path)
            }

            Some(ZkPorCommands::CheckNonNegUser { cfg_path }) => {
                let cfg = zk_por_core::config::ProverConfig::load(&cfg_path)
                    .map_err(|e| PoRError::ConfigError(e))?;
                let prover_cfg = cfg.try_deserialize().unwrap();
                check_non_neg_user(prover_cfg)
            }

            Some(ZkPorCommands::VerifyGlobal { proof_path: global_proof_path }) => {
                let global_proof_path = PathBuf::from_str(&global_proof_path).unwrap();
                verify_global(global_proof_path, true, true)
            }

            Some(ZkPorCommands::VerifyUser { global_proof_path, user_proof_path_pattern }) => {
                let global_proof_path = PathBuf::from_str(&global_proof_path).unwrap();
                verify_user(global_proof_path, user_proof_path_pattern, true)
            }

            Some(ZkPorCommands::ShowCommitHash) => {
                let commit_hash = option_env!("COMMIT_HASH").unwrap_or("n.a.");
                println!("\tCOMMIT_HASH: {}", commit_hash);
                Ok(())
            }

            None => {
                println!("============Validation started============");

                let exec_parent_path = std::env::current_exe()
                    .expect("fail to get current exe path")
                    .parent()
                    .unwrap()
                    .to_path_buf();

                // join the dir path and GLOBAL_PROOF_FILENAME
                let global_proof_path = exec_parent_path.join(GLOBAL_PROOF_FILENAME);

                let user_proof_path_pattern = exec_parent_path
                    .join(DEFAULT_USER_PROOF_FILE_PATTERN)
                    .to_str()
                    .unwrap()
                    .to_string();

                let global_result = verify_global(global_proof_path.clone(), false, false);
                let user_result = verify_user(global_proof_path, &user_proof_path_pattern, false);

                if global_result.is_ok() {
                    println!("Total sum and non-negative constraint validation passed");
                } else {
                    println!("Total sum and non-negative constraint validation failed");
                }

                if user_result.is_ok() {
                    println!("Inclusion constraint validation passed");
                } else {
                    println!("Inclusion constraint validation failed");
                }
                println!("============Validation finished============");

                if global_result.is_err() {
                    global_result
                } else if user_result.is_err() {
                    user_result
                } else {
                    Ok(())
                }
            }
        }
    }
}

#[tokio::main]
async fn main() {
    #[cfg(feature="cuda")]
    init_cuda_degree_rs(22);

    let cli = Cli::parse();
    let start = std::time::Instant::now();

    #[cfg(feature = "async")]
    let r = cli.command.execute().await.expect("Failed to execute command");

    #[cfg(not(feature = "async"))]
    let r = cli.command.execute();

    let duration = start.elapsed();
    println!("Execution result: {:?}, duration: {:?}", r, duration);

    let is_prove_command =
        matches!(cli.command, Some(ZkPorCommands::Prove { cfg_path: _, output_path: _ }));
    if !is_prove_command {
        println!("Press Enter to quit...");
        stdin().read_exact(&mut [0]).unwrap();
    }
}
