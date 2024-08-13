use plonky2::util::log2_strict;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

use crate::global::GlobalMst;

#[derive(Debug, Clone)]
pub struct MerkleProofIndex {
    pub sum_tree_siblings: Vec<usize>,
    pub recursive_tree_siblings: Vec<Vec<usize>>,
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
) -> Vec<Vec<usize>> {
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

    let layers = log2_strict(global_mst.cfg.num_of_batches.next_power_of_two())
        / log2_strict(global_mst.cfg.recursion_branchout_num);

    for i in 0..layers {
        let mut layer = Vec::new();
        if i == 0 {
            for j in 0..global_mst.cfg.recursion_branchout_num {
                if j != recursive_offset {
                    let index = first_mst_root_idx
                        + (global_mst.cfg.recursion_branchout_num * recursive_idx)
                        + j;
                    layer.push(index);
                }
            }
        } else {
            for j in 0..global_mst.cfg.recursion_branchout_num {
                if j != recursive_offset {
                    let index = global_mst.get_recursive_global_index(
                        i,
                        recursive_idx * global_mst.cfg.recursion_branchout_num + j,
                    );
                    layer.push(index);
                }
            }
        }

        siblings.push(layer);

        recursive_idx = recursive_idx / global_mst.cfg.recursion_branchout_num;
        recursive_offset = recursive_idx % global_mst.cfg.recursion_branchout_num;
    }

    siblings
}

#[cfg(test)]
pub mod test {
    use crate::{
        global::{GlobalConfig, GlobalMst},
        merkle_proof::get_recursive_siblings_index,
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
        assert_eq!(siblings, vec![vec![91, 92, 93], vec![107, 108, 109]]);

        let gmst = GlobalMst::new(GlobalConfig {
            num_of_tokens: 100,
            num_of_batches: 30,
            batch_size: 8,
            recursion_branchout_num: 4,
        });

        let global_index = 163;

        let siblings = get_recursive_siblings_index(global_index, &gmst);
        assert_eq!(siblings, vec![vec![441, 442, 443], vec![456, 458, 459]]);
    }
}
