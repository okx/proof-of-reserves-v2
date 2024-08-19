use std::{fs::File, io::Write, str::FromStr};

use serde_json::json;
use zk_por_core::{
    config::ProverConfig,
    database::{DataBase, DbOption},
    error::PoRError,
    global::GlobalConfig,
    merkle_proof::MerkleProof,
    parser::{AccountParser, FileAccountReader, FileManager, FilesCfg},
};

use crate::constant::RECURSION_BRANCHOUT_NUM;

pub fn get_merkle_proof(
    user_id: String,
    cfg: ProverConfig,
    output_path: String,
) -> Result<(), PoRError> {
    let database = DataBase::new(DbOption {
        user_map_dir: cfg.db.level_db_user_path.to_string(),
        gmst_dir: cfg.db.level_db_gmst_path.to_string(),
    });

    let batch_size = cfg.prover.batch_size as usize;
    let token_num = cfg.prover.num_of_tokens as usize;

    // the path to dump the final generated proof
    let file_manager = FileManager {};
    let account_parser = FileAccountReader::new(
        FilesCfg {
            dir: std::path::PathBuf::from_str(&cfg.prover.user_data_path).unwrap(),
            batch_size: cfg.prover.batch_size,
            num_of_tokens: cfg.prover.num_of_tokens,
        },
        &file_manager,
    );
    account_parser.log_state();
    // let mut account_parser: Box<dyn AccountParser> = Box::new(parser);

    let batch_num = account_parser.total_num_of_users().div_ceil(batch_size);

    let global_cfg = GlobalConfig {
        num_of_tokens: token_num,
        num_of_batches: batch_num,
        batch_size: batch_size,
        recursion_branchout_num: RECURSION_BRANCHOUT_NUM,
    };

    let merkle_proof = MerkleProof::new_from_user_id(user_id, &database, &global_cfg).expect("Unable to generate merkle proof");
    
    let mut file = File::create(output_path.clone())
        .expect(format!("fail to create proof file at {:#?}", output_path).as_str());
    file.write_all(json!(merkle_proof).to_string().as_bytes())
        .expect("fail to write proof to file");

    Ok(())
}
