use itertools::Itertools;
use plonky2_field::types::Field;
use zk_por_core::{
    account::{gen_accounts_with_random_data, Account},
    circuit_config::STANDARD_CONFIG,
    circuit_registry::registry::CircuitRegistry,
    e2e::{batch_prove_accounts, recursive_prove_subproofs},
    types::F,
};

use zk_por_tracing::{init_tracing, TraceConfig};

#[test]
fn test_prove() {
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

    const RECURSION_FACTOR: usize = 4;
    let batch_size = 8;
    let asset_num = 4;

    let circuit_registry = CircuitRegistry::<RECURSION_FACTOR>::init(
        batch_size,
        asset_num,
        STANDARD_CONFIG,
        vec![STANDARD_CONFIG; 2],
    );

    let proving_thread_num = 2;

    // a total of 9 batches (3x3) to test for padding in each level.
    let mut equity_sum = 0;
    let mut debt_sum = 0;
    let mut batch_proofs = vec![];
    for _ in 0..3 {
        let accounts = gen_accounts_with_random_data(batch_size * 3, 4);

        equity_sum += accounts
            .iter()
            .map(|account| account.equity.iter().map(|e| e.0).sum::<u64>())
            .sum::<u64>();
        debt_sum += accounts
            .iter()
            .map(|account| account.debt.iter().map(|e| e.0).sum::<u64>())
            .sum::<u64>();

        let account_batches: Vec<Vec<Account>> = accounts
            .into_iter()
            .chunks(batch_size)
            .into_iter()
            .map(|chunk| chunk.collect())
            .collect();

        let proofs = batch_prove_accounts(&circuit_registry, account_batches, proving_thread_num);
        batch_proofs.extend(proofs.into_iter());
    }

    let root_proof = recursive_prove_subproofs(batch_proofs, &circuit_registry, proving_thread_num);

    tracing::debug!("equity_sum: {}, debt_sum: {}", equity_sum, debt_sum);
    assert_eq!(F::from_canonical_u64(equity_sum), root_proof.public_inputs[0],);
    assert_eq!(F::from_canonical_u64(debt_sum), root_proof.public_inputs[1],);
}
