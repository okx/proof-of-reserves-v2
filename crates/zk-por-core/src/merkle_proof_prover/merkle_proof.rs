use plonky2::hash::hash_types::HashOut;

use crate::{account::Account, types::F};

/// Proving Inputs required for a merkle proof. These inputs are used to prove the merkle tree proof of inclusion for a single
/// user.
pub struct MerkleProofProvingInputs {
    pub siblings: Vec<HashOut<F>>,
    pub root: HashOut<F>,
    pub index: F,
    pub account: Account,
}

impl MerkleProofProvingInputs {
    /// Gets the leaf hash of an account
    pub fn get_account_hash(self) -> HashOut<F> {
        self.account.get_hash()
    }
}
