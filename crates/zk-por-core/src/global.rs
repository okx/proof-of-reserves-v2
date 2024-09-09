use crate::{
    circuit_utils::recursive_levels,
    database::PoRDB,
    merkle_sum_prover::utils::hash_2_subhashes,
    recursive_prover::prover::hash_n_subhashes,
    types::{D, F},
    util::{get_node_level, pad_to_multiple_of},
};
use once_cell::sync::OnceCell;
use plonky2::{hash::hash_types::HashOut, util::log2_strict};
use std::{ops::Div, sync::RwLock};
use tracing::debug;

#[derive(Debug, Clone, Copy)]
pub struct GlobalConfig {
    pub num_of_tokens: usize,
    pub num_of_batches: usize,
    pub batch_size: usize, // num of accounts witin one batch
    pub recursion_branchout_num: usize,
}

pub static GLOBAL_MST: OnceCell<RwLock<GlobalMst>> = OnceCell::new();

pub struct GlobalMst {
    pub inner: Vec<HashOut<F>>,
    top_recursion_level: usize,
    pub cfg: GlobalConfig,
}

impl GlobalMst {
    pub fn new(cfg: GlobalConfig) -> Self {
        let top_level = recursive_levels(cfg.num_of_batches, cfg.recursion_branchout_num);

        let mst_vec = vec![HashOut::default(); 0]; // will resize later
        let mut mst = Self { inner: mst_vec, top_recursion_level: top_level, cfg: cfg };
        // the number of hash is one smaller to the index of the root node of the last recursion level.
        let root_node_idx = GlobalMst::get_recursive_global_index(&cfg, top_level, 0);
        let tree_size = root_node_idx + 1;
        mst.inner.resize(tree_size, HashOut::default());
        mst
    }

    pub fn get_tree_length(&self) -> usize {
        self.inner.len()
    }

    pub fn get_num_of_leaves(cfg: &GlobalConfig) -> usize {
        cfg.batch_size * cfg.num_of_batches
    }

    pub fn get_nodes(&self, range: std::ops::Range<usize>) -> &[HashOut<F>] {
        &self.inner[range]
    }

    pub fn get_root(&self) -> Option<&HashOut<F>> {
        self.inner.last()
    }

    /// convert a mst node inner index to global index in gmst.
    /// For a mst, the inner index is level-by-level, e.g.,
    ///       14
    ///   12      13
    ///  8-9,   10-11
    /// 0 - 3,  4 - 7
    pub fn get_batch_tree_global_index(
        cfg: &GlobalConfig,
        batch_idx: usize,
        inner_tree_idx: usize,
    ) -> usize {
        let batch_size = cfg.batch_size;
        let tree_depth = log2_strict(batch_size);
        let batch_tree_level = get_node_level(batch_size, inner_tree_idx);

        let level_from_bottom = tree_depth - batch_tree_level;

        let numeritor = 2 * batch_size * cfg.num_of_batches;
        let global_tree_vertical_offset = numeritor - numeritor.div(1 << level_from_bottom); // the gmst idx of the first node at {level_from_bottom} level

        let level_node_counts = batch_size.div(1 << level_from_bottom);
        let global_inter_tree_horizontal_offset = level_node_counts * (batch_idx); // the number of preceding nodes at {level_from_bottom} level in the preceding mst.

        let intra_tree_horizontal_offset =
            inner_tree_idx - (2 * batch_size - 2 * batch_size.div(1 << level_from_bottom));
        // the number of preceding nodes at {level_from_bottom} level in the current mst.

        let index = global_tree_vertical_offset
            + global_inter_tree_horizontal_offset
            + intra_tree_horizontal_offset;
        index
    }

