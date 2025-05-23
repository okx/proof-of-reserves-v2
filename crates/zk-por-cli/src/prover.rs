use super::constant::{
    DEFAULT_BATCH_SIZE, GLOBAL_INFO_FILENAME, GLOBAL_PROOF_FILENAME, RECURSION_BRANCHOUT_NUM,
    USER_PROOF_DIRNAME,
};
use indicatif::ProgressBar;

use mpi::{traits::*, Threading};

use plonky2::{
    hash::hash_types::HashOut,
    plonk::{config::PoseidonGoldilocksConfig, proof::ProofWithPublicInputs},
    util::serialization::DefaultGateSerializer,
};
use plonky2_field::{goldilocks_field::GoldilocksField, types::PrimeField64};
use rayon::{iter::ParallelIterator, prelude::*};

use std::{
    fs,
    fs::File,
    io::{BufWriter, Write},
    path::PathBuf,
    str::FromStr,
    sync::{Arc, RwLock},
};
use zk_por_core::{
    account::{persist_account_id_to_gmst_pos, Account},
    circuit_config::{get_recursive_circuit_configs, STANDARD_CONFIG},
    circuit_registry::registry::CircuitRegistry,
    config::{ConfigProver, ProverConfig},
    database::{init_db, PoRDB},
    e2e::{batch_prove_accounts, prove_subproofs},
    error::PoRError,
    global::{GlobalConfig, GlobalMst, GLOBAL_MST},
    merkle_proof::MerkleProof,
    merkle_sum_prover::circuits::merkle_sum_circuit::MerkleSumNodeTarget,
    merkle_sum_tree::MerkleSumTree,
    parser::{AccountParser, FileAccountReader, FileManager, FilesCfg},
    recursive_prover::recursive_circuit::RecursiveTargets,
    types::F,
    CircuitsInfo, General, Info, Proof,
};
use zk_por_tracing::{init_tracing, TraceConfig};

// as we use one thread to prove each batch, we load num_cpus batches to increase the parallelism.
pub fn calculate_per_parse_account_num(batch_size: usize, threads_num: usize) -> usize {
    let num_cpus = num_cpus::get();
    let num_cpus = if threads_num < num_cpus { threads_num } else { num_cpus };
    num_cpus * batch_size
}

fn ensure_output_dir_empty(user_proof_dir: PathBuf) -> Result<(), PoRError> {
    fs::create_dir_all(&user_proof_dir).map_err(|e| return PoRError::Io(e))?;
    let is_empty =
        fs::read_dir(user_proof_dir.clone()).map_err(|e| return PoRError::Io(e))?.count() == 0;
    if !is_empty {
        return Err(PoRError::Io(std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            format!(
                "user proof output directory {} is not empty",
                user_proof_dir.to_str().unwrap(),
            ),
        )));
    }
    return Ok(());
}

