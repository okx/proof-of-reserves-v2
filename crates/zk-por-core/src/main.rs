use zk_por_tracing::{init_tracing, TraceConfig};

#[cfg(not(feature = "mpi"))]
pub fn main() {
    use tracing::{debug, Level};

    let cfg = TraceConfig {
        prefix: "zkpor".to_string(),
        dir: "logs".to_string(),
        level: Level::DEBUG,
        console: true,
        flame: false,
    };
    let guard = init_tracing(cfg);
    debug!("tracing works");
    drop(guard)
}

#[cfg(feature = "mpi")]
fn main() {
    // a total of 9 batches (3x3) to test for padding in each level
    // run it with 3 MPI processes:
    // $ cargo build -r
    // $ mpirun -np 3 target/release/zk-por-core

    use mpi::traits::*;
    use plonky2::plonk::{config::PoseidonGoldilocksConfig, proof::ProofWithPublicInputs};
    use plonky2_field::types::Field;
    use zk_por_core::{
        account::gen_accounts_with_random_data,
        circuit_config::STANDARD_CONFIG,
        circuit_registry::registry::CircuitRegistry,
        e2e::{batch_prove_accounts, recursive_prove_subproofs},
        types::F,
    };

    let universe = mpi::initialize().unwrap();
    let world = universe.world();
    let root_rank = 0;
    let rank = world.rank();

    assert_eq!(world.size(), 3);

    let cfg = TraceConfig {
        prefix: "zkpor".to_string(),
        dir: "logs".to_string(),
        level: tracing::Level::DEBUG,
        console: true,
        flame: false,
    };

    {
        init_tracing(cfg)
    };

    const RECURSION_BRANCHOUT_NUM: usize = 4;
    let batch_size = 8;
    let token_num = 4;

    let circuit_registry = CircuitRegistry::<RECURSION_BRANCHOUT_NUM>::init(
        batch_size,
        token_num,
        STANDARD_CONFIG,
        vec![STANDARD_CONFIG; 2],
    );
    let circuit_data_ref = &circuit_registry.get_batch_circuit().0.common;

    let proving_thread_num = 2;

    let mut equity_sum = 0;
    let mut debt_sum = 0;

    // This is tha same as zk-por-core/tests/e2e.rs: we replace the for loop with MPI processes.
    // for _ in 0..3 {

    let accounts = gen_accounts_with_random_data(batch_size * 3, 4);

    equity_sum +=
        accounts.iter().map(|account| account.equity.iter().map(|e| e.0).sum::<u64>()).sum::<u64>();
    debt_sum +=
        accounts.iter().map(|account| account.debt.iter().map(|e| e.0).sum::<u64>()).sum::<u64>();

    let proofs = batch_prove_accounts(&circuit_registry, accounts, proving_thread_num, batch_size);

    if rank == root_rank {
        let mut batch_proofs = vec![];
        batch_proofs.extend(proofs.into_iter());
        for i in 1..world.size() {
            let partial_equity_sum = world.process_at_rank(i).receive::<u64>();
            let partial_debt_sum = world.process_at_rank(i).receive::<u64>();
            equity_sum += partial_equity_sum.0;
            debt_sum += partial_debt_sum.0;

            let msg_num_proofs = world.process_at_rank(i).receive::<usize>();
            let num_proofs = msg_num_proofs.0;
            tracing::debug!("Process {} got message: number of proofs: {}.", rank, num_proofs);
            for _ in 0..num_proofs {
                let msg_proof_size = world.process_at_rank(i).receive::<usize>();
                let proof_size = msg_proof_size.0;
                tracing::debug!("Process {} got message: proof size: {}.", rank, proof_size);

                let mut proof_buffer = vec![0u8; proof_size];
                world.process_at_rank(i).receive_into(&mut proof_buffer);

                let proof = ProofWithPublicInputs::<F, PoseidonGoldilocksConfig, 2>::from_bytes(
                    proof_buffer,
                    &circuit_data_ref,
                )
                .unwrap();
                batch_proofs.push(proof);
            }
        }
        let root_proof =
            recursive_prove_subproofs(batch_proofs, &circuit_registry, proving_thread_num);
        tracing::debug!("equity_sum: {}, debt_sum: {}", equity_sum, debt_sum);
        assert_eq!(F::from_canonical_u64(equity_sum), root_proof.public_inputs[0],);
        assert_eq!(F::from_canonical_u64(debt_sum), root_proof.public_inputs[1],);
        tracing::info!("Proofs verified successfully!");
    } else {
        world.process_at_rank(root_rank).send(&equity_sum);
        world.process_at_rank(root_rank).send(&debt_sum);
        let nproofs = proofs.len();
        world.process_at_rank(root_rank).send(&nproofs);
        proofs.into_iter().for_each(|proof| {
            let bytes = proof.to_bytes();
            let num_bytes = bytes.len();
            world.process_at_rank(root_rank).send(&num_bytes);
            world.process_at_rank(root_rank).send(&bytes);
        });
    }

    // end of loop
    // }
}
