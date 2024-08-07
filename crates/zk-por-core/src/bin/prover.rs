use zk_por_core::{
    account::Account, circuit_registry::registry::CircuitRegistry, config::ProverConfig,
    parser::read_json_into_accounts_vec,
};

use plonky2::{hash::hash_types::HashOut, plonk::proof::ProofWithPublicInputs};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{fs::File, io::Write, path::PathBuf, sync::mpsc, thread};
use zk_por_core::{
    circuit_config::{STANDARD_CONFIG, STANDARD_ZK_CONFIG},
    e2e::stream_prove,
    types::{C, D, F},
};

#[derive(Serialize, Deserialize)]
struct Proof {
    round_num: u32,
    root_vd_digest: HashOut<F>,
    proof: ProofWithPublicInputs<F, C, D>,
}

fn main() {
    let cfg = ProverConfig::try_new().unwrap();
    // let trace_cfg: TraceConfig = cfg.log.into();
    // init_tracing(trace_cfg);
    const RECURSION_FACTOR: usize = 64;
    const PROVING_THREADS_NUM: usize = 4;
    if cfg.prover.hyper_tree_size as usize != RECURSION_FACTOR {
        panic!("The hyper_tree_size is not configured to be equal to 64 (Recursion_Factor)");
    }
    let batch_size = cfg.prover.batch_size as usize;
    let asset_num = 4; // TODO: read from config

    // TODO: read path from args
    let account_paths = vec![PathBuf::from("../../test-data/batch0.json")];
    // the path to dump the final generated proof
    let proof_path = PathBuf::from("../../test-data/proof.json");

    // Hardcode three levels of recursive circuits, each branching out 64 children, with the last level with zk enabled.
    // Hence given batch_size=1024, the current setting can support 268M (1024*64^3) accounts, enough for the foreseeable future. (Currently we have 10M accounts)
    let recursive_circuit_configs = vec![STANDARD_CONFIG, STANDARD_CONFIG, STANDARD_ZK_CONFIG];

    let circuit_registry = CircuitRegistry::<RECURSION_FACTOR>::init(
        batch_size,
        asset_num,
        STANDARD_CONFIG,
        recursive_circuit_configs,
    );
    let root_circuit_digest = circuit_registry.get_root_circuit().verifier_only.circuit_digest;

    let (tx, rx) = mpsc::channel::<Vec<Account>>();
    let prover =
        thread::spawn(move || stream_prove(rx, &circuit_registry, batch_size, PROVING_THREADS_NUM));
    thread::spawn(move || {
        for (file_idx, account_path) in account_paths.iter().enumerate() {
            let accounts = read_json_into_accounts_vec(account_path.to_str().unwrap());
            tx.send(accounts).unwrap()
        }
    });

    let root_proof = prover.join().unwrap();
    let proof = Proof {
        round_num: cfg.prover.round_no,
        root_vd_digest: root_circuit_digest,
        proof: root_proof,
    };

    let mut file = File::create(proof_path.clone())
        .expect(format!("fail to create proof file at {:#?}", proof_path).as_str());
    file.write_all(json!(proof).to_string().as_bytes()).expect("fail to write proof to file");
}