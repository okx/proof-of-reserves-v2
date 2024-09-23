use super::{constant::DEFAULT_BATCH_SIZE, prover::calculate_per_parse_account_num};
use plonky2_field::types::{Field, PrimeField64};

use std::str::FromStr;
use zk_por_core::{
    config::ProverConfig,
    error::PoRError,
    parser::{AccountParser, FileAccountReader, FileManager, FilesCfg},
    types::F,
};
use zk_por_tracing::{init_tracing, TraceConfig};

pub fn check_non_neg_user(cfg: ProverConfig) -> Result<(), PoRError> {
    let trace_cfg: TraceConfig = cfg.log.into();
    let _g = init_tracing(trace_cfg);

    let batch_size = cfg.prover.batch_size.unwrap_or(DEFAULT_BATCH_SIZE);
    let file_manager = FileManager {};
    let mut account_parser = FileAccountReader::new(
        FilesCfg {
            dir: std::path::PathBuf::from_str(&cfg.prover.user_data_path).unwrap(),
            batch_size: batch_size,
            tokens: cfg.prover.tokens.clone(),
        },
        &file_manager,
    );
    account_parser.log_state();
    let mut offset = 0;
    let batch_prove_threads_num = cfg.prover.batch_prove_threads_num;
    let per_parse_account_num =
        calculate_per_parse_account_num(batch_size, batch_prove_threads_num);

    let batch_num = account_parser.total_num_of_users().div_ceil(batch_size);
    let token_num = cfg.prover.tokens.len();
    let mut parse_num = 0;

    tracing::info!(
        "start to check {} accounts with {} tokens, {} batch size",
        account_parser.total_num_of_users(),
        token_num,
        batch_size,
    );

    while offset < account_parser.total_num_of_users() {
        let accounts = account_parser.read_n_accounts(offset, per_parse_account_num, &file_manager);
        let account_num = accounts.len();

        tracing::info!(
            "parse {} times, with number of accounts {}, number of batches {}",
            parse_num,
            account_num,
            batch_num,
        );
        parse_num += 1;
        tracing::info!("finish checking {} accounts", offset);
        offset += per_parse_account_num;

        for account in accounts {
            let equity_sum =
                account.equity.iter().fold(F::ZERO, |acc, x| acc + *x).to_canonical_u64();
            let debt_sum = account.debt.iter().fold(F::ZERO, |acc, x| acc + *x).to_canonical_u64();

            if equity_sum < debt_sum {
                tracing::error!(
                    "account {} has negative equity, the equity sum {}, the debt sum {}",
                    account.id,
                    equity_sum,
                    debt_sum
                );
                return Err(PoRError::InvalidUser);
            }
        }
    }
    Ok(())
}
