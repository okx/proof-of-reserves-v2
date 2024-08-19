use itertools::Itertools;
use plonky2::{
    hash::{hash_types::HashOut, poseidon::PoseidonHash},
    plonk::config::Hasher,
};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use serde::{Deserialize, Serialize};

use crate::{
    account::Account, database::{DataBase, UserId}, error::PoRError, global::{GlobalConfig, GlobalMst}, merkle_sum_prover::utils::hash_2_subhashes, types::{D, F}
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
    pub fn new_from_user_index(user_index: usize, global_mst: &GlobalConfig) -> MerkleProofIndex {
        let sum_tree_siblings = get_mst_siblings_index(user_index, global_mst);
        let recursive_tree_siblings = get_recursive_siblings_index(user_index, global_mst);

        MerkleProofIndex { sum_tree_siblings, recursive_tree_siblings }
    }
}

/// Get the siblings index for the merkle proof of inclusion given a leaf index of a binary merkle sum tree.
/// We get the parent index of a leaf using the formula: parent = index / 2 + num_leaves
pub fn get_mst_siblings_index(global_leaf_index: usize, cfg: &GlobalConfig) -> Vec<usize> {
    // Make sure our global index is within the number of leaves
    assert!(global_leaf_index < GlobalMst::get_num_of_leaves(cfg));

    let batch_idx = global_leaf_index / cfg.batch_size;
    let mut siblings = Vec::new();

    // This is the index in the local mst tree
    let mut local_index = global_leaf_index % cfg.batch_size;

    while local_index < (cfg.batch_size * 2 - 2) {
        if local_index % 2 == 1 {
            let sibling_index = local_index - 1;
            siblings.push(sibling_index);
        } else {
            let sibling_index = local_index + 1;
            siblings.push(sibling_index);
        }

        let parent = local_index / 2 + cfg.batch_size;
        local_index = parent;
    }

    siblings
        .par_iter()
        .map(|x| GlobalMst::get_batch_tree_global_index(cfg, batch_idx, *x))
        .collect()
}

/// Gets the recursive siblings indexes (recursive tree is n-ary tree) as a Vec of vecs, each inner vec is one layer of siblings.
pub fn get_recursive_siblings_index(
    global_index: usize,
    cfg: &GlobalConfig,
) -> Vec<RecursiveIndex> {
    // Make sure our global index is within the number of leaves
    assert!(global_index < GlobalMst::get_num_of_leaves(cfg));

    let mut siblings = Vec::new();
    let local_mst_root_index = cfg.batch_size * 2 - 2;
    let mst_batch_idx = global_index / cfg.batch_size;
    let this_mst_root_idx =
        GlobalMst::get_batch_tree_global_index(cfg, mst_batch_idx, local_mst_root_index);

    let first_mst_root_idx = GlobalMst::get_batch_tree_global_index(cfg, 0, local_mst_root_index);
    assert!(this_mst_root_idx >= first_mst_root_idx);

    let this_mst_root_offset = this_mst_root_idx - first_mst_root_idx;
    let mut recursive_idx = this_mst_root_offset / cfg.recursion_branchout_num;
    let mut recursive_offset = this_mst_root_offset % cfg.recursion_branchout_num;

    let layers = (cfg.num_of_batches.next_power_of_two() as f64)
        .log(cfg.recursion_branchout_num as f64)
        .ceil() as usize;

    for i in 0..layers {
        let mut left_layer = Vec::new();
        let mut right_layer = Vec::new();
        if i == 0 {
            for j in 0..cfg.recursion_branchout_num {
                if j < recursive_offset {
                    let index =
                        first_mst_root_idx + (cfg.recursion_branchout_num * recursive_idx) + j;
                    left_layer.push(index);
                }

                if j > recursive_offset {
                    let index =
                        first_mst_root_idx + (cfg.recursion_branchout_num * recursive_idx) + j;
                    right_layer.push(index);
                }
            }
        } else {
            for j in 0..cfg.recursion_branchout_num {
                if j < recursive_offset {
                    let index = GlobalMst::get_recursive_global_index(
                        cfg,
                        i,
                        recursive_idx * cfg.recursion_branchout_num + j,
                    );
                    left_layer.push(index);
                }

                if j > recursive_offset {
                    let index = GlobalMst::get_recursive_global_index(
                        cfg,
                        i,
                        recursive_idx * cfg.recursion_branchout_num + j,
                    );
                    right_layer.push(index);
                }
            }
        }

        siblings.push(RecursiveIndex { left_indexes: left_layer, right_indexes: right_layer });

        recursive_offset = recursive_idx % cfg.recursion_branchout_num;
        recursive_idx = recursive_idx / cfg.recursion_branchout_num;
    }

    siblings
}