    // mst root node at level 0,
    pub fn get_recursive_global_index(
        cfg: &GlobalConfig,
        recursive_level: usize,
        inner_level_idx: usize,
    ) -> usize {
        let mst_node_num = 2 * cfg.batch_size - 1;
        let batch_num = cfg.num_of_batches;
        let branchout_num = cfg.recursion_branchout_num;
        if recursive_level == 0 {
            // level of merkle sum tree root
            if inner_level_idx < cfg.num_of_batches {
                // the global index of the root of the batch tree
                let mst_root_idx = mst_node_num - 1;
                return GlobalMst::get_batch_tree_global_index(cfg, inner_level_idx, mst_root_idx);
            } else {
                return batch_num * mst_node_num + (inner_level_idx - cfg.num_of_batches);
            }
        }

        // pad num_of_batches to be multiple of recursion_branchout_num.
        let pad_num = if batch_num % branchout_num == 0 {
            0
        } else {
            branchout_num - batch_num % branchout_num
        };

        let mut last_level_node_num = batch_num + pad_num;
        assert_eq!(0, last_level_node_num % branchout_num);

        let mut recursive_offset = batch_num * mst_node_num + pad_num;

        let mut level = recursive_level;
        while level > 1 {
            let mut this_level_node_num = last_level_node_num / cfg.recursion_branchout_num;
            this_level_node_num =
                pad_to_multiple_of(this_level_node_num, cfg.recursion_branchout_num);

            recursive_offset += this_level_node_num;

            last_level_node_num = this_level_node_num;
            level -= 1;
        }

        let global_recursive_index = recursive_offset + inner_level_idx;
        global_recursive_index
    }

    /// `batch_idx`: index indicating the batch index
    /// `i`: the sub batch tree index; e.g the batch tree is of size 1<<10; i \in [0, 2*batch_size)
    pub fn set_batch_hash(&mut self, batch_idx: usize, i: usize, hash: HashOut<F>) {
        let global_mst_idx = GlobalMst::get_batch_tree_global_index(&self.cfg, batch_idx, i);
        self.inner[global_mst_idx] = hash;
    }

    pub fn get_batch_root_hash(&self, batch_idx: usize) -> HashOut<F> {
        debug!("get batch root hash, batch_idx: {:?}", batch_idx);
        assert!(batch_idx < self.cfg.num_of_batches);
        let root_idx = GlobalMst::get_batch_tree_global_index(
            &self.cfg,
            batch_idx,
            2 * self.cfg.batch_size - 2,
        );
        self.inner[root_idx]
    }

    /// `recursive_level` count from bottom to top; recursive_level = 1 means the bottom layer; increase whilve moving to the top.
    pub fn set_recursive_hash(&mut self, recursive_level: usize, index: usize, hash: HashOut<F>) {
        let idx = GlobalMst::get_recursive_global_index(&self.cfg, recursive_level, index);
        tracing::debug!(
            "set_recursive_hash, recursive_level: {:?}, index: {:?}, hash: {:?}, idx: {:?}",
            recursive_level,
            index,
            hash,
            idx,
        );
        self.inner[idx] = hash;
    }

