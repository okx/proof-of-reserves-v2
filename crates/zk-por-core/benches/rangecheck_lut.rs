use criterion::{
    criterion_group, criterion_main, measurement::WallTime, BenchmarkGroup, Criterion, SamplingMode,
};
use plonky2::{
    gates::lookup_table::LookupTable,
    iop::witness::{PartialWitness, WitnessWrite},
    plonk::{
        circuit_builder::CircuitBuilder,
        circuit_data::CircuitConfig,
        config::{GenericConfig, PoseidonGoldilocksConfig},
        proof::ProofWithPublicInputs,
        prover::prove,
    },
    util::timing::TimingTree,
};
use plonky2_field::{
    goldilocks_field::GoldilocksField,
    types::{Field, Sample},
};
use rand::Rng;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use std::sync::Arc;
use zk_por_core::{
    account::gen_accounts_with_random_data,
    circuit_config::STANDARD_CONFIG,
    gadgets::rangecheck::RangeCheckTargets,
    merkle_sum_prover::{
        circuits::merkle_sum_circuit::build_merkle_sum_tree_circuit, prover::MerkleSumTreeProver,
    },
    recursive_prover::{prover::RecursiveProver, recursive_circuit::build_recursive_n_circuit},
    types::{C, D, F},
    U16_TABLE,
};

fn gen_rand(max: u64) -> u64 {
    let mut rng = rand::thread_rng();
    let random_number: u64 = rng.gen_range(0..max);
    random_number
}

const SIZE: usize = 1024 * 220;
const MAX_VAL: u64 = 1 << 48;
/// Benchmark the range check numbers using lookup table of multiple inputs
pub fn bench_lut_rangecheck(c: &mut Criterion) {
    const D: usize = 2;
    type C = PoseidonGoldilocksConfig;
    type F = <C as GenericConfig<D>>::F;

    let vals_to_check =
        (0..SIZE).into_iter().map(|_| F::from_canonical_u64(gen_rand(MAX_VAL))).collect::<Vec<F>>();

    let table =
        U16_TABLE.into_iter().enumerate().map(|(i, v)| (i as u16, v)).collect::<Vec<(u16, u16)>>();

    let table: LookupTable = Arc::new(table);

    let mut group = c.benchmark_group("lut");
    group.sample_size(10);

    group.bench_function("lut_rangecheck", |b| {
        let config = CircuitConfig::standard_recursion_config();
        let mut builder = CircuitBuilder::<F, D>::new(config);

        let table_index = builder.add_lookup_table_from_pairs(table.clone());
        let mut pw = PartialWitness::new();
        vals_to_check.iter().for_each(|value| {
            let target = RangeCheckTargets::new(&mut builder, table_index);

            let low = value.0 & 0xFFFF;
            let high = value.0 >> 32;
            let mid = (value.0 >> 16) & 0xFFFF;
            let limbs = vec![
                F::from_canonical_u64(low),
                F::from_canonical_u64(mid),
                F::from_canonical_u64(high),
            ];
            target.set_targets(*value, &limbs, &mut pw);
        });

        builder.print_gate_counts(0);
        let data = builder.build::<C>();

        b.iter(|| {
            let mut pw_local = pw.clone();
            let mut timing = TimingTree::default();
            let proof = prove(&data.prover_only, &data.common, pw_local, &mut timing).unwrap();
        })
    });
    group.finish();
}

pub fn bench_rangecheck_bit_split(c: &mut Criterion) {
    const D: usize = 2;
    type C = PoseidonGoldilocksConfig;
    type F = <C as GenericConfig<D>>::F;

    let vals_to_check =
        (0..SIZE).into_iter().map(|_| F::from_canonical_u64(gen_rand(MAX_VAL))).collect::<Vec<F>>();

    let mut group = c.benchmark_group("split");
    group.sample_size(10);
    group.bench_function("rangecheck_split_bits", |b| {
        let config = CircuitConfig::standard_recursion_config();
        let mut builder = CircuitBuilder::<F, D>::new(config);

        let log_max = 48;
        let mut pw = PartialWitness::new();
        // The secret value.
        vals_to_check.iter().for_each(|value| {
            let val_target = builder.add_virtual_target();
            builder.range_check(val_target, log_max);

            pw.set_target(val_target, *value);
        });
        builder.print_gate_counts(0);

        let data = builder.build::<C>();

        b.iter(|| {
            let mut pw_local = pw.clone();
            let mut timing = TimingTree::default();
            let proof = data.prove(pw_local).unwrap();
        })
    });
    group.finish();
}

criterion_group!(benches, bench_lut_rangecheck, bench_rangecheck_bit_split);
criterion_main!(benches);
