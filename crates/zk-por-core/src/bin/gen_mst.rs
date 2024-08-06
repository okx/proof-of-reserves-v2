use plonky2::{hash::hash_types::HashOut, util::log2_strict};
use rayon::prelude::*;

use std::{ops::Div, str::FromStr, sync::RwLock};
use tracing::debug;
use zk_por_core::{
    config::ProverConfig,
    merkle_sum_tree::MerkleSumTree,
    parser::{FilesCfg, FilesParser},
    util::{get_node_level, get_recursive_hash_nums},
    GlobalConfig, GLOBAL_CONFIG, GLOBAL_MST,
};
use zk_por_tracing::{init_tracing, TraceConfig};

/// currently, we assume the first n-1 files contain users of number that is a multiple of batch_size;
/// the last file might not be an exact multiple;
/// we also assume the multiple is same for the first n-1 files
fn main() {
    let cfg = ProverConfig::try_new().unwrap();
    let trace_cfg: TraceConfig = cfg.log.into();
    init_tracing(trace_cfg);

    let mut parser = FilesParser::new(FilesCfg {
        dir: std::path::PathBuf::from_str(&cfg.prover.user_data_path).unwrap(),
        batch_size: cfg.prover.batch_size,
        num_of_tokens: cfg.prover.num_of_tokens,
    });
    parser.log_state();

    GLOBAL_CONFIG
        .set(GlobalConfig {
            num_of_tokens: cfg.prover.num_of_tokens,
            num_of_batches: parser.total_num_of_batches,
        })
        .unwrap();
    let vec_size = parser.total_num_of_batches * (2 * parser.cfg.batch_size - 1)
        + get_recursive_hash_nums(parser.total_num_of_batches, cfg.prover.hyper_tree_size);
    debug!("global mst size: {:?}", vec_size);
    GLOBAL_MST.set(RwLock::new(vec![HashOut::default(); vec_size])).unwrap();

    let batch_size = parser.cfg.batch_size;

    let mut offset = 0;
    while offset < parser.total_num_of_users {
        let num_cpus = num_cpus::get();
        let mut batch_accts = Vec::new();
        // TODO: we can read multile docs concurrently
        for _ in (0..num_cpus) {
            if offset < parser.total_num_of_users {
                let account = parser.read_n_accounts(offset, batch_size);
                let acct_len = account.len();
                batch_accts.push((offset / batch_size, account));
                offset = offset + acct_len;
            }
        }

        let _: Vec<()> = batch_accts
            .into_par_iter()
            .map(|(chunk_idx, chunk)| {
                debug!("chunk_idx: {:}, chunk data: {:?}", chunk_idx, chunk.len());
                let mst = MerkleSumTree::new_tree_from_accounts(&chunk.to_vec(), batch_size);
                let tree_depth = log2_strict(batch_size);

                let global_cfg = GLOBAL_CONFIG.get().unwrap();
                let global_mst = GLOBAL_MST.get().unwrap();
                let mut _g = global_mst.write().expect("unable to get a lock");
                for i in 0..batch_size * 2 - 1 {
                    let batch_tree_level = get_node_level(batch_size, i);
                    let level_from_bottom = tree_depth - batch_tree_level;

                    let global_tree_vertical_offset = 2 * batch_size * global_cfg.num_of_batches
                        - (2 * batch_size * global_cfg.num_of_batches).div(1 << level_from_bottom);

                    let level_node_counts = batch_size.div(1 << level_from_bottom);
                    let global_inter_tree_horizontal_offset = level_node_counts * chunk_idx;
                    let intra_tree_horizontal_offset =
                        i - (2 * batch_size - 2 * batch_size.div(1 << level_from_bottom));

                    _g[global_tree_vertical_offset
                        + global_inter_tree_horizontal_offset
                        + intra_tree_horizontal_offset] = mst.merkle_sum_tree[i].hash;
                }
                drop(_g);
            })
            .collect();
    }
}