    pub fn is_integral(&self) -> bool {
        // we check all nodes are examined to ensure global_index-related functions are correct.
        let mut visited_global_idx = vec![false; self.inner.len()];
        let batch_num = self.cfg.num_of_batches;
        for tree_idx in 0..self.cfg.num_of_batches {
            let leaf_size = self.cfg.batch_size;
            let mst_size = 2 * leaf_size - 1;
            for inner_tree_idx in leaf_size..mst_size {
                let inner_left_child_idx = 2 * (inner_tree_idx - leaf_size);
                let inner_right_child_idx = 2 * (inner_tree_idx - leaf_size) + 1;

                let global_parent_idx =
                    GlobalMst::get_batch_tree_global_index(&self.cfg, tree_idx, inner_tree_idx);
                let global_left_child_idx = GlobalMst::get_batch_tree_global_index(
                    &self.cfg,
                    tree_idx,
                    inner_left_child_idx,
                );
                let global_right_child_idx = GlobalMst::get_batch_tree_global_index(
                    &self.cfg,
                    tree_idx,
                    inner_right_child_idx,
                );

                visited_global_idx[global_left_child_idx] = true;
                visited_global_idx[global_right_child_idx] = true;

                let expected_parent_hash = hash_2_subhashes::<F, D>(
                    &self.inner[global_left_child_idx],
                    &self.inner[global_right_child_idx],
                );
                if expected_parent_hash != self.inner[global_parent_idx] {
                    tracing::error!("Inconsistent hash at mst tree {}, global index (parent: {:?}, left child: {:?}, right child: {:?}), inner index (parent: {:?}, left child: {:?}, right child: {:?}), expected parent hash: {:?}, actual parent hash: {:?}", tree_idx, global_parent_idx, global_left_child_idx, global_right_child_idx,  inner_tree_idx, inner_left_child_idx, inner_right_child_idx, expected_parent_hash, self.inner[global_parent_idx]);
                    return false;
                }
            }
        }
        let branchout_num = self.cfg.recursion_branchout_num;
        let mut last_level_node_count = pad_to_multiple_of(batch_num, branchout_num);
        for level in 1..=self.top_recursion_level {
            let this_level_node_count = last_level_node_count / branchout_num;
            for inner_idx in 0..this_level_node_count {
                let inner_child_indexes = (0..branchout_num)
                    .map(|i| inner_idx * branchout_num + i)
                    .collect::<Vec<usize>>();
                let global_idx = GlobalMst::get_recursive_global_index(&self.cfg, level, inner_idx);
                let global_child_indexes = inner_child_indexes
                    .iter()
                    .map(|&i| {
                        let child_global_idx =
                            GlobalMst::get_recursive_global_index(&self.cfg, level - 1, i);
                        visited_global_idx[child_global_idx] = true;
                        child_global_idx
                    })
                    .collect::<Vec<usize>>();

                let children_hashes = global_child_indexes
                    .iter()
                    .map(|&i| self.inner[i])
                    .collect::<Vec<HashOut<F>>>();

                let expected_parent_hash = hash_n_subhashes::<F, D>(&children_hashes);

                if expected_parent_hash != self.inner[global_idx] {
                    tracing::error!("Inconsistent hash at recursive level {}, Global index: {:?}, global child indexes: {:?}, inner index: {:?}, child indexes {:?}, expected parent hash: {:?}, actual parent hash: {:?}. ", level, global_idx, global_child_indexes, inner_idx, inner_child_indexes, expected_parent_hash, self.inner[global_idx]);
                    return false;
                }
                last_level_node_count = pad_to_multiple_of(this_level_node_count, branchout_num);
            }
        }
        let global_root_idx =
            GlobalMst::get_recursive_global_index(&self.cfg, self.top_recursion_level, 0);
        visited_global_idx[global_root_idx] = true;

        visited_global_idx.iter().all(|&v| v)
    }

    pub fn persist(&self, db: &mut Box<dyn PoRDB>) {
        let length = self.get_tree_length();
        tracing::info!("start to persist gmst into db of size: {:?}", length);
        let chunk_size = 1 << 12;
        let mut i = 0;
        while i < length {
            let end = if i + chunk_size <= length { i + chunk_size } else { length };
            let nodes = self.get_nodes(i..end);
            let batches = (i..end)
                .into_iter()
                .enumerate()
                .map(|(chunk_idx, j)| ((j).try_into().unwrap(), nodes[chunk_idx]))
                .collect::<Vec<(i32, HashOut<F>)>>();
            db.add_batch_gmst_nodes(batches);
            i += chunk_size;
        }
    }
}

#[cfg(test)]
mod test {
    use super::GlobalMst;
    use crate::{
        account::gen_accounts_with_random_data,
        merkle_sum_tree::MerkleSumTree,
        recursive_prover::prover::hash_n_subhashes,
        types::{D, F},
        util::pad_to_multiple_of,
    };
    use plonky2::hash::hash_types::HashOut;
    use zk_por_tracing::{init_tracing, TraceConfig};

