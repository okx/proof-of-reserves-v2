use rayon::prelude::*;

use std::{str::FromStr, sync::RwLock};
use tracing::debug;
use zk_por_core::{
    account::Account,
    config::ProverConfig,
    global::{GlobalConfig, GlobalMst, GLOBAL_MST},
    merkle_sum_tree::MerkleSumTree,
    parser::{FilesCfg, FilesParser},
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

    let ret = GLOBAL_MST.set(RwLock::new(GlobalMst::new(GlobalConfig {
        num_of_tokens: cfg.prover.num_of_tokens,
        num_of_batches: parser.total_num_of_batches,
        batch_size: parser.cfg.batch_size,
        hyper_tree_size: cfg.prover.hyper_tree_size,
    })));
    match ret {
        Ok(_) => (),
        Err(_) => {
            panic!("set global mst error");
        }
    }

    let batch_size = parser.cfg.batch_size;

    let mut offset = 0;
    while offset < parser.total_num_of_users {
        let num_cpus = num_cpus::get();
        let mut batch_accts = Vec::new();
        // TODO: we can read multile docs concurrently
        for _ in 0..num_cpus {
            if offset < parser.total_num_of_users {
                let accounts = parser.read_n_accounts(offset, batch_size);
                let acct_len = accounts.len();
                batch_accts.push((offset / batch_size, accounts));
                offset += acct_len;
            }
        }

        let _: Vec<()> = batch_accts
            .par_iter_mut()
            .map(|(chunk_idx, chunk)| {
                debug!("chunk_idx: {:}, chunk data: {:?}", chunk_idx, chunk.len());
                if chunk.len() < batch_size {
                    chunk.resize(batch_size, Account::get_empty_account(parser.cfg.num_of_tokens));
                };
                let mst = MerkleSumTree::new_tree_from_accounts(&chunk.to_vec());

                let global_mst = GLOBAL_MST.get().unwrap();
                let mut _g = global_mst.write().expect("unable to get a lock");
                for i in 0..batch_size * 2 - 1 {
                    _g.set_batch_hash(*chunk_idx, i, mst.merkle_sum_tree[i].hash);
                }
                drop(_g);
            })
            .collect();
    }
}
