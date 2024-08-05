use plonky2::{hash::hash_types::HashOut, util::log2_strict};
use rayon::prelude::*;
use serde_json::error;
use std::{
    fs,
    ops::Div,
    path::{Path, PathBuf},
    str::FromStr,
    sync::RwLock,
};
use tracing::{debug, error, info, warn, Level};
use zk_por_core::{
    account::Account,
    config::ProverConfig,
    merkle_sum_tree::MerkleSumTree,
    parser::read_json_into_accounts_vec,
    util::{get_node_level, get_recursive_hash_nums},
    GlobalConfig, GLOBAL_CONFIG, GLOBAL_MST,
};
use zk_por_tracing::{init_tracing, TraceConfig};

fn list_json_files(dir: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut json_files = Vec::new();
    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                if let Some(extension) = path.extension() {
                    if extension == "json" {
                        json_files.push(path);
                    }
                }
            } else if path.is_dir() {
                json_files.extend(list_json_files(&path)?);
            }
        }
    }
    json_files.sort();
    Ok(json_files)
}

fn build_batch_mst() {}

/// `accounts` accounts within one doc; the last
/// `total_num_of_docs`: total number of docs;
fn process_accounts(
    accounts: &[Account],
    doc_index: usize,
    batch_size: u32,
    batch_numbers_per_doc: usize,
    num_of_tokens: usize,
    total_num_of_docs: usize,
) {
    let empty_acct = Account::get_empty_account(num_of_tokens);
}

/// currently, we assume the first n-1 files contain users of number that is a multiple of batch_size;
/// the last file might not be an exact multiple;
/// we also assume the multiple is same for the first n-1 files
fn main() {
    let cfg = ProverConfig::try_new().unwrap();
    let trace_cfg: TraceConfig = cfg.log.into();
    init_tracing(trace_cfg);
    let user_data_path = std::path::Path::new(&cfg.prover.user_data_path);
    if !user_data_path.exists() {
        panic!("dir: {:?} does not exist", user_data_path);
    }

    let json_files = list_json_files(&user_data_path);
    match json_files {
        Ok(docs) => {
            debug!("files: {:?}", docs);
            let doc_len = docs.len();
            if doc_len < 1 {
                warn!("no json files under the folder: {:?}", user_data_path);
                std::process::exit(0);
            } else {
                let batch_size = cfg.prover.batch_size;
                let mut last_doc_account_num = 0;
                for (doc_idx, file) in docs.clone().into_iter().enumerate() {
                    let mut accounts = read_json_into_accounts_vec(file.to_str().unwrap());
                    let accounts_len = accounts.len();
                    last_doc_account_num = accounts.len();

                    if doc_idx == 0 && doc_len > 1 {
                        assert_eq!(accounts_len % batch_size, 0);
                    }
                    if doc_idx > 0 && doc_idx < doc_len - 1 {
                        assert_eq!(accounts_len, last_doc_account_num);
                    }
                    if doc_idx == doc_len - 1 {
                        assert!(accounts_len <= last_doc_account_num);
                    }

                    if doc_idx == 0 {
                        // TODO: try find a way read number of entries per doc rather than loading the whole doc content; otherwise, the last doc will be read twice
                        let last_doc_accounts_len = if doc_len > 1 {
                            let acct =
                                read_json_into_accounts_vec(docs[doc_len - 1].to_str().unwrap());
                            acct.len()
                        } else {
                            accounts.len()
                        };
                        let total_num_of_users =
                            (doc_len - 1) * accounts_len + last_doc_accounts_len;

                        let num_of_batches = total_num_of_users.div_ceil(batch_size);
                        GLOBAL_CONFIG
                            .set(GlobalConfig {
                                batch_size: batch_size,
                                doc_num: doc_len,
                                num_of_batches_per_doc: accounts_len.div_ceil(batch_size),
                                total_num_of_users,
                                num_of_tokens: cfg.prover.num_of_tokens,
                                num_of_batches,
                            })
                            .unwrap();
                        let vec_size = num_of_batches * (2 * batch_size - 1)
                            + get_recursive_hash_nums(num_of_batches, cfg.prover.hyper_tree_size);
                        debug!("global mst size: {:?}", vec_size);
                        GLOBAL_MST.set(RwLock::new(vec![HashOut::default(); vec_size])).unwrap();
                    }
                    let global_cfg = GLOBAL_CONFIG.get().unwrap();
                    info!("global_cfg: {:?}", global_cfg);

                    let chunks: Vec<&mut [Account]> = accounts.chunks_mut(batch_size).collect(); // Collect mutable slices into a Vec

                    let _: Vec<()> = chunks
                        .into_par_iter()

                        .enumerate()
                        .map(|(chunk_idx, chunk)| {

                            debug!("chunk_idx: {:}, chunk data: {:?}",chunk_idx, chunk.len());
                            let mst =
                                MerkleSumTree::new_tree_from_accounts(&chunk.to_vec(), batch_size);
                            let tree_depth = log2_strict(batch_size);

                            let global_cfg = GLOBAL_CONFIG.get().unwrap();
                            let global_mst = GLOBAL_MST.get().unwrap();
                            let mut _g = global_mst.write().expect("unable to get a lock");
                            for i in 0..batch_size * 2 - 1 {
                                let batch_tree_level = get_node_level(batch_size, i);
                                let level_from_bottom = tree_depth - batch_tree_level;

                                let global_tree_vertical_offset = 2 * batch_size * global_cfg.num_of_batches- 2 * batch_size* global_cfg.num_of_batches/(1 << level_from_bottom);
    

                                let level_node_counts = batch_size.div(1<< level_from_bottom);
                                let global_inter_tree_horizontal_offset = level_node_counts * chunk_idx;
                                let intra_tree_horizontal_offset = i- (2*batch_size - 2*batch_size.div(1 << level_from_bottom));
                                debug!("chunk_idx: {:}, i: {:?}, global_tree_vertical_offset: {}, global_idx: {:?}, level_from_bottom: {:?}",  chunk_idx, i,
                                global_tree_vertical_offset,
                                global_tree_vertical_offset+global_inter_tree_horizontal_offset+intra_tree_horizontal_offset,level_from_bottom);
                                _g[global_tree_vertical_offset+global_inter_tree_horizontal_offset+intra_tree_horizontal_offset] = mst.merkle_sum_tree[i].hash;
                            }
                            drop(_g);

          
                        })
                        .collect();
                }
            }
        }
        Err(e) => panic!("list json files err: {:?}", e),
    }
}