    #[test]
    fn test_index() {
        let gmst = GlobalMst::new(super::GlobalConfig {
            num_of_tokens: 22,
            num_of_batches: 6,
            batch_size: 8,
            recursion_branchout_num: 4,
        });
        let total_len = gmst.get_tree_length();

        /*
        L2:                                 96
        L1:         92            93                 94                95
        L0:     84,    85,    86,     87,        88,      89,       90e,  91e
                72-73, 74-75,  76-77,  78-79,    80-81,   82-83,
                48-51, 52-55,  56-59,   60-63,   64-67,   68-71
                0 - 7, 8 - 15, 16 - 23, 24 - 31, 32 - 39, 40 - 47
        */
        assert_eq!(total_len, 97);
        assert_eq!(gmst.top_recursion_level, 2);

        assert_eq!(GlobalMst::get_batch_tree_global_index(&gmst.cfg, 0, 1), 1);
        assert_eq!(GlobalMst::get_batch_tree_global_index(&gmst.cfg, 0, 14), 84);
        assert_eq!(GlobalMst::get_batch_tree_global_index(&gmst.cfg, 1, 1), 9);
        assert_eq!(GlobalMst::get_batch_tree_global_index(&gmst.cfg, 1, 14), 85);
        assert_eq!(GlobalMst::get_batch_tree_global_index(&gmst.cfg, 5, 7), 47);
        assert_eq!(GlobalMst::get_batch_tree_global_index(&gmst.cfg, 5, 14), 89);

        assert_eq!(GlobalMst::get_recursive_global_index(&gmst.cfg, 0, 7), 91);
        assert_eq!(GlobalMst::get_recursive_global_index(&gmst.cfg, 0, 1), 85);
        assert_eq!(GlobalMst::get_recursive_global_index(&gmst.cfg, 1, 0), 92);
        assert_eq!(GlobalMst::get_recursive_global_index(&gmst.cfg, 1, 1), 93);
        assert_eq!(GlobalMst::get_recursive_global_index(&gmst.cfg, 1, 2), 94);
        assert_eq!(GlobalMst::get_recursive_global_index(&gmst.cfg, 1, 3), 95);
        assert_eq!(GlobalMst::get_recursive_global_index(&gmst.cfg, 2, 0), 96);
    }

    #[test]
    fn test_integrity() {
        let cfg = TraceConfig {
            prefix: "zkpor".to_string(),
            dir: "logs".to_string(),
            level: tracing::Level::DEBUG,
            console: true,
            flame: false,
        };

        {
            init_tracing(cfg)
        };

        let mut gmst = GlobalMst::new(super::GlobalConfig {
            num_of_tokens: 22,
            num_of_batches: 6,
            batch_size: 8,
            recursion_branchout_num: 4,
        });

        assert!(!gmst.is_integral());
        let batch_num = gmst.cfg.num_of_batches;
        let batch_size = gmst.cfg.batch_size;
        let branchout_num = gmst.cfg.recursion_branchout_num;

        for batch_idx in 0..batch_num {
            let accounts = gen_accounts_with_random_data(batch_size, 1);
            let mst = MerkleSumTree::new_tree_from_accounts(&accounts);

            for i in 0..batch_size * 2 - 1 {
                gmst.set_batch_hash(batch_idx, i, mst.merkle_sum_tree[i].hash);
            }
        }

        let mut last_level_node_num = pad_to_multiple_of(batch_num, branchout_num);
        for level in 1..=gmst.top_recursion_level {
            let this_level_node_count = last_level_node_num / branchout_num;
            for inner_idx in 0..this_level_node_count {
                let children_hashes = (0..branchout_num)
                    .map(|i| {
                        let child_global_idx = GlobalMst::get_recursive_global_index(
                            &gmst.cfg,
                            level - 1,
                            inner_idx * branchout_num + i,
                        );
                        gmst.inner[child_global_idx]
                    })
                    .collect::<Vec<HashOut<F>>>();

                let expected_parent_hash = hash_n_subhashes::<F, D>(&children_hashes);
                gmst.set_recursive_hash(level, inner_idx, expected_parent_hash);
            }

            last_level_node_num = pad_to_multiple_of(this_level_node_count, branchout_num);
        }
        assert!(gmst.is_integral());
    }
}
