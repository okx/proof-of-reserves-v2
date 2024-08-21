use std::{path::PathBuf, str::FromStr};

use clap::{Parser, Subcommand};
use zk_por_cli::{prover::prove, verifier::verify_global, verifier::verify_user};
use zk_por_core::error::PoRError;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: ZkPorCommitCommands,
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

impl Execute for ZkPorCommitCommands {
    fn execute(&self) -> std::result::Result<(), PoRError> {
        match self {
            ZkPorCommitCommands::Prove { cfg_path, output_path } => {
                let cfg = zk_por_core::config::ProverConfig::load(&cfg_path)
                    .map_err(|e| PoRError::ConfigError(e))?;
                let prover_cfg = cfg.try_deserialize().unwrap();
                let output_file = PathBuf::from_str(&output_path).unwrap();
                prove(prover_cfg, output_file)
            }

            ZkPorCommitCommands::VerifyGlobal { proof_path: global_proof_path,  } => {
                let global_proof_path = PathBuf::from_str(&global_proof_path).unwrap();
                verify_global(global_proof_path)
            }

            ZkPorCommitCommands::VerifyUser { global_proof_path, user_proof_path_pattern } => {
                let global_proof_path = PathBuf::from_str(&global_proof_path).unwrap();
                verify_user(global_proof_path, user_proof_path_pattern)
            }
        }
    }
}

fn main() -> std::result::Result<(), PoRError> {
    let cli = Cli::parse();
    let start = std::time::Instant::now();
    let result = cli.command.execute();
    println!("result: {:?}, elapsed: {:?}", result, start.elapsed());
    Ok(())
}
