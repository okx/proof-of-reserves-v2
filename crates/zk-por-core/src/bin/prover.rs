use plonky2::{hash::hash_types::HashOut, plonk::proof::ProofWithPublicInputs};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{fs::File, io::Write, path::PathBuf, str::FromStr, env};
use zk_por_core::{
    account::Account, circuit_config::{STANDARD_CONFIG, STANDARD_ZK_CONFIG}, circuit_registry::registry::CircuitRegistry, config::ProverConfig, e2e::{batch_prove_accounts, recursive_prove_subproofs}, parser::{self, AccountParser, FilesCfg, FilesParser}, types::{C, D, F}
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
    init_tracing(trace_cfg);

    const RECURSION_FACTOR: usize = 64;
    const BATCH_PROVING_THREADS_NUM: usize = 4;
    const RECURSIVE_PROVING_THREADS_NUM: usize = 2;

    if cfg.prover.hyper_tree_size as usize != RECURSION_FACTOR {
        panic!("The hyper_tree_size is not configured to be equal to 64 (Recursion_Factor)");
    }
    let batch_size = cfg.prover.batch_size as usize;
    let asset_num = cfg.prover.num_of_tokens as usize;

    // the path to dump the final generated proof
    let mut bench_mode = true;
    let args: Vec<String> = env::args().collect();
    let arg1 = args.get(1)
        .expect("Please provide the first argument, either proof path or '--bench' for benchmark mode");
    let mut account_reader : Box<dyn AccountParser>;
    if arg1 == "--bench" {
        bench_mode = true;
        let account_num = args.get(2)
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

    // Hardcode three levels of recursive circuits, each branching out 64 children, with the last level with zk enabled.
    // Hence given batch_size=1024, the current setting can support 268M (1024*64^3) accounts, enough for the foreseeable future. (Currently we have 10M accounts)
    let recursive_circuit_configs = vec![STANDARD_CONFIG, STANDARD_CONFIG, STANDARD_ZK_CONFIG];

    let circuit_registry = CircuitRegistry::<RECURSION_FACTOR>::init(
        batch_size,
        asset_num,
        STANDARD_CONFIG,
        recursive_circuit_configs,
    );

    tracing::info!("start to prove {} accounts with {} tokens, {} batch size", account_reader.total_num_of_users(), asset_num, batch_size);

    let start = std::time::Instant::now();
    let mut offset = 0;
    let per_parse_account_num = 10 * batch_size;
    let mut parse_num = 0;
    let mut batch_proofs = vec![];
    while offset < account_reader.total_num_of_users() {
        parse_num += 1;
        let accounts = account_reader.read_n_accounts(offset, per_parse_account_num);
        let account_num = accounts.len();
        assert_eq!(account_num % batch_size, 0);

        let account_batches: Vec<Vec<Account>> = accounts
            .into_iter()
            .chunks(batch_size)
            .into_iter()
            .map(|chunk| chunk.collect())
            .collect();

        tracing::info!(
            "parse {} times, with number of accounts {}, number of batches {}",
            parse_num,
            account_num,
            account_batches.len(),
        );

        let proofs = batch_prove_accounts(&circuit_registry, account_batches, BATCH_PROVING_THREADS_NUM);
        offset += batch_size;
        batch_proofs.extend(proofs.into_iter());
        tracing::info!("finish {} batches in {} parse, since start {:?}", batch_proofs.len(), parse_num, start.elapsed());
    }

    tracing::info!("finish batch proving {} accounts, generating {} proofs in {:?}", account_reader.total_num_of_users(), batch_proofs.len(), start.elapsed());

    let batch_proof_num = batch_proofs.len();  
    let root_proof = recursive_prove_subproofs(batch_proofs, &circuit_registry, RECURSIVE_PROVING_THREADS_NUM);
    tracing::info!("finish recursive proving {} subproofs in {:?}", batch_proof_num, start.elapsed());

    let root_circuit_digest = circuit_registry.get_root_circuit().verifier_only.circuit_digest;

    let proof = Proof {
        round_num: cfg.prover.round_no,
        root_vd_digest: root_circuit_digest,
        proof: root_proof,
    };

    if !bench_mode {
        let proof_path_str = arg1;
        let proof_path = PathBuf::from(proof_path_str);
        let mut file = File::create(proof_path.clone())
            .expect(format!("fail to create proof file at {:#?}", proof_path).as_str());
        file.write_all(json!(proof).to_string().as_bytes()).expect("fail to write proof to file");
    }
}
