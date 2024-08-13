use plonky2::{hash::hash_types::HashOut, plonk::proof::ProofWithPublicInputs};
use rayon::{iter::ParallelIterator, prelude::*};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{env, fs::File, io::Write, path::PathBuf, str::FromStr, sync::RwLock};
use zk_por_core::{
    account::Account,
    circuit_config::{STANDARD_CONFIG, STANDARD_ZK_CONFIG},
    circuit_registry::registry::CircuitRegistry,
    config::ProverConfig,
    e2e::{batch_prove_accounts, prove_subproofs},
    global::{GlobalConfig, GlobalMst, GLOBAL_MST},
    merkle_sum_tree::MerkleSumTree,
    parser::{self, AccountParser, FilesCfg, FilesParser},
    types::{C, D, F},
};
use zk_por_tracing::{init_tracing, TraceConfig};

#[derive(Serialize, Deserialize)]
struct Proof {
    round_num: usize,
    root_vd_digest: HashOut<F>,
    proof: ProofWithPublicInputs<F, C, D>,
}

fn main() {
    let cfg = ProverConfig::try_new().unwrap();
    let trace_cfg: TraceConfig = cfg.log.into();
    let _g = init_tracing(trace_cfg);

    const RECURSION_BRANCHOUT_NUM: usize = 64;
    const BATCH_PROVING_THREADS_NUM: usize = 4;
    const RECURSIVE_PROVING_THREADS_NUM: usize = 2;

    if cfg.prover.recursion_branchout_num as usize != RECURSION_BRANCHOUT_NUM {
        panic!("The recursion_branchout_num is not configured to be equal to 64");
    }
    let batch_size = cfg.prover.batch_size as usize;
    let asset_num = cfg.prover.num_of_tokens as usize;

    // the path to dump the final generated proof
    let mut bench_mode = true;
    let args: Vec<String> = env::args().collect();
    let arg1 = args.get(1).expect(
        "Please provide the first argument, either proof path or '--bench' for benchmark mode",
    );
    let mut account_reader: Box<dyn AccountParser>;
    if arg1 == "--bench" {
        bench_mode = true;
        let account_num = args
            .get(2)
            .expect("Please provide the account number as the second argument in benchmark mode")
            .parse::<usize>()
            .expect("The provided account number must be a valid usize");

        if account_num % batch_size != 0 {
            panic!("The account number must be a multiple of batch size");
        }
        account_reader = Box::new(parser::RandomAccountParser::new(account_num, asset_num));
    } else {
        let parser = FilesParser::new(FilesCfg {
            dir: std::path::PathBuf::from_str(&cfg.prover.user_data_path).unwrap(),
            batch_size: cfg.prover.batch_size,
            num_of_tokens: cfg.prover.num_of_tokens,
        });
        parser.log_state();
        account_reader = Box::new(parser);
    }

    let batch_num = account_reader.total_num_of_users().div_ceil(batch_size);

    match GLOBAL_MST.set(RwLock::new(GlobalMst::new(GlobalConfig {
        num_of_tokens: cfg.prover.num_of_tokens,
        num_of_batches: batch_num,
        batch_size: batch_size,
        recursion_branchout_num: cfg.prover.recursion_branchout_num,
    }))) {
        Ok(_) => (),
        Err(_) => {
            panic!("set global mst error");
        }
    }

    // TODO: tmp hardcode three levels of recursive circuits, each branching out 64 children, with the last level with zk enabled.
    // Hence given batch_size=1024, the current setting can support 268M (1024*64^3) accounts, enough for the foreseeable future. (Currently we have 10M accounts)
    let recursive_circuit_configs = vec![STANDARD_CONFIG, STANDARD_CONFIG, STANDARD_ZK_CONFIG];

    let circuit_registry = CircuitRegistry::<RECURSION_BRANCHOUT_NUM>::init(
        batch_size,
        asset_num,
        STANDARD_CONFIG,
        recursive_circuit_configs,
    );

    tracing::info!(
        "start to prove {} accounts with {} tokens, {} batch size",
        account_reader.total_num_of_users(),
        asset_num,
        batch_size
    );
    let expected_batch_num = account_reader.total_num_of_users() / batch_size;

    let start = std::time::Instant::now();
    let mut offset = 0;
    let num_cpus = num_cpus::get();
    let per_parse_account_num = num_cpus * batch_size; // as we use one thread to prove each batch, we load num_cpus batches to increase the parallelism.

    let mut parse_num = 0;
    let mut batch_proofs = vec![];

    while offset < account_reader.total_num_of_users() {
        parse_num += 1;
        let mut accounts: Vec<Account> =
            account_reader.read_n_accounts(offset, per_parse_account_num);
        let account_num = accounts.len();
        if account_num % batch_size != 0 {
            let pad_num = batch_size - account_num % batch_size;
            tracing::info!("in {} parse, account number {} is not a multiple of batch size {}, hence padding {} empty accounts", parse_num, account_num, batch_size,pad_num);
            accounts.resize(account_num + pad_num, Account::get_empty_account(asset_num));
        }

        assert_eq!(account_num % batch_size, 0);

        tracing::info!(
            "parse {} times, with number of accounts {}, number of batches {}",
            parse_num,
            account_num,
            expected_batch_num,
        );

        let batch_idx_base = batch_proofs.len();
        let root_hashes: Vec<HashOut<F>> = accounts
            .par_chunks(batch_size)
            .enumerate()
            .map(|(i, account_batch)| {
                let batch_idx = batch_idx_base + i;
                let mst = MerkleSumTree::new_tree_from_accounts(&account_batch.to_vec());

                let global_mst = GLOBAL_MST.get().unwrap();
                let mut _g = global_mst.write().expect("unable to get a lock");

                for inner_tree_node_idx in 0..batch_size * 2 - 1 {
                    _g.set_batch_hash(
                        batch_idx,
                        inner_tree_node_idx,
                        mst.merkle_sum_tree[inner_tree_node_idx].hash,
                    );
                }
                drop(_g);
                mst.get_root().hash
            })
            .collect();

        let proofs = batch_prove_accounts(
            &circuit_registry,
            accounts,
            BATCH_PROVING_THREADS_NUM,
            batch_size,
        );

        assert_eq!(proofs.len(), root_hashes.len());

        proofs.iter().zip(root_hashes.iter()).enumerate().for_each(|(i, (proof, root_hash))|{
            let batch_idx = batch_idx_base + i;
            // exclude the first two pub inputs for equity and debt
            let proof_root_hash = HashOut::<F>::from_partial(&proof.public_inputs[2..]);
            if proof_root_hash != *root_hash {
                panic!("The root hash in proof is not equal to the one generated by merkle sum tree for batch {}", batch_idx);
            }
        });

        batch_proofs.extend(proofs.into_iter());

        tracing::info!(
            "finish {}/{} batches of accounts in {} parse, since start {:?}",
            batch_proofs.len(),
            expected_batch_num,
            parse_num,
            start.elapsed()
        );
        offset += per_parse_account_num;
    }

    tracing::info!(
        "finish batch proving {} accounts, generating {} proofs in {:?}",
        account_reader.total_num_of_users(),
        batch_proofs.len(),
        start.elapsed()
    );

    let batch_proof_num = batch_proofs.len();

    let (batch_circuit, _) = circuit_registry.get_batch_circuit();
    let mut last_level_circuit_vd = batch_circuit.verifier_only.clone();
    let mut last_level_proofs = batch_proofs;
    let recursive_levels = circuit_registry.get_recursive_levels();

    // level 0 for mst root hash
    for level in 1..=recursive_levels {
        let start = std::time::Instant::now();
        let last_level_vd_digest = last_level_circuit_vd.circuit_digest;
        let last_level_empty_proof =
            circuit_registry.get_empty_proof(&last_level_vd_digest).expect(
                format!("fail to find empty proof for circuit vd {:?}", last_level_vd_digest)
                    .as_str(),
            );

        let subproof_len = last_level_proofs.len();

        if subproof_len % RECURSION_BRANCHOUT_NUM != 0 {
            let pad_num = RECURSION_BRANCHOUT_NUM - subproof_len % RECURSION_BRANCHOUT_NUM;
            tracing::info!("At level {}, {} subproofs are not a multiple of RECURSION_BRANCHOUT_NUM {}, hence padding {} empty proofs. ", level, subproof_len, RECURSION_BRANCHOUT_NUM, pad_num);

            last_level_proofs.resize(subproof_len + pad_num, last_level_empty_proof.clone());
        }

        last_level_proofs.iter().enumerate().for_each(|(i, proof)| {
            let proof_root_hash = HashOut::<F>::from_partial(&proof.public_inputs[2..]);

            let global_mst = GLOBAL_MST.get().unwrap();
            let mut _g = global_mst.write().expect("unable to get a lock");
            _g.set_recursive_hash(level - 1, i, proof_root_hash);
            drop(_g);
        });

        let this_level_proofs = prove_subproofs(
            last_level_proofs,
            last_level_circuit_vd.clone(),
            &circuit_registry,
            RECURSIVE_PROVING_THREADS_NUM,
            level,
        );

        let recursive_circuit = circuit_registry
            .get_recursive_circuit(&last_level_circuit_vd.circuit_digest)
            .expect(
                format!(
                    "No recursive circuit found for inner circuit with vd {:?}",
                    last_level_circuit_vd.circuit_digest
                )
                .as_str(),
            )
            .0;

        last_level_circuit_vd = recursive_circuit.verifier_only.clone();
        last_level_proofs = this_level_proofs;

        tracing::info!(
            "finish recursive level {} with {} proofs in : {:?}",
            level,
            last_level_proofs.len(),
            start.elapsed()
        );
    }

    if last_level_proofs.len() != 1 {
        panic!("The last level proofs should be of length 1, but got {}", last_level_proofs.len());
    }
    let root_proof = last_level_proofs.pop().unwrap();

    // Set the root hash of the recursive circuit to the global mst
    let proof_root_hash = HashOut::<F>::from_partial(&root_proof.public_inputs[2..]);

    let global_mst = GLOBAL_MST.get().unwrap();
    let mut _g = global_mst.write().expect("unable to get a lock");
    _g.set_recursive_hash(recursive_levels, 0, proof_root_hash);
    drop(_g);

    assert!(!GLOBAL_MST.get().unwrap().read().unwrap().is_integral());

    circuit_registry
        .get_root_circuit()
        .verify(root_proof.clone())
        .expect("fail to verify root proof");

    tracing::info!(
        "finish recursive proving {} subproofs in {:?}",
        batch_proof_num,
        start.elapsed()
    );

    if !bench_mode {
        let root_circuit_digest = circuit_registry.get_root_circuit().verifier_only.circuit_digest;

        let proof = Proof {
            round_num: cfg.prover.round_no,
            root_vd_digest: root_circuit_digest,
            proof: root_proof,
        };

        let proof_path_str = arg1;
        let proof_path = PathBuf::from(proof_path_str);
        let mut file = File::create(proof_path.clone())
            .expect(format!("fail to create proof file at {:#?}", proof_path).as_str());
        file.write_all(json!(proof).to_string().as_bytes()).expect("fail to write proof to file");
    }
}
