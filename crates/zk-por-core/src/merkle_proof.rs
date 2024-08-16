use itertools::Itertools;
use plonky2::{
    hash::{hash_types::HashOut, poseidon::PoseidonHash},
    plonk::config::Hasher,
};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

use crate::{
    account::Account,
    database::{DataBase, UserId},
    global::GlobalMst,
    merkle_sum_prover::utils::{hash_2_subhashes, hash_inputs},
    types::{D, F},
};

/// We use this wrapper struct for the left and right indexes of our recursive siblings. This is needed so a user knows the position of
/// their own hash when hashing.
#[derive(Debug, Clone, PartialEq)]
pub struct RecursiveIndex {
    left_indexes: Vec<usize>,
    right_indexes: Vec<usize>,
}

/// Indexes for a given users merkle proof of inclusion siblings in the Global Merkle Sum Tree
#[derive(Debug, Clone, PartialEq)]
pub struct MerkleProofIndex {
    pub sum_tree_siblings: Vec<usize>,
    pub recursive_tree_siblings: Vec<RecursiveIndex>,
}

impl MerkleProofIndex {
    pub fn new_from_user_index(user_index: usize, global_mst: &GlobalMst) -> MerkleProofIndex {
        let sum_tree_siblings = get_mst_siblings_index(user_index, global_mst);
        let recursive_tree_siblings = get_recursive_siblings_index(user_index, global_mst);

        MerkleProofIndex { sum_tree_siblings, recursive_tree_siblings }
    }
}

/// Get the siblings index for the merkle proof of inclusion given a leaf index of a binary merkle sum tree.
/// We get the parent index of a leaf using the formula: parent = index / 2 + num_leaves
pub fn get_mst_siblings_index(global_index: usize, global_mst: &GlobalMst) -> Vec<usize> {
    // Make sure our global index is within the number of leaves
    assert!(global_index < global_mst.get_num_of_leaves());

    let batch_idx = global_index / global_mst.cfg.batch_size;
    let mut siblings = Vec::new();

    // This is the index in the local mst tree
    let mut local_index = global_index % global_mst.cfg.batch_size;

    while local_index < (global_mst.cfg.batch_size * 2 - 2) {
        if local_index % 2 == 1 {
            let sibling_index = local_index - 1;
            siblings.push(sibling_index);
        } else {
            let sibling_index = local_index + 1;
            siblings.push(sibling_index);
        }

        let parent = local_index / 2 + global_mst.cfg.batch_size;
        local_index = parent;
    }

    siblings.par_iter().map(|x| global_mst.get_batch_tree_global_index(batch_idx, *x)).collect()
}

/// Gets the recursive siblings indexes (recursive tree is n-ary tree) as a Vec of vecs, each inner vec is one layer of siblings.
pub fn get_recursive_siblings_index(
    global_index: usize,
    global_mst: &GlobalMst,
) -> Vec<RecursiveIndex> {
    // Make sure our global index is within the number of leaves
    assert!(global_index < global_mst.get_num_of_leaves());

    let mut siblings = Vec::new();
    let local_mst_root_index = global_mst.cfg.batch_size * 2 - 2;
    let mst_batch_idx = global_index / global_mst.cfg.batch_size;
    let this_mst_root_idx =
        global_mst.get_batch_tree_global_index(mst_batch_idx, local_mst_root_index);

    let first_mst_root_idx = global_mst.get_batch_tree_global_index(0, local_mst_root_index);
    assert!(this_mst_root_idx >= first_mst_root_idx);

    let this_mst_root_offset = this_mst_root_idx - first_mst_root_idx;
    let mut recursive_idx = this_mst_root_offset / global_mst.cfg.recursion_branchout_num;
    let mut recursive_offset = this_mst_root_offset % global_mst.cfg.recursion_branchout_num;

    let layers = (global_mst.cfg.num_of_batches.next_power_of_two() as f64)
        .log(global_mst.cfg.recursion_branchout_num as f64)
        .ceil() as usize;

    for i in 0..layers {
        let mut left_layer = Vec::new();
        let mut right_layer = Vec::new();
        if i == 0 {
            for j in 0..global_mst.cfg.recursion_branchout_num {
                if j < recursive_offset {
                    let index = first_mst_root_idx
                        + (global_mst.cfg.recursion_branchout_num * recursive_idx)
                        + j;
                    left_layer.push(index);
                }

                if j > recursive_offset {
                    let index = first_mst_root_idx
                        + (global_mst.cfg.recursion_branchout_num * recursive_idx)
                        + j;
                    right_layer.push(index);
                }
            }
        } else {
            for j in 0..global_mst.cfg.recursion_branchout_num {
                if j < recursive_offset {
                    let index = global_mst.get_recursive_global_index(
                        i,
                        recursive_idx * global_mst.cfg.recursion_branchout_num + j,
                    );
                    left_layer.push(index);
                }

                if j > recursive_offset {
                    let index = global_mst.get_recursive_global_index(
                        i,
                        recursive_idx * global_mst.cfg.recursion_branchout_num + j,
                    );
                    right_layer.push(index);
                }
            }
        }

        siblings.push(RecursiveIndex { left_indexes: left_layer, right_indexes: right_layer });

        recursive_offset = recursive_idx % global_mst.cfg.recursion_branchout_num;
        recursive_idx = recursive_idx / global_mst.cfg.recursion_branchout_num;
    }

    siblings
}

