use plonky2::{
    hash::{hash_types::HashOut, poseidon::PoseidonHash},
    plonk::config::Hasher,
};
use plonky2_field::types::Field;

use crate::types::F;

/// A struct representing a users account. It represents their equity and debt as a Vector of goldilocks field elements.
#[derive(Debug, Clone)]
pub struct Account {
    pub id: String, // 256 bit hex string
    pub equity: Vec<F>,
    pub debt: Vec<F>,
}

impl Account {
    /// Gets the account hash for a given account.
    pub fn get_hash(&self) -> HashOut<F> {
        let sum_equity = self.equity.iter().fold(F::ZERO, |acc, x| acc + *x);

        let sum_debt = self.debt.iter().fold(F::ZERO, |acc, x| acc + *x);

        let id = self.get_user_id_in_field();

        let hash =
            PoseidonHash::hash_no_pad(vec![id, vec![sum_equity, sum_debt]].concat().as_slice());

        hash
    }

    /// Gets a user id as a vec of 5 GF elements.
    pub fn get_user_id_in_field(&self) -> Vec<F> {
        assert!(self.id.len() == 64);
        let segments = vec![
            self.id[0..14].to_string(),  // First 56 bits (14 hex chars)
            self.id[14..28].to_string(), // Second 56 bits
            self.id[28..42].to_string(), // Third 56 bits
            self.id[42..56].to_string(), // Fourth 56 bits
            self.id[56..64].to_string(), // Remaining 32 bits (8 hex chars, fits in 56 bits)
        ];

        segments
            .iter()
            .map(|seg| F::from_canonical_u64(u64::from_str_radix(seg, 16).unwrap()))
            .collect::<Vec<F>>()
    }
}
