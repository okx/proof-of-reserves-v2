use crate::{
    types::F,
    util::{get_node_level, pad_to_multiple_of},
};
use once_cell::sync::OnceCell;
use plonky2::{hash::hash_types::HashOut, util::log2_strict};
use std::{ops::Div, sync::RwLock};
use tracing::debug;
#[derive(Debug)]
pub struct GlobalConfig {
    pub num_of_tokens: usize,
    pub num_of_batches: usize,
    pub batch_size: usize, // num of accounts witin one batch
    pub recursion_branchout_num: usize,
}

pub static GLOBAL_MST: OnceCell<RwLock<GlobalMst>> = OnceCell::new();

pub struct GlobalMst {
    inner: Vec<HashOut<F>>,
    pub cfg: GlobalConfig,
}

impl GlobalMst {
    pub fn new(cfg: GlobalConfig) -> Self {
        let mut level = 0;
        let mut n = cfg.num_of_batches;
        while n > 0 {
            level += 1;
            n = n / cfg.recursion_branchout_num;
        }

        let mst_vec = vec![HashOut::default(); cfg.num_of_batches];
        let mut mst = Self { inner: mst_vec, cfg: cfg };
        // the number of hash is equal to the index of the root node. 
        let root_node_idx = mst.get_recursive_global_index(level, 0);
        mst.inner.resize(root_node_idx, HashOut::default());
        mst
    }

    pub fn any_empty_node(&self) -> bool {
        self.inner.iter().any(|x| *x == HashOut::default())
    }

    #[allow(dead_code)]
    fn get_tree_length(&self) -> usize {
        self.inner.len()
    }

    pub fn get_batch_tree_global_index(&self, batch_idx: usize, i: usize) -> usize {
        let batch_size = self.cfg.batch_size;
        let tree_depth = log2_strict(batch_size);
        let batch_tree_level = get_node_level(batch_size, i);

        let level_from_bottom = tree_depth - batch_tree_level;

        let numeritor = 2 * batch_size * self.cfg.num_of_batches;
        let global_tree_vertical_offset = numeritor - numeritor.div(1 << level_from_bottom);

        let level_node_counts = batch_size.div(1 << level_from_bottom);
        let global_inter_tree_horizontal_offset = level_node_counts * (batch_idx);
        let intra_tree_horizontal_offset =
            i - (2 * batch_size - 2 * batch_size.div(1 << level_from_bottom));
        let index = global_tree_vertical_offset
            + global_inter_tree_horizontal_offset
            + intra_tree_horizontal_offset;
        index
    }

    pub fn get_recursive_global_index(&self, recursive_level: u32, index: usize) -> usize {
        // pad num_of_batches to be multiple of recursion_branchout_num. 
        let pad_num = if self.cfg.num_of_batches % self.cfg.recursion_branchout_num == 0 {
            0
        } else {
            self.cfg.recursion_branchout_num - self.cfg.num_of_batches % self.cfg.recursion_branchout_num
        };

        let mut last_level_node_num = self.cfg.num_of_batches + pad_num;
        assert_eq!(0, last_level_node_num % self.cfg.recursion_branchout_num);

        let mst_node_num = 2 * self.cfg.batch_size - 1;
        let mut recursive_offset = self.cfg.num_of_batches * mst_node_num + pad_num;

        let mut level = recursive_level;
        while level > 1 {
            let mut this_level_node_num = last_level_node_num / self.cfg.recursion_branchout_num;
            this_level_node_num = pad_to_multiple_of(this_level_node_num, self.cfg.recursion_branchout_num);

            recursive_offset += this_level_node_num;

            last_level_node_num = this_level_node_num;
            level -= 1;
        }

        let global_recursive_index = recursive_offset + index;
        global_recursive_index
    }

    /// `batch_idx`: index indicating the batch index
    /// `i`: the sub batch tree index; e.g the batch tree is of size 1<<10; i \in [0, 2*batch_size)
    pub fn set_batch_hash(&mut self, batch_idx: usize, i: usize, hash: HashOut<F>) {
        let global_mst_idx = self.get_batch_tree_global_index(batch_idx, i);
        self.inner[global_mst_idx] = hash;
    }

    pub fn get_batch_root_hash(&self, batch_idx: usize) -> HashOut<F> {
        debug!("get batch root hash, batch_idx: {:?}", batch_idx);
        assert!(batch_idx < self.cfg.num_of_batches);
        let root_idx = self.get_batch_tree_global_index(batch_idx, 2 * self.cfg.batch_size - 2);
        self.inner[root_idx]
    }

    /// `recursive_level` count from bottom to top; recursive_level = 1 means the bottom layer; increase whilve moving to the top.
    pub fn set_recursive_hash(&mut self, recursive_level: u32, index: usize, hash: HashOut<F>) {
        debug!(
            "set_recursive_hash, recursive_level: {:?}, index: {:?}, hash: {:?}",
            recursive_level, index, hash
        );
        let idx = self.get_recursive_global_index(recursive_level, index);
        self.inner[idx] = hash;
    }
}

#[cfg(test)]
mod test {
    // TODO: can add a test case to assert that the generated root is identical to the one in generated in plonky2 proof.

    use super::GlobalMst;

    #[test]
    fn test_global_mst() {
        let gmst = GlobalMst::new(super::GlobalConfig {
            num_of_tokens: 22,
            num_of_batches: 6,
            batch_size: 8,
            recursion_branchout_num: 4,
        });
        let total_len = gmst.get_tree_length();
        assert_eq!(total_len, 95);

        assert_eq!(gmst.get_batch_tree_global_index(0, 1), 1);
        assert_eq!(gmst.get_batch_tree_global_index(0, 14), 84);
        assert_eq!(gmst.get_batch_tree_global_index(1, 1), 9);
        assert_eq!(gmst.get_batch_tree_global_index(1, 14), 85);
        assert_eq!(gmst.get_batch_tree_global_index(5, 7), 47);
        assert_eq!(gmst.get_batch_tree_global_index(5, 14), 89);

        assert_eq!(gmst.get_recursive_global_index(1, 0), 92);
        assert_eq!(gmst.get_recursive_global_index(1, 1), 93);
        assert_eq!(gmst.get_recursive_global_index(1, 2), 94);
        assert_eq!(gmst.get_recursive_global_index(1, 3), 95);
        assert_eq!(gmst.get_recursive_global_index(2, 0), 96);
    }
}