/// Hashes for a given users merkle proof of inclusion siblings in the Global Merkle Sum Tree
#[derive(Debug, Clone)]
pub struct MerkleProof {
    pub sum_tree_siblings: Vec<HashOut<F>>,
    pub recursive_tree_siblings: Vec<RecursiveHashes>,
}

/// We use this wrapper struct for the left and right hashes of our recursive siblings. This is needed so a user knows the position of
/// their own hash when hashing.
#[derive(Debug, Clone, PartialEq)]
pub struct RecursiveHashes {
    left_hashes: Vec<HashOut<F>>,
    right_hashes: Vec<HashOut<F>>,
}

impl RecursiveHashes {
    pub fn new_from_index(indexes: &RecursiveIndex, db: &DataBase) -> Self {
        let left_hashes = indexes
            .left_indexes
            .iter()
            .map(|y| db.get_gmst_node_hash(*y as i32).unwrap())
            .collect_vec();
        let right_hashes = indexes
            .right_indexes
            .iter()
            .map(|y| db.get_gmst_node_hash(*y as i32).unwrap())
            .collect_vec();
        RecursiveHashes { left_hashes, right_hashes }
    }

    pub fn get_calculated_hash(self, own_hash: HashOut<F>) -> HashOut<F> {
        let mut hash_inputs = self.left_hashes;
        hash_inputs.push(own_hash);
        hash_inputs.extend(self.right_hashes);

        let inputs: Vec<F> = hash_inputs.iter().map(|x| x.elements).flatten().collect();

        PoseidonHash::hash_no_pad(inputs.as_slice())
    }
}

impl MerkleProof {
    pub fn new_from_user_id(user_id: UserId, db: &DataBase, global_mst: &GlobalMst) -> MerkleProof {
        let user_index = db.get_user_index(user_id);
        if user_index.is_none() {
            tracing::error!("User with id: {:?} does not exist", user_id.to_string());
        }

        let indexes =
            MerkleProofIndex::new_from_user_index(user_index.unwrap() as usize, global_mst);
        let merkle_proof = get_merkle_proof_hashes_from_indexes(&indexes, db);
        merkle_proof
    }

    pub fn verify_merkle_proof(
        &self,
        account: &Account,
        db: DataBase,
        gmst_root: HashOut<F>,
    ) -> Result<Account, String> {
        let account_hash = account.get_hash();
        let user_index_res = db.get_user_index(UserId::from_hex_string(account.id.clone()));
        if user_index_res.is_none() {
            tracing::error!("User with id: {:?} does not exist", account.id.to_string());
        }

        let mut user_index = user_index_res.unwrap();

        let calculated_mst_hash = self.sum_tree_siblings.iter().fold(account_hash, |acc, x| {
            if user_index % 2 == 0 {
                user_index /= 2;
                hash_2_subhashes::<F, D>(x, &acc)
            } else {
                user_index /= 2;
                hash_2_subhashes::<F, D>(&acc, x)
            }
        });

        let calculated_hash = self
            .recursive_tree_siblings
            .iter()
            .fold(calculated_mst_hash, |acc, x| x.clone().get_calculated_hash(acc));

        if calculated_hash == gmst_root {
            Ok(account.clone())
        } else {
            Err("Merkle Proof is not verified".to_string())
        }
    }
}

