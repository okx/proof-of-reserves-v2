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
    verifier::{print_circuit_verifier_hex, verify_global, verify_user},
};
use zk_por_core::error::PoRError;

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
    CheckNonNegUser {
        #[arg(short, long)]
        cfg_path: String, // path to config file
    },
    PrintRootCircuitVerifier {
        #[arg(short, long)]
        proof_path: String,
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

            Some(ZkPorCommitCommands::CheckNonNegUser { cfg_path }) => {
                let cfg = zk_por_core::config::ProverConfig::load(&cfg_path)
                    .map_err(|e| PoRError::ConfigError(e))?;
                let prover_cfg = cfg.try_deserialize().unwrap();
                check_non_neg_user(prover_cfg)
            }

            Some(ZkPorCommitCommands::PrintRootCircuitVerifier { proof_path }) => {
                let global_proof_path = PathBuf::from_str(&proof_path).unwrap();
                print_circuit_verifier_hex(global_proof_path)
            }

            Some(ZkPorCommitCommands::VerifyGlobal { proof_path: global_proof_path }) => {
                let global_proof_path = PathBuf::from_str(&global_proof_path).unwrap();
                verify_global(global_proof_path, true, true)
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
    let cli = Cli::parse();
    let r = cli.command.execute();
    let is_prove_command =
        matches!(cli.command, Some(ZkPorCommitCommands::Prove { cfg_path: _, output_path: _ }));
    if is_prove_command {
        println!("Execution result: {:?}", r);
    } else {
        println!("Execution result: {:?}. Press Enter to quit...", r);
        stdin().read_exact(&mut [0]).unwrap();
    }
}
