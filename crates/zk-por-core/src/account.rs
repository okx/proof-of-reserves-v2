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
    pub fn get_hash(self) -> HashOut<F> {
        let mut sum_equity = F::ZERO;
        let mut sum_debt = F::ZERO;

        self.equity.iter().for_each(|x| {
            sum_equity = sum_equity + *x;
        });
        self.debt.iter().for_each(|x| {
            sum_debt = sum_debt + *x;
        });

        let hash = PoseidonHash::hash_no_pad(vec![sum_equity, sum_debt].as_slice());

        hash
    }
}