/// We use this wrapper struct for the left and right hashes of our recursive siblings. This is needed so a user knows the position of
/// their own hash when hashing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

    /// Left hashes || own hash || Right hashes
    pub fn get_calculated_hash(self, own_hash: HashOut<F>) -> HashOut<F> {
        let mut hash_inputs = self.left_hashes;
        hash_inputs.push(own_hash);
        hash_inputs.extend(self.right_hashes);

        let inputs: Vec<F> = hash_inputs.iter().map(|x| x.elements).flatten().collect();

        PoseidonHash::hash_no_pad(inputs.as_slice())
    }
}

/// Hashes for a given users merkle proof of inclusion siblings in the Global Merkle Sum Tree
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MerkleProof {
    pub index: usize,
    pub sum_tree_siblings: Vec<HashOut<F>>,
    pub recursive_tree_siblings: Vec<RecursiveHashes>,
}

impl MerkleProof {
    pub fn new_from_user_id(
        user_id_string: String,
        db: &DataBase,
        cfg: &GlobalConfig,
    ) -> Result<MerkleProof, PoRError> {
        let user_id_res = UserId::from_hex_string(user_id_string);
        if user_id_res.is_err(){
            return Err(user_id_res.unwrap_err());
        }

        let user_id = user_id_res.unwrap();

        let user_index = db.get_user_index(user_id.clone());
        if user_index.is_none() {
            tracing::error!("User with id: {:?} does not exist", user_id.to_string());
            return Err(PoRError::InvalidParameter(user_id.to_string()))
        }

        let indexes = MerkleProofIndex::new_from_user_index(user_index.unwrap() as usize, cfg);
        let merkle_proof =
            get_merkle_proof_hashes_from_indexes(&indexes, user_index.unwrap() as usize, db);
        Ok(merkle_proof)
    }

    pub fn verify_merkle_proof(
        &self,
        account: &Account,
        gmst_root: HashOut<F>,
    ) -> Result<(), PoRError> {
        let account_hash = account.get_hash();

        let mut index = self.index;

        let calculated_mst_hash = self.sum_tree_siblings.iter().fold(account_hash, |acc, x| {
            if index % 2 == 0 {
                index /= 2;
                hash_2_subhashes::<F, D>(&acc, x)
            } else {
                index /= 2;
                hash_2_subhashes::<F, D>(x, &acc)
            }
        });

        let calculated_hash = self
            .recursive_tree_siblings
            .iter()
            .fold(calculated_mst_hash, |acc, x| x.clone().get_calculated_hash(acc));

        if calculated_hash == gmst_root {
            Ok(())
        } else {
            Err(PoRError::InvalidMerkleProof)
        }
    }
}

/// Given the indexes for the MST siblings, get the hashes from the database for the merkle proof of inclusion.
pub fn get_merkle_proof_hashes_from_indexes(
    indexes: &MerkleProofIndex,
    user_index: usize,
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

    MerkleProof {
        sum_tree_siblings: mst_hashes,
        recursive_tree_siblings: recursive_hashes,
        index: user_index,
    }
}

#[cfg(test)]
pub mod test {
    use itertools::Itertools;
    use plonky2::hash::hash_types::HashOut;

    use crate::{
        account::Account,
        global::{GlobalConfig, GlobalMst},
        merkle_proof::{get_recursive_siblings_index, MerkleProofIndex, RecursiveIndex},
        types::F,
    };
    use plonky2_field::types::Field;

    use super::{get_mst_siblings_index, MerkleProof, RecursiveHashes};

