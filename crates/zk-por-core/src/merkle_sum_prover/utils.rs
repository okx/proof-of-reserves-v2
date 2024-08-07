use plonky2::{
    hash::{
        hash_types::{HashOut, RichField},
        poseidon::PoseidonHash,
    },
    plonk::config::Hasher,
};
use plonky2_field::extension::Extendable;

pub fn hash_2_subhashes<F: RichField + Extendable<D>, const D: usize>(
    hash_1: &HashOut<F>,
    hash_2: &HashOut<F>,
) -> HashOut<F> {
    #[allow(clippy::useless_vec)]
    let inputs = vec![hash_1.elements.to_vec(), hash_2.elements.to_vec()].concat();
    hash_inputs(inputs)
}

pub fn hash_inputs<F: RichField>(inputs: Vec<F>) -> HashOut<F> {
    let hash = PoseidonHash::hash_no_pad(inputs.as_slice());
    hash
}
