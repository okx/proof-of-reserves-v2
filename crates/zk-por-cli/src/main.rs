use std::{
    io::{stdin, Read},
    path::PathBuf,
    str::FromStr,
};

use clap::{Parser, Subcommand};
use zk_por_cli::{
    checker::check_non_neg_user,
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
    command: Option<ZkPorCommands>,
}

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
                let mut result = Ok(());
                if verify_global(global_proof_path.clone(), false, false).is_ok() {
                    println!("Total sum and non-negative constraint validation passed");
                } else {
                    println!("Total sum and non-negative constraint validation failed");
                    result = Err(PoRError::InvalidProof);
                }

                if verify_user(global_proof_path, &user_proof_path_pattern, false).is_ok() {
                    println!("Inclusion constraint validation passed");
                } else {
                    println!("Inclusion constraint validation failed");
                    result = Err(PoRError::InvalidProof);
                }
                println!("============Validation finished============");
                result
            }
        }
    }
}

fn main() {
    #[cfg(feature="cuda")]
    init_cuda_degree_rs(22);

    let cli = Cli::parse();
    let start = std::time::Instant::now();
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