    #[test]
    pub fn test_get_siblings_index() {
        let gmst = GlobalMst::new(GlobalConfig {
            num_of_tokens: 100,
            num_of_batches: 4,
            batch_size: 8,
            recursion_branchout_num: 4,
        });

        let global_index = 0;

        let siblings = get_mst_siblings_index(global_index, &gmst.cfg);
        assert_eq!(siblings, vec![1, 33, 49]);

        let gmst = GlobalMst::new(GlobalConfig {
            num_of_tokens: 100,
            num_of_batches: 8,
            batch_size: 8,
            recursion_branchout_num: 4,
        });

        let global_index = 0;

        let siblings = get_mst_siblings_index(global_index, &gmst.cfg);
        assert_eq!(siblings, vec![1, 65, 97]);

        let gmst = GlobalMst::new(GlobalConfig {
            num_of_tokens: 100,
            num_of_batches: 6,
            batch_size: 8,
            recursion_branchout_num: 4,
        });

        let global_index = 0;

        let siblings = get_mst_siblings_index(global_index, &gmst.cfg);
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

        let siblings = get_recursive_siblings_index(global_index, &gmst.cfg);

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

        let siblings = get_recursive_siblings_index(global_index, &gmst.cfg);
        assert_eq!(
            siblings,
            vec![
                RecursiveIndex { left_indexes: vec![], right_indexes: vec![441, 442, 443] },
                RecursiveIndex { left_indexes: vec![456], right_indexes: vec![458, 459] },
                RecursiveIndex { left_indexes: vec![460], right_indexes: vec![462, 463] }
            ]
        );

        let gmst = GlobalMst::new(GlobalConfig {
            num_of_tokens: 10,
            num_of_batches: 6,
            batch_size: 4,
            recursion_branchout_num: 4,
        });

        let global_index = 20;

        let siblings = get_recursive_siblings_index(global_index, &gmst.cfg);
        assert_eq!(
            siblings,
            vec![
                RecursiveIndex { left_indexes: vec![40], right_indexes: vec![42, 43] },
                RecursiveIndex { left_indexes: vec![44], right_indexes: vec![46, 47] },
            ]
        );
    }

    #[test]
    pub fn test_get_new_merkle_index_from_user_index() {
        let gmst = GlobalMst::new(GlobalConfig {
            num_of_tokens: 100,
            num_of_batches: 15,
            batch_size: 4,
            recursion_branchout_num: 4,
        });

        let global_index = 0;

        let merkle_proof_indexes = MerkleProofIndex::new_from_user_index(global_index, &gmst.cfg);

        assert_eq!(
            merkle_proof_indexes,
            MerkleProofIndex {
                sum_tree_siblings: vec![1, 61],
                recursive_tree_siblings: vec![
                    RecursiveIndex { left_indexes: vec![], right_indexes: vec![91, 92, 93] },
                    RecursiveIndex { left_indexes: vec![], right_indexes: vec![107, 108, 109] }
                ],
            }
        );
    }

    #[test]
    pub fn test_verify_merkle_proof() {
        let _gmst = GlobalMst::new(GlobalConfig {
            num_of_tokens: 3,
            num_of_batches: 4,
            batch_size: 2,
            recursion_branchout_num: 4,
        });

        let sum_tree_siblings = vec![HashOut::from_vec(
            vec![
                7609058119952049295,
                8895839458156070742,
                1052773619972611009,
                6038312163525827182,
            ]
            .iter()
            .map(|x| F::from_canonical_u64(*x))
            .collect::<Vec<F>>(),
        )];

        let recursive_tree_siblings = vec![RecursiveHashes {
            left_hashes: vec![],
            right_hashes: vec![
                HashOut::from_vec(
                    vec![
                        15026394135096265436,
                        13313300609834454638,
                        10151802728958521275,
                        6200471959130767555,
                    ]
                    .iter()
                    .map(|x| F::from_canonical_u64(*x))
                    .collect::<Vec<F>>(),
                ),
                HashOut::from_vec(
                    vec![
                        2010803994799996791,
                        568450490466247075,
                        18209684900543488748,
                        7678193912819861368,
                    ]
                    .iter()
                    .map(|x| F::from_canonical_u64(*x))
                    .collect::<Vec<F>>(),
                ),
                HashOut::from_vec(
                    vec![
                        13089029781628355232,
                        10704046654659337561,
                        15794212269117984095,
                        15948192230150472783,
                    ]
                    .iter()
                    .map(|x| F::from_canonical_u64(*x))
                    .collect::<Vec<F>>(),
                ),
            ],
        }];

        let merkle_proof = MerkleProof { sum_tree_siblings, recursive_tree_siblings, index: 0 };

        let root = HashOut::from_vec(
            vec![
                10628303359772907103,
                7478459528589413745,
                12007196562137971174,
                2652030368197917032,
            ]
            .iter()
            .map(|x| F::from_canonical_u64(*x))
            .collect::<Vec<F>>(),
        );

        let equity = vec![3, 3, 3].iter().map(|x| F::from_canonical_u32(*x)).collect_vec();
        let debt = vec![1, 1, 1].iter().map(|x| F::from_canonical_u32(*x)).collect_vec();

        let res = merkle_proof.verify_merkle_proof(
            &Account {
                id: "320b5ea99e653bc2b593db4130d10a4efd3a0b4cc2e1a6672b678d71dfbd33ad".to_string(),
                equity: equity.clone(),
                debt: debt.clone(),
            },
            root,
        );

        res.unwrap();
    }

