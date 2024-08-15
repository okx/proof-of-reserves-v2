use super::constant::{
    BATCH_PROVING_THREADS_NUM, RECURSION_BRANCHOUT_NUM, RECURSIVE_PROVING_THREADS_NUM,
};
use indicatif::ProgressBar;
use plonky2::hash::hash_types::HashOut;
use rayon::{iter::ParallelIterator, prelude::*};
use serde_json::json;
use std::{fs::File, io::Write, path::PathBuf, str::FromStr, sync::RwLock};
use zk_por_core::{
    account::{persist_account_id_to_gmst_pos, Account},
    circuit_config::{get_recursive_circuit_configs, STANDARD_CONFIG},
    circuit_registry::registry::CircuitRegistry,
    config::ProverConfig,
    database::{DataBase, DbOption},
    e2e::{batch_prove_accounts, prove_subproofs},
    error::PoRError,
    global::{GlobalConfig, GlobalMst, GLOBAL_MST},
    merkle_sum_prover::circuits::merkle_sum_circuit::MerkleSumNodeTarget,
    merkle_sum_tree::MerkleSumTree,
    parser::{AccountParser, FilesCfg, FilesParser},
    recursive_prover::recursive_circuit::RecursiveTargets,
    types::F,
    General, Proof,
};
use zk_por_tracing::{init_tracing, TraceConfig};