pub fn prove(cfg: ProverConfig, proof_output_path: PathBuf) -> Result<(), PoRError> {
    let trace_cfg: TraceConfig = cfg.log.into();

    let _g = init_tracing(trace_cfg);
    let user_proof_output_path = proof_output_path.join(USER_PROOF_DIRNAME);
    ensure_output_dir_empty(user_proof_output_path)?;

    let mut database = init_db(cfg.db);

    let batch_size = cfg.prover.batch_size.unwrap_or(DEFAULT_BATCH_SIZE);
    let token_num = cfg.prover.tokens.len();
    let batch_prove_threads_num = cfg.prover.batch_prove_threads_num;
    let recursive_prove_threads_num = cfg.prover.recursive_prove_threads_num;

    // the path to dump the final generated proof
    let file_manager = FileManager {};
    let mut account_parser = FileAccountReader::new(
        FilesCfg {
            dir: std::path::PathBuf::from_str(&cfg.prover.user_data_path).unwrap(),
            batch_size: batch_size,
            tokens: cfg.prover.tokens.clone(),
        },
        &file_manager,
    );
    account_parser.log_state();

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
    let batch_circuit_config = STANDARD_CONFIG;
    let circuit_registry = CircuitRegistry::<RECURSION_BRANCHOUT_NUM>::init(
        batch_size,
        token_num,
        batch_circuit_config.clone(),
        recursive_circuit_configs.clone(),
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
    let per_parse_account_num =
        calculate_per_parse_account_num(batch_size, batch_prove_threads_num);

    let mut parse_num = 0;
    let mut batch_proofs = vec![];
    let bar = ProgressBar::new(account_parser.total_num_of_users() as u64);
    while offset < account_parser.total_num_of_users() {
        parse_num += 1;
        let mut accounts =
            account_parser.read_n_accounts(offset, per_parse_account_num, &file_manager);

        persist_account_id_to_gmst_pos(&mut database, &accounts, offset);

        let account_num = accounts.len();
        if account_num % batch_size != 0 {
            let pad_num = batch_size - account_num % batch_size;
            tracing::info!("in {} parse, account number {} is not a multiple of batch size {}, hence padding {} empty accounts", parse_num, account_num, batch_size,pad_num);
            accounts.resize(account_num + pad_num, Account::get_empty_account(token_num));
        }

        assert_eq!(accounts.len() % batch_size, 0);

        tracing::info!(
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

        let proofs =
            batch_prove_accounts(&circuit_registry, accounts, batch_prove_threads_num, batch_size);

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

        tracing::info!(
            "finish {}/{} batches of accounts in {} parse, since start {:?}",
            batch_proofs.len(),
            batch_num,
            parse_num,
            start.elapsed()
        );
        bar.inc(account_num as u64);
        offset += per_parse_account_num;
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
            recursive_prove_threads_num,
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

    let root_vd_digest = circuit_registry.get_root_circuit().verifier_only.circuit_digest;

    let root_circuit_verifier_data = circuit_registry.get_root_circuit().verifier_data();

    let root_circuit_verifier_data_bytes = root_circuit_verifier_data
        .to_bytes(&DefaultGateSerializer)
        .expect("fail to serialize root circuit verifier data");
    let root_circuit_verifier_data_hex_str = hex::encode(root_circuit_verifier_data_bytes);

    let proof = Proof {
        general: General {
            round_num: cfg.prover.round_no,
            recursion_branchout_num: RECURSION_BRANCHOUT_NUM,
            batch_size: batch_size,
            token_num: token_num,
        },
        circuits_info: Some(CircuitsInfo {
            batch_circuit_config: batch_circuit_config,
            recursive_circuit_configs: recursive_circuit_configs,
            root_verifier_data_hex: root_circuit_verifier_data_hex_str,
        }),
        root_vd_digest: root_vd_digest,
        proof: root_proof,
    };

    // persist gmst to database

    let global_mst = GLOBAL_MST.get().unwrap();

    let _g = global_mst.read().expect("unable to get a lock");
    let root_hash = _g.get_root().expect("no root");
    tracing::info!("root hash is {:?}", root_hash);
    let start = std::time::Instant::now();
    _g.persist(&mut database);
    tracing::info!("persist gmst to db in {:?}", start.elapsed());

    dump_proofs(&cfg.prover, proof_output_path, database, &proof)?;
    tracing::info!("finish dumping global proof and user proofs in {:?}", start.elapsed());

    return Ok(());
}

fn dump_proofs(
    cfg: &ConfigProver,
    proof_output_dir_path: PathBuf,
    db: Box<dyn PoRDB>,
    root_proof: &Proof,
) -> Result<(), PoRError> {
    let user_proof_output_dir_path = proof_output_dir_path.join(USER_PROOF_DIRNAME); // directory has been checked empty before.

    let global_proof_output_path = proof_output_dir_path.join(GLOBAL_PROOF_FILENAME);
    let global_proof_file =
        File::create(global_proof_output_path.clone()).map_err(|e| PoRError::Io(e))?;

    let mut global_proof_writer = BufWriter::new(global_proof_file);
    serde_json::to_writer(&mut global_proof_writer, &root_proof).expect(
        format!("fail to dump global proof file to {:?}", global_proof_output_path).as_str(),
    );
    global_proof_writer.flush()?;

    ///////////////////////////////////////////////
    let hash_offset = RecursiveTargets::<RECURSION_BRANCHOUT_NUM>::pub_input_hash_offset();
    let root_hash = HashOut::<F>::from_partial(&root_proof.proof.public_inputs[hash_offset]);
    let root_hash_bytes = root_hash
        .elements
        .iter()
        .map(|x| x.to_canonical_u64().to_le_bytes())
        .flatten()
        .collect::<Vec<u8>>();
    let root_hash = hex::encode(root_hash_bytes);

    let equity_offset = RecursiveTargets::<RECURSION_BRANCHOUT_NUM>::pub_input_equity_offset();
    let equity_sum = root_proof.proof.public_inputs[equity_offset].to_canonical_u64();

    let debt_offset = RecursiveTargets::<RECURSION_BRANCHOUT_NUM>::pub_input_debt_offset();
    let debt_sum = root_proof.proof.public_inputs[debt_offset].to_canonical_u64();
    assert!(equity_sum >= debt_sum);
    let balance_sum = equity_sum - debt_sum;
    let info = Info {
        root_hash: root_hash,
        equity_sum: equity_sum,
        debt_sum: debt_sum,
        balance_sum: balance_sum,
    };

    let global_info_output_path = proof_output_dir_path.join(GLOBAL_INFO_FILENAME);
    let global_info_file =
        File::create(global_info_output_path.clone()).map_err(|e| PoRError::Io(e))?;

    let mut global_info_writer = BufWriter::new(global_info_file);
    serde_json::to_writer(&mut global_info_writer, &info).expect(
        format!("fail to dump global info file to {:?}", global_proof_output_path).as_str(),
    );
    global_info_writer.flush()?;

    ///////////////////////////////////////////////
    // generate and dump proof for each user
    // create a new account reader to avoid buffering previously loaded accounts in memory
    let file_manager = FileManager {};
    let batch_size = cfg.batch_size.unwrap_or(DEFAULT_BATCH_SIZE);
    let mut account_reader = FileAccountReader::new(
        FilesCfg {
            dir: std::path::PathBuf::from_str(&cfg.user_data_path).unwrap(),
            batch_size: batch_size,
            tokens: cfg.tokens.clone(),
        },
        &file_manager,
    );

    let global_cfg = GlobalConfig {
        num_of_tokens: cfg.tokens.len(),
        num_of_batches: account_reader.total_num_of_batches,
        batch_size: batch_size,
        recursion_branchout_num: RECURSION_BRANCHOUT_NUM,
    };
    let user_num = account_reader.total_num_of_users();

    tracing::info!("start to generate and dump merkle proof for each of {} accounts", user_num);

    let bar = ProgressBar::new(user_num as u64);
    let per_parse_account_num =
        calculate_per_parse_account_num(batch_size, cfg.batch_prove_threads_num);

    let cdb: Arc<dyn PoRDB> = Arc::from(db);
    let mut offset = 0;
    let chunk_size: usize = num_cpus::get();
    while offset < account_reader.total_num_of_users() {
        let accounts: Vec<Account> =
            account_reader.read_n_accounts(offset, per_parse_account_num, &file_manager);
        accounts.chunks(chunk_size).for_each(|chunk| {
            chunk.par_iter().for_each(|account| {
                let user_proof = MerkleProof::new_from_account(account, cdb.clone(), &global_cfg)
                    .expect(
                        format!("fail to generate merkle proof for account {}", account.id)
                            .as_str(),
                    );

                let user_proof_output_path =
                    user_proof_output_dir_path.join(format!("{}.json", account.id));

                let user_proof_file = File::create(user_proof_output_path).expect(
                    format!("fail to create user proof file for account {}", user_proof.account.id)
                        .as_str(),
                );

                let mut user_proof_writer = BufWriter::new(user_proof_file);
                serde_json::to_writer(&mut user_proof_writer, &user_proof).expect(
                    format!("fail to write user proof file for account {}", user_proof.account.id)
                        .as_str(),
                );
                user_proof_writer.flush().expect(
                    format!("fail to write user proof file for account {}", user_proof.account.id)
                        .as_str(),
                )
            });

            bar.inc(chunk.len() as u64);
        });
        offset += per_parse_account_num;
    }
    bar.finish();

    return Ok(());
}

fn update_global_mst(
    batch_idx_base: usize,
    msts: &Vec<MerkleSumTree>,
    proofs: &Vec<ProofWithPublicInputs<GoldilocksField, PoseidonGoldilocksConfig, 2>>,
) {
    let global_mst = GLOBAL_MST.get().unwrap();
    let mut _g: std::sync::RwLockWriteGuard<GlobalMst> =
        global_mst.write().expect("unable to get a lock");

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
}

pub fn prove_distributed(cfg: ProverConfig, proof_output_path: PathBuf) -> Result<(), PoRError> {
    // let universe = mpi::initialize().unwrap();
    let (universe, threading) = mpi::initialize_with_threading(Threading::Multiple).unwrap();
    assert_eq!(threading, mpi::environment::threading_support());
    println!("Supported level of threading: {:?}", threading);
    let world = universe.world();
    let root_rank = 0;
    let rank = world.rank();
    let root_process = world.process_at_rank(root_rank);

    let trace_cfg: TraceConfig = cfg.log.into();
    let _g = init_tracing(trace_cfg);

    let batch_size = cfg.prover.batch_size.unwrap_or(DEFAULT_BATCH_SIZE);
    let token_num = cfg.prover.tokens.len();
    let batch_prove_threads_num = cfg.prover.batch_prove_threads_num;
    let recursive_prove_threads_num = cfg.prover.recursive_prove_threads_num;

    if rank == root_rank {
        // other ranks do the following
        tracing::info!("MPI rank {} starting...", rank);

        let user_proof_output_path = proof_output_path.join(USER_PROOF_DIRNAME);
        ensure_output_dir_empty(user_proof_output_path).expect("output dir is not empty");

        let mut database = init_db(cfg.db);

        // the path to dump the final generated proof
        let file_manager = FileManager {};
        let mut account_parser = FileAccountReader::new(
            FilesCfg {
                dir: std::path::PathBuf::from_str(&cfg.prover.user_data_path).unwrap(),
                batch_size: batch_size,
                tokens: cfg.prover.tokens.clone(),
            },
            &file_manager,
        );
        account_parser.log_state();

        let batch_num = account_parser.total_num_of_users().div_ceil(batch_size);
        tracing::info!(
            "MPI rank {} read {} accounts, with batch size {}, total {} batches",
            rank,
            account_parser.total_num_of_users(),
            batch_size,
            batch_num
        );

        // broadcast the batch_num to all ranks
        // note: we could not use broadcast_into() due to synchronization issues
        for i in 1..world.size() {
            world.process_at_rank(i).send(&batch_num);
        }

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
            "MPI rank {} start to precompute circuits and empty proofs for {} recursive levels",
            rank,
            recursive_level
        );
        let batch_circuit_config = STANDARD_CONFIG;
        let circuit_registry = CircuitRegistry::<RECURSION_BRANCHOUT_NUM>::init(
            batch_size,
            token_num,
            batch_circuit_config.clone(),
            recursive_circuit_configs.clone(),
        );

        world.barrier();

        tracing::info!(
            "MPI rank 0 start to prove {} accounts with {} tokens, {} batch size, {} recursive level",
            account_parser.total_num_of_users(),
            token_num,
            batch_size,
            recursive_level,
        );

        let start = std::time::Instant::now();
        let mut offset = 0;
        let per_parse_account_num =
            calculate_per_parse_account_num(batch_size, batch_prove_threads_num);

        let mut parse_num = 0;
        let mut batch_proofs = vec![];
        let bar = ProgressBar::new(account_parser.total_num_of_users() as u64);

        let mut local_accounts_root_rank: Vec<Vec<Account>> = Vec::new();
        let mut remote_batches_per_rank: Vec<usize> = vec![0; world.size() as usize];

        while offset < account_parser.total_num_of_users() {
            parse_num += 1;
            let mut accounts =
                account_parser.read_n_accounts(offset, per_parse_account_num, &file_manager);

            persist_account_id_to_gmst_pos(&mut database, &accounts, offset);

            let account_num = accounts.len();
            if account_num % batch_size != 0 {
                let pad_num = batch_size - account_num % batch_size;
                tracing::info!("in {} parse, account number {} is not a multiple of batch size {}, hence padding {} empty accounts", parse_num, account_num, batch_size,pad_num);
                accounts.resize(account_num + pad_num, Account::get_empty_account(token_num));
            }

            assert_eq!(accounts.len() % batch_size, 0);

            tracing::info!(
                "parse {} times, with number of accounts {}, number of batches {}",
                parse_num,
                account_num,
                batch_num,
            );

            let to_rank = parse_num % world.size();

            if to_rank == root_rank {
                // save in local buffer
                local_accounts_root_rank.push(accounts);
            } else {
                // send to process to_rank
                tracing::info!("MPI process 0 sending to rank {} size {}", to_rank, accounts.len());
                world.process_at_rank(to_rank).send(&(accounts.len()));
                accounts.iter().for_each(|account| {
                    let str_account =
                        serde_json::to_string(account).expect("failed to serialize account");
                    let bytes_account = str_account.as_bytes();
                    world.process_at_rank(to_rank).send(&(bytes_account.len()));
                    world.process_at_rank(to_rank).send(bytes_account);
                });
                remote_batches_per_rank[to_rank as usize] += 1;
            }
            offset += per_parse_account_num;
            bar.inc(account_num as u64);
        }
        let finish_size: usize = 0;
        for i in 1..world.size() {
            world.process_at_rank(i).send(&finish_size);
        }
        let max_remote_batches_per_rank =
            remote_batches_per_rank.iter().max().expect("fail to get max remote batches");

        world.barrier();

        bar.finish();

        local_accounts_root_rank.iter().for_each(|raccounts| {
            let msts: Vec<MerkleSumTree> = (*raccounts)
                .par_chunks(batch_size)
                .map(|account_batch| MerkleSumTree::new_tree_from_accounts(&account_batch.to_vec()))
                .collect();

            let proofs = batch_prove_accounts(
                &circuit_registry,
                (*raccounts).clone(),
                batch_prove_threads_num,
                batch_size,
            );

            update_global_mst(batch_proofs.len(), &msts, &proofs);

            batch_proofs.extend(proofs.into_iter());

            tracing::info!(
                "finish {}/{} batches of accounts in {} parse, since start {:?}",
                batch_proofs.len(),
                batch_num,
                parse_num,
                start.elapsed()
            );
        });

        let circuit_data_ref = &circuit_registry.get_batch_circuit().0.common;

        for rounds in 1..(*max_remote_batches_per_rank + 1) {
            // MPI: get MSTs from other ranks
            for i in 1..world.size() {
                if remote_batches_per_rank[i as usize] < rounds {
                    continue;
                }

                let num_mst = world.process_at_rank(i).receive::<usize>().0;
                tracing::info!("Process {} expects {} MSTs.", rank, num_mst);
                let mut msts: Vec<MerkleSumTree> = Vec::with_capacity(num_mst);
                for _ in 0..num_mst {
                    let msg_mst_size = world.process_at_rank(i).receive::<usize>();
                    let mst_size = msg_mst_size.0;
                    tracing::info!("Process {} got message: mst size: {}.", rank, mst_size);

                    let mut mst_buffer = vec![0u8; mst_size];
                    world.process_at_rank(i).receive_into(&mut mst_buffer);

                    let str_mst = String::from_utf8(mst_buffer).unwrap();
                    let mst: MerkleSumTree = serde_json::from_str(&str_mst)
                        .expect("failed to deserialize merkle sum tree");
                    msts.push(mst);
                }

                let msg_num_proofs = world.process_at_rank(i).receive::<usize>();
                let num_proofs = msg_num_proofs.0;
                tracing::info!("Process {} got message: number of proofs: {}.", rank, num_proofs);
                let mut proofs: Vec<ProofWithPublicInputs<F, PoseidonGoldilocksConfig, 2>> =
                    Vec::with_capacity(num_proofs);
                for _ in 0..num_proofs {
                    let msg_proof_size = world.process_at_rank(i).receive::<usize>();
                    let proof_size = msg_proof_size.0;
                    tracing::info!("Process {} got message: proof size: {}.", rank, proof_size);

                    let mut proof_buffer = vec![0u8; proof_size];
                    world.process_at_rank(i).receive_into(&mut proof_buffer);

                    let proof =
                        ProofWithPublicInputs::<F, PoseidonGoldilocksConfig, 2>::from_bytes(
                            proof_buffer,
                            &circuit_data_ref,
                        )
                        .expect("failed to deserialize proof");

                    proofs.push(proof);
                }

                update_global_mst(batch_proofs.len(), &msts, &proofs);

                batch_proofs.extend(proofs.into_iter());

                tracing::info!(
                    "finish {}/{} batches of accounts in {} parse, since start {:?}",
                    batch_proofs.len(),
                    batch_num,
                    parse_num,
                    start.elapsed()
                );
            }
        }

        tracing::info!(
            "MPI rank 0: finish batch proving {} accounts, generating {} proofs in {:?}",
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
                let hash_offset =
                    RecursiveTargets::<RECURSION_BRANCHOUT_NUM>::pub_input_hash_offset();
                let proof_root_hash = HashOut::<F>::from_partial(&proof.public_inputs[hash_offset]);

                _g.set_recursive_hash(level - 1, i, proof_root_hash);
            });
            drop(_g);

            let this_level_proofs = prove_subproofs(
                last_level_proofs,
                last_level_circuit_vd.clone(),
                &circuit_registry,
                recursive_prove_threads_num,
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
            panic!(
                "The last level proofs should be of length 1, but got {}",
                last_level_proofs.len()
            );
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

        let root_vd_digest = circuit_registry.get_root_circuit().verifier_only.circuit_digest;

        let root_circuit_verifier_data = circuit_registry.get_root_circuit().verifier_data();

        let root_circuit_verifier_data_bytes = root_circuit_verifier_data
            .to_bytes(&DefaultGateSerializer)
            .expect("fail to serialize root circuit verifier data");
        let root_circuit_verifier_data_hex_str = hex::encode(root_circuit_verifier_data_bytes);

        let proof = Proof {
            general: General {
                round_num: cfg.prover.round_no,
                recursion_branchout_num: RECURSION_BRANCHOUT_NUM,
                batch_size: batch_size,
                token_num: token_num,
            },
            circuits_info: Some(CircuitsInfo {
                batch_circuit_config: batch_circuit_config,
                recursive_circuit_configs: recursive_circuit_configs,
                root_verifier_data_hex: root_circuit_verifier_data_hex_str,
            }),
            root_vd_digest: root_vd_digest,
            proof: root_proof,
        };

        // persist gmst to database
        let global_mst = GLOBAL_MST.get().unwrap();

        let _g = global_mst.read().expect("unable to get a lock");
        let root_hash = _g.get_root().expect("no root");
        tracing::info!("root hash is {:?}", root_hash);
        let start = std::time::Instant::now();
        _g.persist(&mut database);
        tracing::info!("persist gmst to db in {:?}", start.elapsed());

        dump_proofs(&cfg.prover, proof_output_path, database, &proof).expect("fail to dump proofs");
        tracing::info!("finish dumping global proof and user proofs in {:?}", start.elapsed());

    // hre it's where root process (0) ends
    } else {
        // other ranks do the following
        tracing::info!("MPI rank {} starting...", rank);

        let batch_num = root_process.receive::<usize>().0;
        tracing::info!("MPI rank {} received batch_num {}", rank, batch_num);

        let recursive_circuit_configs =
            get_recursive_circuit_configs::<RECURSION_BRANCHOUT_NUM>(batch_num);
        let recursive_level = recursive_circuit_configs.len();
        tracing::info!(
            "MPI rank {} start to precompute circuits and empty proofs for {} recursive levels",
            rank,
            recursive_level
        );
        let batch_circuit_config = STANDARD_CONFIG;
        let circuit_registry = CircuitRegistry::<RECURSION_BRANCHOUT_NUM>::init(
            batch_size,
            token_num,
            batch_circuit_config.clone(),
            recursive_circuit_configs.clone(),
        );

        tracing::info!("MPI rank {} before barrier", rank);
        world.barrier();
        tracing::info!("MPI rank {} after barrier", rank);

        let mut local_accounts_root_rank: Vec<Vec<Account>> = Vec::new();

        loop {
            let mut accounts: Vec<Account> = Vec::new();
            let num_accounts = root_process.receive::<usize>().0;
            tracing::info!("MPI rank {} received {} accounts", rank, num_accounts);
            if num_accounts == 0 {
                tracing::info!("MPI rank {} exits accounts loop", rank);
                break;
            }
            for _ in 0..num_accounts {
                let num_account_bytes = root_process.receive::<usize>().0;
                let mut account_bytes = vec![0u8; num_account_bytes];
                root_process.receive_into(&mut account_bytes);
                let account_str =
                    String::from_utf8(account_bytes).expect("failed to convert bytes to string");
                let account =
                    serde_json::from_str(&account_str).expect("failed to deserialize account");
                accounts.push(account);
            }
            local_accounts_root_rank.push(accounts);
        }

        world.barrier();

        tracing::info!(
            "MPI rank {} received {} account batches",
            rank,
            local_accounts_root_rank.len()
        );

        local_accounts_root_rank.iter().for_each(|accounts| {
            let msts: Vec<MerkleSumTree> = accounts
                .par_chunks(batch_size)
                .map(|account_batch| MerkleSumTree::new_tree_from_accounts(&account_batch.to_vec()))
                .collect();

            root_process.send(&(msts.len()));
            msts.iter().for_each(|mst: &MerkleSumTree| {
                let serialized_tree_bytes = serde_json::to_string(&mst).unwrap().into_bytes();
                root_process.send(&(serialized_tree_bytes.len()));
                root_process.send(&serialized_tree_bytes);
            });

            let proofs = batch_prove_accounts(
                &circuit_registry,
                accounts.clone(),
                batch_prove_threads_num,
                batch_size,
            );

            root_process.send(&(proofs.len()));
            proofs.iter().for_each(|proof| {
                let serialized_proof_bytes = proof.to_bytes();
                root_process.send(&(serialized_proof_bytes.len()));
                root_process.send(&serialized_proof_bytes);
            });
        });

        tracing::info!("MPI rank {} finished.", rank);
    }

    return Ok(());
}
