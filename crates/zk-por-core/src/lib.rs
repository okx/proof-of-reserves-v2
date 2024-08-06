pub mod account;
pub mod circuit_config;
pub mod circuit_registry;
pub mod circuit_utils;
pub mod config;
pub mod error;
pub mod merkle_sum_prover;
pub mod merkle_sum_tree;
pub mod parser;
pub mod recursive_prover;
pub mod types;
pub mod util;

use account::Account;
use once_cell::sync::OnceCell;
use plonky2::hash::hash_types::HashOut;
use std::sync::RwLock;
use types::F;

#[derive(Debug)]
pub struct GlobalConfig {
    pub batch_size: usize,
    pub doc_num: usize,
    pub num_of_batches_per_doc: usize,
    pub total_num_of_users: usize,
    pub num_of_tokens: usize,
    pub num_of_batches: usize,
}

pub static EMPTY_ACCT: OnceCell<Account> = OnceCell::new();
pub static GLOBAL_CONFIG: OnceCell<GlobalConfig> = OnceCell::new();
pub static GLOBAL_MST: OnceCell<RwLock<Vec<HashOut<F>>>> = OnceCell::new();
