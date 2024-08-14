#![allow(unused_variables)]

use std::{path::PathBuf, str::FromStr};

use clap::{Parser, Subcommand};
use zk_por_cli::{prover::prove, verifier::verify};
use zk_por_core::error::ProofError;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: ZkPorCommitCommands,
}

pub trait Execute {
    fn execute(&self) -> std::result::Result<(), ProofError>;
}

#[derive(Subcommand)]
pub enum ZkPorCommitCommands {
    Prove {
        #[arg(short, long)]
        cfg_path: String, // path to config file
        #[arg(short, long)]
        output_path: String, // path to output file
    },
    GetMerkleProof {
        #[arg(short, long)]
        user_id: String,
    },
    Verify {
        #[arg(short, long)]
        global_proof_path: String,
        #[arg(short, long)]
        inclusion_proof_path: Option<String>,
    },
}

impl Execute for ZkPorCommitCommands {
    fn execute(&self) -> std::result::Result<(), ProofError> {
        match self {
            ZkPorCommitCommands::Prove { cfg_path, output_path } => {
                let cfg = zk_por_core::config::ProverConfig::load(&cfg_path)
                    .map_err(|e| ProofError::FileReadError)?;
                let prover_cfg = cfg.try_deserialize().unwrap();
                let output_file = PathBuf::from_str(&output_path).unwrap();
                prove(prover_cfg, output_file)
            }
            ZkPorCommitCommands::GetMerkleProof { user_id } => todo!(),
            ZkPorCommitCommands::Verify { global_proof_path, inclusion_proof_path } => {
                let global_proof_path = PathBuf::from_str(&global_proof_path).unwrap();
                let inclusion_proof_path =
                    inclusion_proof_path.clone().map(|p| PathBuf::from_str(&p).unwrap());
                verify(global_proof_path, inclusion_proof_path)
            }
        }
    }
}

fn main() -> std::result::Result<(), ProofError> {
    let cli = Cli::parse();
    let start = std::time::Instant::now();
    let result = cli.command.execute();
    println!("result: {:?}, elapsed: {:?}", result, start.elapsed());
    Ok(())
}