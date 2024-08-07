use zk_por_core::{
    account::{gen_accounts_with_random_data, Account},
    circuit_registry::registry::CircuitRegistry,
    config::ProverConfig,
};

use plonky2::{hash::hash_types::HashOut, plonk::proof::ProofWithPublicInputs};
use serde::{Deserialize, Serialize};
use std::{env, sync::mpsc, thread, time::Instant};
use zk_por_core::{
    circuit_config::{STANDARD_CONFIG, STANDARD_ZK_CONFIG},
    e2e::stream_prove,
    types::{C, D, F},
};
use zk_por_tracing::{init_tracing, TraceConfig};

#[derive(Serialize, Deserialize)]
struct Proof {
    round_num: u32,
    root_vd_digest: HashOut<F>,
    proof: ProofWithPublicInputs<F, C, D>,
}

/// this binary to benchmark e2e proving.
/// we donot use criterion because it will run the benchmark multiple times, which will be too slow.
fn main() {
    let args: Vec<String> = env::args().collect();
    let file_num: usize = args
        .get(1)
        .expect("Please provide the file number as the first argument")
        .parse()
        .expect("The provided file number must be a valid usize");

    let per_file_batch_num: usize = args
        .get(2)
        .expect("Please provide the per file batch number as the second argument")
        .parse()
        .expect("The provided number must be a valid usize");

    let cfg = TraceConfig {
        prefix: "zkpor-bench".to_string(),
        dir: "logs".to_string(),
        level: tracing::Level::INFO,
        console: true,
        flame: false,
    };

    {
        init_tracing(cfg)
    };

    let cfg = ProverConfig::try_new().unwrap();
    // let trace_cfg: TraceConfig = cfg.log.into();
    // init_tracing(trace_cfg);
    const RECURSION_FACTOR: usize = 64;
    const PROVING_THREADS_NUM: usize = 4;
    let batch_size = cfg.prover.batch_size as usize;
    let asset_num = 200; // TODO: read from config
    let recursive_circuit_configs = vec![STANDARD_CONFIG, STANDARD_CONFIG, STANDARD_ZK_CONFIG];

    let circuit_registry = CircuitRegistry::<RECURSION_FACTOR>::init(
        batch_size,
        asset_num,
        STANDARD_CONFIG,
        recursive_circuit_configs,
    );

    let (tx, rx) = mpsc::channel::<Vec<Account>>();
    let start = Instant::now();
    let prover =
        thread::spawn(move || stream_prove(rx, &circuit_registry, batch_size, PROVING_THREADS_NUM));

    thread::spawn(move || {
        (0..file_num).for_each(|_| {
            let accounts =
                gen_accounts_with_random_data(per_file_batch_num * batch_size, asset_num);
            tx.send(accounts).unwrap()
        })
    });

    _ = prover.join().unwrap();
    tracing::info!(
        "finish e2e proving {} accounts in {:?}",
        batch_size * file_num * per_file_batch_num,
        start.elapsed()
    );
}
