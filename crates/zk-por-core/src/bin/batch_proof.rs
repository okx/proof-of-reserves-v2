use std::str::FromStr;

use plonky2::{
    iop::witness::PartialWitness,
    plonk::{circuit_builder::CircuitBuilder, circuit_data::CircuitData, prover::prove},
    util::timing::TimingTree,
};
use plonky2_field::goldilocks_field::GoldilocksField;

use tracing::Level;
use zk_por_core::{
    circuit_config::STANDARD_CONFIG,
    config::ProverConfig,
    merkle_sum_prover::prover::MerkleSumTreeProver,
    parser::read_json_into_accounts_vec,
    types::{C, D, F},
};
use zk_por_tracing::{init_tracing, TraceConfig};

fn main() {
    let cfg = ProverConfig::try_new().unwrap();

    let trace_cfg = TraceConfig {
        prefix: cfg.log.file_name_prefix,
        dir: cfg.log.dir,
        level: Level::from_str(&cfg.log.level).unwrap(),
        console: cfg.log.console,
        flame: cfg.log.flame,
    };
    let guard = init_tracing(trace_cfg);

    let mut builder = CircuitBuilder::<F, D>::new(STANDARD_CONFIG);
    let mut pw = PartialWitness::<GoldilocksField>::new();

    let path = "../../test-data/batch0.json";
    let accounts = read_json_into_accounts_vec(path);
    let prover = MerkleSumTreeProver {
        // batch_id: 0,
        accounts,
    };

    prover.build_and_set_merkle_tree_targets(&mut builder, &mut pw);

    let data = builder.build::<C>();

    let CircuitData { prover_only, common, verifier_only: _ } = &data;

    println!("Started Proving");
    let mut timing = TimingTree::new("prove", log::Level::Debug);
    let proof_res = prove(&prover_only, &common, pw.clone(), &mut timing);
    let proof = proof_res.expect("Proof failed");

    println!("Verifying Proof");
    // Verify proof
    let _proof_verification_res = data.verify(proof.clone()).unwrap();
    drop(guard);
}