pub fn prove(cfg: ProverConfig, proof_output_path: PathBuf) -> Result<(), PoRError> {
    let trace_cfg: TraceConfig = cfg.log.into();
    let _g = init_tracing(trace_cfg);

    let mut database = DataBase::new(DbOption {
        user_map_dir: cfg.db.level_db_user_path.to_string(),
        gmst_dir: cfg.db.level_db_gmst_path.to_string(),
    });

    let batch_size = cfg.prover.batch_size as usize;
    let token_num = cfg.prover.num_of_tokens as usize;

    // the path to dump the final generated proof
    let parser = FilesParser::new(FilesCfg {
        dir: std::path::PathBuf::from_str(&cfg.prover.user_data_path).unwrap(),
        batch_size: cfg.prover.batch_size,
        num_of_tokens: cfg.prover.num_of_tokens,
    });
    parser.log_state();
    let mut account_parser: Box<dyn AccountParser> = Box::new(parser);

    let batch_num = account_parser.total_num_of_users().div_ceil(batch_size);

    match GLOBAL_MST.set(RwLock::new(GlobalMst::new(GlobalConfig {
        num_of_tokens: token_num,
        num_of_batches: batch_num,
        batch_size: batch_size,
        recursion_branchout_num: RECURSION_BRANCHOUT_NUM,
    }))) {
        Ok(_) => (),
        Err(_) => {
            panic!("set global mst error");
        }
    }

    let recursive_circuit_configs =
        get_recursive_circuit_configs::<RECURSION_BRANCHOUT_NUM>(batch_num);
    let recursive_level = recursive_circuit_configs.len();

    tracing::info!(
        "start to precompute circuits and empty proofs for {} recursive levels",
        recursive_level
    );
    let circuit_registry = CircuitRegistry::<RECURSION_BRANCHOUT_NUM>::init(
        batch_size,
        token_num,
        STANDARD_CONFIG,
        recursive_circuit_configs,
    );

    tracing::info!(
        "start to prove {} accounts with {} tokens, {} batch size, {} recursive level",
        account_parser.total_num_of_users(),
        token_num,
        batch_size,
        recursive_level,
    );

    let start = std::time::Instant::now();
    let mut offset = 0;
    let num_cpus = num_cpus::get();
    let num_cpus =
        if BATCH_PROVING_THREADS_NUM < num_cpus { BATCH_PROVING_THREADS_NUM } else { num_cpus };
    let per_parse_account_num = num_cpus * batch_size; // as we use one thread to prove each batch, we load num_cpus batches to increase the parallelism.

    let mut parse_num = 0;
    let mut batch_proofs = vec![];
    let bar = ProgressBar::new(account_parser.total_num_of_users() as u64);
    while offset < account_parser.total_num_of_users() {
        parse_num += 1;
        let mut accounts: Vec<Account> =
            account_parser.read_n_accounts(offset, per_parse_account_num);
        let account_num_read = accounts.len();

        persist_account_id_to_gmst_pos(&mut database, &accounts, offset);

        let account_num = accounts.len();
        if account_num % batch_size != 0 {
            let pad_num = batch_size - account_num % batch_size;
            tracing::info!("in {} parse, account number {} is not a multiple of batch size {}, hence padding {} empty accounts", parse_num, account_num, batch_size,pad_num);
            accounts.resize(account_num + pad_num, Account::get_empty_account(token_num));
        }

        assert_eq!(account_num % batch_size, 0);

        tracing::debug!(
            "parse {} times, with number of accounts {}, number of batches {}",
            parse_num,
            account_num,
            batch_num,
        );

        let msts: Vec<MerkleSumTree> = accounts
            .par_chunks(batch_size)
            .map(|account_batch| MerkleSumTree::new_tree_from_accounts(&account_batch.to_vec()))
            .collect();

        let global_mst = GLOBAL_MST.get().unwrap();
        let mut _g: std::sync::RwLockWriteGuard<GlobalMst> =
            global_mst.write().expect("unable to get a lock");
        let batch_idx_base = batch_proofs.len();

        let root_hashes: Vec<HashOut<F>> = msts
            .into_iter()
            .enumerate()
            .map(|(i, mst)| {
                let batch_idx = batch_idx_base + i;
                mst.merkle_sum_tree.iter().enumerate().for_each(|(j, node)| {
                    _g.set_batch_hash(batch_idx, j, node.hash);
                });
                mst.get_root().hash
            })
            .collect();
        drop(_g);

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
            let hash_offset = MerkleSumNodeTarget::pub_input_root_hash_offset();
            let proof_root_hash = HashOut::<F>::from_partial(&proof.public_inputs[hash_offset]);
            if proof_root_hash != *root_hash {
                panic!("The root hash in proof is not equal to the one generated by merkle sum tree for batch {}", batch_idx);
            }
        });

        batch_proofs.extend(proofs.into_iter());

        tracing::debug!(
            "finish {}/{} batches of accounts in {} parse, since start {:?}",
            batch_proofs.len(),
            batch_num,
            parse_num,
            start.elapsed()
        );
        bar.inc(account_num as u64);
        offset += account_num_read;
    }
    bar.finish();

    tracing::info!(
        "finish batch proving {} accounts, generating {} proofs in {:?}",
        account_parser.total_num_of_users(),
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
        let last_level_empty_proof = circuit_registry
            .get_empty_proof(&last_level_vd_digest)
            .expect(
                format!("fail to find empty proof for circuit vd {:?}", last_level_vd_digest)
                    .as_str(),
            )
            .clone();

        let subproof_len = last_level_proofs.len();

        tracing::info!(
            "start to recursively prove {} subproofs at level {}/{}",
            subproof_len,
            level,
            recursive_levels,
        );

        if subproof_len % RECURSION_BRANCHOUT_NUM != 0 {
            let pad_num = RECURSION_BRANCHOUT_NUM - subproof_len % RECURSION_BRANCHOUT_NUM;
            tracing::info!("At level {}, {} subproofs are not a multiple of RECURSION_BRANCHOUT_NUM {}, hence padding {} empty proofs. ", level, subproof_len, RECURSION_BRANCHOUT_NUM, pad_num);

            last_level_proofs.resize(subproof_len + pad_num, last_level_empty_proof);
        }

        let global_mst = GLOBAL_MST.get().unwrap();
        let mut _g = global_mst.write().expect("unable to get a lock");
        last_level_proofs.iter().enumerate().for_each(|(i, proof)| {
            let hash_offset = RecursiveTargets::<RECURSION_BRANCHOUT_NUM>::pub_input_hash_offset();
            let proof_root_hash = HashOut::<F>::from_partial(&proof.public_inputs[hash_offset]);

            _g.set_recursive_hash(level - 1, i, proof_root_hash);
        });
        drop(_g);

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

        tracing::debug!(
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
    let hash_offset = RecursiveTargets::<RECURSION_BRANCHOUT_NUM>::pub_input_hash_offset();
    let proof_root_hash = HashOut::<F>::from_partial(&root_proof.public_inputs[hash_offset]);

    let global_mst = GLOBAL_MST.get().unwrap();
    let mut _g = global_mst.write().expect("unable to get a lock");
    _g.set_recursive_hash(recursive_levels, 0, proof_root_hash);
    drop(_g);

    let start = std::time::Instant::now();
    assert!(GLOBAL_MST.get().unwrap().read().unwrap().is_integral());
    tracing::info!("verify global mst in {:?}", start.elapsed());

    circuit_registry
        .get_root_circuit()
        .verify(root_proof.clone())
        .expect("fail to verify root proof");

    tracing::info!(
        "finish recursive proving {} subproofs in {:?}",
        batch_proof_num,
        start.elapsed()
    );

    let root_circuit_digest = circuit_registry.get_root_circuit().verifier_only.circuit_digest;

    let proof = Proof {
        general: General {
            round_num: cfg.prover.round_no,
            batch_num: batch_num,
            recursion_branchout_num: RECURSION_BRANCHOUT_NUM,
            batch_size: batch_size,
            token_num: token_num,
        },
        root_vd_digest: root_circuit_digest,
        proof: root_proof,
    };

    let mut file = File::create(proof_output_path.clone())
        .expect(format!("fail to create proof file at {:#?}", proof_output_path).as_str());
    file.write_all(json!(proof).to_string().as_bytes()).expect("fail to write proof to file");

    // persist gmst to database

    let global_mst = GLOBAL_MST.get().unwrap();
    let _g = global_mst.read().expect("unable to get a lock");
    let start = std::time::Instant::now();
    _g.persist(&mut database);
    tracing::info!("persist gmst to db in {:?}", start.elapsed());

    return Ok(());
}
