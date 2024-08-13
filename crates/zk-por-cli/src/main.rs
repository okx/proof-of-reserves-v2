#![allow(unused_variables)]

use clap::{Parser, Subcommand};
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
    },
    GetMerkleProof {
        #[arg(short, long)]
        user_id: String,
    },
    VerifyProof {
        #[arg(short, long)]
        final_proof: String,
        #[arg(short, long)]
        merkle_proof: u64,
        #[arg(short, long)]
        user_id: String,
        #[arg(short, long)]
        skip_circit_verify: bool,
        #[arg(short, long)]
        circuit_vd: String,
    },
}

impl Execute for ZkPorCommitCommands {
    fn execute(&self) -> std::result::Result<(), ProofError> {
        match self {
            ZkPorCommitCommands::Prove { cfg_path } => todo!(),
            ZkPorCommitCommands::GetMerkleProof { user_id } => todo!(),
            ZkPorCommitCommands::VerifyProof {
                final_proof,
                merkle_proof,
                user_id,
                skip_circit_verify,
                circuit_vd,
            } => todo!(),
        }
    }
}

fn main() -> std::result::Result<(), ProofError> {
    let cli = Cli::parse();
    let start = std::time::Instant::now();
    let _ = cli.command.execute();
    println!("elapsed: {:?}", start.elapsed());
    Ok(())
}
