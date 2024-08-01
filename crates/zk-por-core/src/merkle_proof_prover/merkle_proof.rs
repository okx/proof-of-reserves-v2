use plonky2::hash::hash_types::HashOut;

use crate::{account::Account, merkle_sum_prover::merkle_sum_tree::MerkleSumTree, types::F};

use plonky2_field::types::Field;

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

    pub fn new_from_merkle_tree(
        index: usize,
        account: &Account,
        tree: &MerkleSumTree,
    ) -> MerkleProofProvingInputs {
        MerkleProofProvingInputs {
            siblings: tree.get_siblings_hashes(index),
            root: tree.get_root().hash,
            index: F::from_canonical_u64(index as u64),
            account: account.clone(),
        }
    }
}