/// Given the indexes for the MST siblings, get the hashes from the database for the merkle proof of inclusion.
pub fn get_merkle_proof_hashes_from_indexes(
    indexes: &MerkleProofIndex,
    db: &DataBase,
) -> MerkleProof {
    let mst_hashes: Vec<HashOut<F>> = indexes
        .sum_tree_siblings
        .iter()
        .map(|x| db.get_gmst_node_hash(*x as i32).unwrap())
        .collect();

    let recursive_hashes: Vec<RecursiveHashes> = indexes
        .recursive_tree_siblings
        .iter()
        .map(|x| RecursiveHashes::new_from_index(x, db))
        .collect();

    MerkleProof { sum_tree_siblings: mst_hashes, recursive_tree_siblings: recursive_hashes }
}

#[cfg(test)]
pub mod test {
    use crate::{
        global::{GlobalConfig, GlobalMst},
        merkle_proof::{get_recursive_siblings_index, RecursiveIndex},
    };

    use super::get_mst_siblings_index;

    #[test]
    pub fn test_get_siblings_index() {
        let gmst = GlobalMst::new(GlobalConfig {
            num_of_tokens: 100,
            num_of_batches: 4,
            batch_size: 8,
            recursion_branchout_num: 4,
        });

        let global_index = 0;

        let siblings = get_mst_siblings_index(global_index, &gmst);
        assert_eq!(siblings, vec![1, 33, 49]);

        let gmst = GlobalMst::new(GlobalConfig {
            num_of_tokens: 100,
            num_of_batches: 8,
            batch_size: 8,
            recursion_branchout_num: 4,
        });

        let global_index = 0;

        let siblings = get_mst_siblings_index(global_index, &gmst);
        assert_eq!(siblings, vec![1, 65, 97]);

        let gmst = GlobalMst::new(GlobalConfig {
            num_of_tokens: 100,
            num_of_batches: 6,
            batch_size: 8,
            recursion_branchout_num: 4,
        });

        let global_index = 0;

        let siblings = get_mst_siblings_index(global_index, &gmst);
        assert_eq!(siblings, vec![1, 49, 73]);
    }

    #[test]
    pub fn test_get_recursive_siblings_index() {
        let gmst = GlobalMst::new(GlobalConfig {
            num_of_tokens: 100,
            num_of_batches: 15,
            batch_size: 4,
            recursion_branchout_num: 4,
        });

        let global_index = 0;

        let siblings = get_recursive_siblings_index(global_index, &gmst);

        assert_eq!(
            siblings,
            vec![
                RecursiveIndex { left_indexes: vec![], right_indexes: vec![91, 92, 93] },
                RecursiveIndex { left_indexes: vec![], right_indexes: vec![107, 108, 109] }
            ]
        );

        let gmst = GlobalMst::new(GlobalConfig {
            num_of_tokens: 100,
            num_of_batches: 30,
            batch_size: 8,
            recursion_branchout_num: 4,
        });

        let global_index = 163;

        let siblings = get_recursive_siblings_index(global_index, &gmst);
        assert_eq!(
            siblings,
            vec![
                RecursiveIndex { left_indexes: vec![], right_indexes: vec![441, 442, 443] },
                RecursiveIndex { left_indexes: vec![456], right_indexes: vec![458, 459] },
                RecursiveIndex { left_indexes: vec![460], right_indexes: vec![462, 463] }
            ]
        );

        let gmst = GlobalMst::new(GlobalConfig {
            num_of_tokens: 100,
            num_of_batches: 6,
            batch_size: 4,
            recursion_branchout_num: 4,
        });

        let global_index = 20;

        let siblings = get_recursive_siblings_index(global_index, &gmst);
        assert_eq!(
            siblings,
            vec![
                RecursiveIndex { left_indexes: vec![40], right_indexes: vec![42, 43] },
                RecursiveIndex { left_indexes: vec![44], right_indexes: vec![46, 47] },
            ]
        );
    }
}
