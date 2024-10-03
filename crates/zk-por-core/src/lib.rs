use plonky2::plonk::{circuit_data::CircuitConfig, proof::ProofWithPublicInputs};
use plonky2::hash::hash_types::HashOut;
use serde::*;
use types::{C, D, F};

pub mod account;
pub mod circuit_config;
pub mod circuit_registry;
pub mod circuit_utils;
pub mod config;
pub mod database;
pub mod e2e;
pub mod error;
pub mod global;
pub mod merkle_proof;
pub mod merkle_sum_prover;
pub mod merkle_sum_tree;
pub mod parser;
pub mod recursive_prover;
pub mod types;
pub mod util;

#[derive(Serialize, Deserialize)]
pub struct General {
    pub round_num: usize,
    pub recursion_branchout_num: usize,
    pub batch_size: usize,
    pub token_num: usize,
}

#[derive(Serialize, Deserialize)]
pub struct Proof {
    pub general: General,
    pub root_vd_digest: HashOut<F>,
    #[serde(default)] // some proof files may not have this field. 
    pub circuit_configs: Option<CircuitConfigs>,
    pub proof: ProofWithPublicInputs<F, C, D>,
}

#[derive(Serialize, Deserialize)]
pub struct CircuitConfigs {
    pub batch_circuit_config: CircuitConfig,
    pub recursive_circuit_configs: Vec<CircuitConfig>,
}

#[derive(Serialize, Deserialize)]
pub struct Info {
    pub root_hash: String,
    pub equity_sum: u64,
    pub debt_sum: u64,
    pub balance_sum: u64,
}
