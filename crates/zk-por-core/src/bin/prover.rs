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

// fn main() {
//     let cfg = ProverConfig::try_new().unwrap();
//     // let trace_cfg: TraceConfig = cfg.log.into();
//     // init_tracing(trace_cfg);
//     const RECURSION_FACTOR: usize = 64;
//     const PROVING_THREADS_NUM: usize = 4;
//     if cfg.prover.hyper_tree_size as usize != RECURSION_FACTOR {
//         panic!("The hyper_tree_size is not configured to be equal to 64 (Recursion_Factor)");
//     }
//     let batch_size = cfg.prover.batch_size as usize;
//     let asset_num = 4; // TODO: read from config

//     // TODO: read path from args
//     let account_paths = vec![PathBuf::from("../../test-data/batch0.json")];
//     // the path to dump the final generated proof
//     let proof_path = PathBuf::from("../../test-data/proof.json");

//     // Hardcode three levels of recursive circuits, each branching out 64 children, with the last level with zk enabled.
//     // Hence given batch_size=1024, the current setting can support 268M (1024*64^3) accounts, enough for the foreseeable future. (Currently we have 10M accounts)
//     let recursive_circuit_configs = vec![STANDARD_CONFIG, STANDARD_CONFIG, STANDARD_ZK_CONFIG];
//     let recursive_levels = recursive_circuit_configs.len();

//     let circuit_registry = CircuitRegistry::<RECURSION_FACTOR>::init(
//         batch_size,
//         asset_num,
//         STANDARD_CONFIG,
//         recursive_circuit_configs,
//     );

//     let mut batch_proofs: Vec<ProofWithPublicInputs<F, C, D>> = Vec::new();
//     let (batch_circuit, account_targets) = circuit_registry.get_batch_circuit();

//     for (file_idx, account_path) in account_paths.iter().enumerate() {
//         let accounts = read_json_into_accounts_vec(account_path.to_str().unwrap());
//         let num_accounts = accounts.len();

//         // split accounts into vector of batch_size
//         let mut account_batches: Vec<Vec<Account>> = accounts
//             .into_iter()
//             .chunks(batch_size)
//             .into_iter()
//             .map(|chunk| chunk.collect())
//             .collect();

//         tracing::info!(
//             "Number of accounts {}, number of batches {}, file_idx {}, file_path {}",
//             account_batches.len(),
//             num_accounts,
//             file_idx,
//             account_path.to_str().unwrap()
//         );
//         if let Some(last_batch) = account_batches.last_mut() {
//             let last_batch_size = last_batch.len();

//             // fill the last batch with empty accounts so that it is of size batch_size
//             let empty_accounts = gen_empty_accounts(batch_size - last_batch_size, asset_num); // TODO: to be consistent with the one used for building merkle tree.
//             last_batch.extend(empty_accounts.into_iter());
//         } else {
//             panic!("No account batches found in the file {}", account_path.to_str().unwrap());
//         }

//         // split account_batches into chunks of size PROVING_THREADS_NUM and then parallelize the proving in each chunk.
//         account_batches
//             .into_iter()
//             .chunks(PROVING_THREADS_NUM)
//             .into_iter()
//             .map(|chunk| chunk.collect())
//             .for_each(|chunk: Vec<Vec<Account>>| {
//                 let proofs: Vec<ProofWithPublicInputs<F, C, D>> = chunk
//                     .into_par_iter()
//                     .map(|accounts| {
//                         let prover = MerkleSumTreeProver { accounts };
//                         let proof = prover
//                             .get_proof_with_circuit_data(account_targets.clone(), &batch_circuit);
//                         proof
//                         // TODO: parse tree node from proof and check against the one generated by merkle sum tree.
//                     })
//                     .collect();
//                 batch_proofs.extend(proofs.into_iter());
//                 tracing::info!("finish proving {} batches", batch_proofs.len());
//             });
//     }
//     let mut last_level_circuit_vd = batch_circuit.verifier_only.clone();
//     let mut last_level_proofs = batch_proofs;

//     for level in 0..recursive_levels {
//         let last_level_vd_digest = last_level_circuit_vd.circuit_digest;
//         let last_level_empty_proof =
//             circuit_registry.get_empty_proof(&last_level_vd_digest).expect(
//                 format!(
//                     "fail to find empty proof at recursive level {} with inner circuit vd {:?}",
//                     level, last_level_vd_digest
//                 )
//                 .as_str(),
//             );

//         let (recursive_circuit, recursive_targets) = circuit_registry
//             .get_recursive_circuit(&last_level_vd_digest)
//             .expect(format!("No recursive circuit found for level {}", level).as_str());
//         let subproof_len = last_level_proofs.len();

//         let mut subproof_batches: Vec<Vec<ProofWithPublicInputs<F, C, D>>> = last_level_proofs
//             .into_iter()
//             .chunks(RECURSION_FACTOR)
//             .into_iter()
//             .map(|chunk| chunk.collect())
//             .collect();

//         tracing::info!(
//             "Recursive Level {}, number of subproofs {}, number of batches {}",
//             level,
//             subproof_len,
//             subproof_batches.len()
//         );
//         if let Some(last_batch) = subproof_batches.last_mut() {
//             let last_batch_size = last_batch.len();

//             // fill the last batch with empty subproofs so that it is of size RECURSION_FACTOR
//             let empty_proofs =
//                 vec![last_level_empty_proof.clone(); RECURSION_FACTOR - last_batch_size];
//             last_batch.extend(empty_proofs.into_iter());
//         } else {
//             panic!("No last proof batches found in the level {}", level);
//         }

//         let mut this_level_proofs = vec![];

//         subproof_batches
//             .into_iter()
//             .chunks(PROVING_THREADS_NUM)
//             .into_iter()
//             .map(|chunk| chunk.collect())
//             .for_each(|chunk: Vec<Vec<ProofWithPublicInputs<F, C, D>>>| {
//                 let proofs: Vec<ProofWithPublicInputs<F, C, D>> = chunk
//                     .into_par_iter()
//                     .map(|subproofs| {
//                         let sub_proofs: [ProofWithPublicInputs<F, C, D>; RECURSION_FACTOR] =
//                             subproofs
//                                 .try_into()
//                                 .expect("subproofs length not equal to RECURSION_FACTOR");
//                         let recursive_prover = RecursiveProver {
//                             sub_proofs,
//                             sub_circuit_vd: last_level_circuit_vd.clone(),
//                         };
//                         let proof = recursive_prover.get_proof_with_circuit_data(
//                             recursive_targets.clone(),
//                             &recursive_circuit,
//                         );
//                         // TODO: consider valid proof and parse the tree node from the proof and check against the one generated by merkle sum tree.
//                         proof
//                     })
//                     .collect();
//                 this_level_proofs.extend(proofs.into_iter());
//                 tracing::info!(
//                     "finish proving {} subproofs at level {}",
//                     this_level_proofs.len(),
//                     level
//                 );
//             });

//         last_level_circuit_vd = recursive_circuit.verifier_only.clone();
//         last_level_proofs = this_level_proofs;
//     }

//     if last_level_proofs.len() != 1 {
//         panic!("The last level proofs should be of length 1, but got {}", last_level_proofs.len());
//     }

// }
