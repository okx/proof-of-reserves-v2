use plonky2::{
    hash::hash_types::HashOutTarget, iop::target::Target, plonk::circuit_builder::CircuitBuilder,
};

use crate::{
    circuit_utils::{hash_2_subhashes_circuit, select_hash},
    merkle_sum_prover::circuits::account_circuit::{AccountSumTargets, AccountTargets},
    types::{D, F},
};

pub struct MerkleProofTargets {
    pub merkle_root_target: HashOutTarget,
    pub account_hash: HashOutTarget,
    pub index_target: Target,
    pub siblings: Vec<HashOutTarget>,
    pub merkle_tree_depth: usize,
}

impl MerkleProofTargets {
    pub fn new_from_account_targets(
        builder: &mut CircuitBuilder<F, D>,
        account_targets: &AccountTargets,
        merkle_proof_len: usize,
    ) -> MerkleProofTargets {
        let merkle_root_target = builder.add_virtual_hash();
        let account_sum_targets = AccountSumTargets::from_account_target(&account_targets, builder);
        let account_hash = account_sum_targets.get_account_hash_targets(builder);
        let index_target = builder.add_virtual_target();
        let siblings: Vec<HashOutTarget> =
            (0..merkle_proof_len).enumerate().map(|_| builder.add_virtual_hash()).collect();
        MerkleProofTargets {
            merkle_root_target,
            account_hash,
            index_target,
            siblings,
            merkle_tree_depth: merkle_proof_len,
        }
    }

    /// Given the siblings in a merkle tree and my root hash, verify the merkle proof of inclusion of the supplied leaf hash.
    /// Since the order of the hash depends on my siblings position, we use the index bits of the index target to determine the order of the
    /// hash inputs.
    pub fn verify_merkle_proof_circuit(self, builder: &mut CircuitBuilder<F, D>) {
        // Get the index bits up to the merkle tree depth number of bits from Little endian representation
        let index_bits = builder.split_le(self.index_target, self.merkle_tree_depth);

        let mut leaf_hash = self.account_hash;

        for i in 0..self.siblings.len() {
            let sibling = self.siblings.get(i).unwrap();

            // Order is based on the index bits at the ith index
            let first_hash = select_hash(builder, index_bits[i], *sibling, leaf_hash);
            let second_hash = select_hash(builder, index_bits[i], leaf_hash, *sibling);
            leaf_hash = hash_2_subhashes_circuit(builder, &first_hash, &second_hash);
        }

        builder.connect_hashes(leaf_hash, self.merkle_root_target);
    }
}