    // THIS IS THE TEST DATA FOR VERIFY
    // #[test]
    // pub fn poseidon_hash() {
    //     let equity = vec![3,3,3,].iter().map(|x| F::from_canonical_u32(*x)).collect_vec();
    //     let debt = vec![1,1,1,].iter().map(|x| F::from_canonical_u32(*x)).collect_vec();

    //     let accounts = vec![
    //         Account{
    //             id: "320b5ea99e653bc2b593db4130d10a4efd3a0b4cc2e1a6672b678d71dfbd33ad".to_string(),
    //             equity: equity.clone(),
    //             debt: debt.clone(),
    //         },
    //         Account{
    //             id: "320b5ea99e653bc2b593db4130d10a4efd3a0b4cc2e1a6672b678d71dfbd33ac".to_string(),
    //             equity: equity.clone(),
    //             debt: debt.clone(),
    //         },
    //         Account{
    //             id: "320b5ea99e653bc2b593db4130d10a4efd3a0b4cc2e1a6672b678d71dfbd33ab".to_string(),
    //             equity: equity.clone(),
    //             debt: debt.clone(),
    //         },
    //         Account{
    //             id: "320b5ea99e653bc2b593db4130d10a4efd3a0b4cc2e1a6672b678d71dfbd33aa".to_string(),
    //             equity: equity.clone(),
    //             debt: debt.clone(),
    //         },
    //         Account{
    //             id: "320b5ea99e653bc2b593db4130d10a4efd3a0b4cc2e1a6672b678d71dfbd33a1".to_string(),
    //             equity: equity.clone(),
    //             debt: debt.clone(),
    //         },
    //         Account{
    //             id: "320b5ea99e653bc2b593db4130d10a4efd3a0b4cc2e1a6672b678d71dfbd33a2".to_string(),
    //             equity: equity.clone(),
    //             debt: debt.clone(),
    //         },
    //         Account{
    //             id: "320b5ea99e653bc2b593db4130d10a4efd3a0b4cc2e1a6672b678d71dfbd33a3".to_string(),
    //             equity: equity.clone(),
    //             debt: debt.clone(),
    //         },
    //         Account{
    //             id: "320b5ea99e653bc2b593db4130d10a4efd3a0b4cc2e1a6672b678d71dfbd33a4".to_string(),
    //             equity: equity.clone(),
    //             debt: debt.clone(),
    //         }
    //     ];

    //     let msts: Vec<MerkleSumTree> = accounts
    //         .chunks(2)
    //         .map(|account_batch| MerkleSumTree::new_tree_from_accounts(&account_batch.to_vec()))
    //         .collect();

    //     let mst_hashes = msts.iter().map(|x| x.merkle_sum_tree.iter().map(|y| y.hash).collect_vec()).collect_vec();
    //     println!("msts:{:?}", mst_hashes);
    //     let inputs = vec![
    //         HashOut::from_vec(
    //             vec![
    //                 8699257539652901730,
    //                 12847577670763395377,
    //                 14540605839220144846,
    //                 1921995570040415498,
    //             ]
    //             .iter()
    //             .map(|x| F::from_canonical_u64(*x))
    //             .collect::<Vec<F>>(),
    //         ),
    //         HashOut::from_vec(
    //             vec![
    //                 15026394135096265436,
    //                 13313300609834454638,
    //                 10151802728958521275,
    //                 6200471959130767555,
    //             ]
    //             .iter()
    //             .map(|x| F::from_canonical_u64(*x))
    //             .collect::<Vec<F>>(),
    //         ),
    //         HashOut::from_vec(
    //             vec![
    //                 2010803994799996791,
    //                 568450490466247075,
    //                 18209684900543488748,
    //                 7678193912819861368,
    //             ]
    //             .iter()
    //             .map(|x| F::from_canonical_u64(*x))
    //             .collect::<Vec<F>>(),
    //         ),
    //         HashOut::from_vec(
    //             vec![
    //                 13089029781628355232,
    //                 10704046654659337561,
    //                 15794212269117984095,
    //                 15948192230150472783,
    //             ]
    //             .iter()
    //             .map(|x| F::from_canonical_u64(*x))
    //             .collect::<Vec<F>>(),
    //         ),
    //     ];

    //     let hash = PoseidonHash::hash_no_pad(
    //         inputs.iter().map(|x| x.elements).flatten().collect_vec().as_slice(),
    //     );
    //     println!("Hash: {:?}", hash);
    // }
}
