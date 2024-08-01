use plonky2::{
    hash::{hash_types::HashOut, poseidon::PoseidonHash},
    plonk::config::Hasher,
};
use plonky2_field::types::Field;

use crate::types::F;

/// A struct representing a users account. It represents their equity and debt as a Vector of goldilocks field elements.
#[derive(Debug, Clone)]
pub struct Account {
    pub id: String,
    pub equity: Vec<F>,
    pub debt: Vec<F>,
}

impl Account {
    /// Gets the account hash for a given account.
    pub fn get_hash(&self) -> HashOut<F> {
        let sum_equity = self.equity.iter().fold(F::ZERO, |acc, x| acc + *x);

        let sum_debt = self.debt.iter().fold(F::ZERO, |acc, x| acc + *x);

        let hash = PoseidonHash::hash_no_pad(vec![sum_equity, sum_debt].as_slice());

        hash
    }
}
