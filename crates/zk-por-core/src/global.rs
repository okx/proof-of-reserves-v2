use crate::{
    types::F,
    util::{get_node_level, get_recursive_hash_nums},
};
use once_cell::sync::OnceCell;
use plonky2::{hash::hash_types::HashOut, util::log2_strict};
use std::{ops::Div, sync::RwLock};
use tracing::debug;
#[derive(Debug)]
pub struct GlobalConfig {
    pub num_of_tokens: usize,
    pub num_of_batches: usize,
    pub batch_size: usize,
    pub hyper_tree_size: usize,
}

pub static GLOBAL_MST: OnceCell<RwLock<GlobalMst>> = OnceCell::new();

pub struct GlobalMst {
    inner: Vec<HashOut<F>>,
    pub cfg: GlobalConfig,
}

impl GlobalMst {
    pub fn new(cfg: GlobalConfig) -> Self {
        let vec_size = cfg.num_of_batches * (2 * cfg.batch_size - 1)
            + get_recursive_hash_nums(cfg.num_of_batches, cfg.hyper_tree_size);
        let mst_vec = vec![HashOut::default(); vec_size];
        Self { inner: mst_vec, cfg }
    }

    /// `batch_idx`: index indicating the batch index
    /// `i`: the sub batch tree index; e.g the batch tree is of size 1<<10; i \in [0, 2*batch_size)
    pub fn set_batch_hash(&mut self, batch_idx: usize, i: usize, hash: HashOut<F>) {
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

        self.inner[global_tree_vertical_offset
            + global_inter_tree_horizontal_offset
            + intra_tree_horizontal_offset] = hash;
    }

    pub fn get_batch_root_hash(&self, batch_idx: usize) -> HashOut<F> {
        debug!("get batch root hash, batch_idx: {:?}", batch_idx);
        assert!(batch_idx < self.cfg.num_of_batches);
        todo!()
    }

    pub fn set_recursive_hash(&mut self, recursive_level: usize, index: usize, hash: HashOut<F>) {
        debug!(
            "set_recursive_hash, recursive_level: {:?}, index: {:?}, hash: {:?}",
            recursive_level, index, hash
        );
        todo!()
    }
}

#[cfg(test)]
mod test {
    // TODO: can add a test case to assert that the generated root is identical to the one in generated in plonky2 proof.
}
