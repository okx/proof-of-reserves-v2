use plonky2::{hash::hash_types::HashOut, util::log2_strict};

use crate::{
    account::Account,
    merkle_sum_prover::utils::hash_2_subhashes,
    types::{D, F},
};

use plonky2_field::types::Field;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MerkleSumNode {
    pub sum_equity: F,
    pub sum_debt: F,
    pub hash: HashOut<F>,
}

impl MerkleSumNode {
    /// Get a new merkle sum node given a account.
    pub fn new_from_account(account: &Account) -> MerkleSumNode {
        let sum_equity = account.equity.iter().fold(F::ZERO, |acc, x| acc + *x);

        let sum_debt = account.debt.iter().fold(F::ZERO, |acc, x| acc + *x);

        let hash = account.get_hash();
        MerkleSumNode { hash, sum_equity, sum_debt }
    }

    /// Get a new MerkleSumNode given its 2 child nodes.
    pub fn new_from_children_nodes(node1: &MerkleSumNode, node2: &MerkleSumNode) -> MerkleSumNode {
        let hash = hash_2_subhashes::<F, D>(&node1.hash, &node2.hash);
        let sum_equity = node1.sum_equity + node2.sum_equity;
        let sum_debt = node2.sum_debt + node2.sum_debt;
        MerkleSumNode { hash, sum_equity, sum_debt }
    }
}

/// Struct representing a merkle sum tree, it is represented as a vector of Merkle Sum Nodes.
#[derive(Debug, Clone)]
pub struct MerkleSumTree {
    pub merkle_sum_tree: Vec<MerkleSumNode>,
    pub tree_depth: usize,
}

impl MerkleSumTree {
    pub fn new_tree_from_accounts(accounts: &[Account]) -> MerkleSumTree {
        let num_leaves = accounts.len();
        let tree_depth = log2_strict(num_leaves);
        let mut merkle_sum_tree: Vec<MerkleSumNode> = Vec::new();

        for i in 0..num_leaves * 2 - 1 {
            if i < num_leaves {
                let acct = accounts.get(i).unwrap();
                merkle_sum_tree.push(MerkleSumNode::new_from_account(acct));
            } else {
                let left_child_index = 2 * (i - num_leaves);
                let right_child_index = 2 * (i - num_leaves) + 1;
                let left_child = merkle_sum_tree.get(left_child_index).unwrap();
                let right_child = merkle_sum_tree.get(right_child_index).unwrap();
                let node = MerkleSumNode::new_from_children_nodes(left_child, right_child);
                merkle_sum_tree.push(node);
            }
        }

        MerkleSumTree { merkle_sum_tree, tree_depth }
    }

    pub fn get_root(&self) -> MerkleSumNode {
        *self.merkle_sum_tree.last().unwrap()
    }

    pub fn get_from_index(&self, index: usize) -> Option<&MerkleSumNode> {
        return self.merkle_sum_tree.get(index);
    }
}

#[cfg(test)]
pub mod test {
    use crate::{
        account::gen_accounts_with_random_data,
        circuit_config::STANDARD_CONFIG,
        merkle_sum_prover::{
            circuits::merkle_sum_circuit::{build_merkle_sum_tree_circuit, MerkleSumNodeTarget},
            prover::MerkleSumTreeProver,
        },
        parser::{FileManager, JsonFileManager},
        types::F,
    };
    use plonky2::hash::hash_types::HashOut;
    use plonky2_field::types::Field;

    use super::{MerkleSumNode, MerkleSumTree};

    #[test]
    pub fn test_new_from_account() {
        let path = "../../test-data/batch0.json";
        let fm = FileManager {};
        let tokens = vec!["BTC".to_owned(), "ETH".to_owned()];
        let accounts = fm.read_json_into_accounts_vec(path, &tokens);

        let account = accounts.get(0).unwrap();
        let node = MerkleSumNode::new_from_account(account);
        let btc_amount = 574041;
        let eth_amount = 38553;
        assert_eq!(node.sum_equity, F::from_canonical_u64(btc_amount + eth_amount));
        assert_eq!(node.sum_debt, F::from_canonical_u64(0));
    }

    #[test]
    pub fn test_new_from_children_nodes() {
        let fm = FileManager {};
        let path = "../../test-data/batch0.json";
        let tokens = vec!["BTC".to_owned(), "ETH".to_owned()];
        let accounts = fm.read_json_into_accounts_vec(path, &tokens);

        let account1 = accounts.get(0).unwrap();
        let node1 = MerkleSumNode::new_from_account(account1);
        let acc1_btc_amount = 574041;
        let acc1_eth_amount = 38553;
        let acc2_btc_amount = 4864585;
        let acc2_eth_amount = 6877764;

        let account2 = accounts.get(1).unwrap();
        let node2 = MerkleSumNode::new_from_account(account2);
        let node3 = MerkleSumNode::new_from_children_nodes(&node1, &node2);
        assert_eq!(
            node3.sum_equity,
            F::from_canonical_u64(acc1_btc_amount + acc1_eth_amount + acc2_btc_amount + acc2_eth_amount),
        );
        assert_eq!(node3.sum_debt, F::from_canonical_u64(0));
    }

    #[test]
    pub fn test_new_tree_from_accounts() {
        let fm = FileManager {};
        let path = "../../test-data/batch0.json";
        let tokens = vec!["BTC".to_owned(), "ETH".to_owned()];
        let accounts = fm.read_json_into_accounts_vec(path, &tokens);
        let mut sum_equity = F::ZERO;

        for i in 0..accounts.len() {
            let account = accounts.get(i).unwrap();
            account.equity.iter().for_each(|x| {
                sum_equity = sum_equity + *x;
            });
        }

        let tree = MerkleSumTree::new_tree_from_accounts(&accounts);

        let root = tree.get_root();
        assert_eq!(root.sum_equity, sum_equity);
        assert_eq!(root.sum_debt, F::ZERO);
    }

    #[test]
    fn test_identical_root_hash_with_proving() {
        let batch_num = 4;
        let num_assets = 4;
        let accounts = gen_accounts_with_random_data(4, num_assets);

        let merkle_sum_tree = MerkleSumTree::new_tree_from_accounts(&accounts);

        let (batch_circuit, account_targets) =
            build_merkle_sum_tree_circuit(batch_num, num_assets, STANDARD_CONFIG);

        let prover = MerkleSumTreeProver { accounts };
        let proof = prover.get_proof_with_circuit_data(&account_targets, &batch_circuit);

        let hash_offset = MerkleSumNodeTarget::pub_input_root_hash_offset();
        let proof_root_hash = HashOut::<F>::from_partial(&proof.public_inputs[hash_offset]);
        assert_eq!(proof_root_hash, merkle_sum_tree.get_root().hash);
    }
}
