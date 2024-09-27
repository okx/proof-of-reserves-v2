use std::{path::PathBuf, str::FromStr};

use clap::{Parser, Subcommand};
use zk_por_cli::{
    constant::{DEFAULT_USER_PROOF_FILE_PATTERN, GLOBAL_PROOF_FILENAME},
    prover::prove,
    verifier::{verify_global, verify_user},
};
use zk_por_core::error::PoRError;

#[cfg(feature="cuda")]
use cryptography_cuda::init_cuda_degree_rs;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<ZkPorCommitCommands>,
}

pub trait Execute {
    fn execute(&self) -> std::result::Result<(), PoRError>;
}

#[derive(Subcommand)]
pub enum ZkPorCommitCommands {
    Prove {
        #[arg(short, long)]
        cfg_path: String, // path to config file
        #[arg(short, long)]
        output_path: String, // path to output file
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
}

impl Execute for Option<ZkPorCommitCommands> {
    fn execute(&self) -> std::result::Result<(), PoRError> {
        match self {
            Some(ZkPorCommitCommands::Prove { cfg_path, output_path }) => {
                let cfg = zk_por_core::config::ProverConfig::load(&cfg_path)
                    .map_err(|e| PoRError::ConfigError(e))?;
                let prover_cfg = cfg.try_deserialize().unwrap();
                let output_path = PathBuf::from_str(&output_path).unwrap();
                prove(prover_cfg, output_path)
            }

            Some(ZkPorCommitCommands::VerifyGlobal { proof_path: global_proof_path }) => {
                let global_proof_path = PathBuf::from_str(&global_proof_path).unwrap();
                verify_global(global_proof_path, true)
            }

            Some(ZkPorCommitCommands::VerifyUser {
                global_proof_path,
                user_proof_path_pattern,
            }) => {
                let global_proof_path = PathBuf::from_str(&global_proof_path).unwrap();
                verify_user(global_proof_path, user_proof_path_pattern, true)
            }

            None => {
                println!("============Validation started============");
                let global_proof_path = PathBuf::from_str(GLOBAL_PROOF_FILENAME).unwrap();
                let user_proof_path_pattern = DEFAULT_USER_PROOF_FILE_PATTERN.to_owned();
                if verify_global(global_proof_path.clone(), false).is_ok() {
                    println!("Total sum and non-negative constraint validation passed")
                } else {
                    println!("Total sum and non-negative constraint validation failed")
                }

                if verify_user(global_proof_path, &user_proof_path_pattern, false).is_ok() {
                    println!("Inclusion constraint validation passed")
                } else {
                    println!("Inclusion constraint validation failed")
                }
                println!("============Validation finished============");
                Ok(())
            }
        }
    }
}

fn main() -> std::result::Result<(), PoRError> {
    #[cfg(feature="cuda")]
    init_cuda_degree_rs(22);

    let cli = Cli::parse();
    let r = cli.command.execute();
    println!("Execution result: {:?}", r);
    Ok(())
}
