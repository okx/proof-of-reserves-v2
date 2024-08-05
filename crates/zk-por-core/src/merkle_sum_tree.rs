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
pub struct MerkleSumTree {
    pub merkle_sum_tree: Vec<MerkleSumNode>,
    pub tree_depth: usize,
}

impl MerkleSumTree {
    /// Create a new merkle sum tree from a set of accounts, note that this set must be a power of 2.
    /// In the future we can try to pad with empty accounts.
    pub fn new_tree_from_accounts(accounts: &Vec<Account>) -> MerkleSumTree {
        let mut merkle_sum_tree: Vec<MerkleSumNode> = Vec::new();
        let num_leaves = accounts.len();

        for i in 0..num_leaves * 2 - 1 {
            if i < num_leaves {
                let account = accounts.get(i).unwrap();
                merkle_sum_tree.push(MerkleSumNode::new_from_account(account));
            } else {
                let left_child_index = 2 * (i - num_leaves);
                let right_child_index = 2 * (i - num_leaves) + 1;
                let left_child = merkle_sum_tree.get(left_child_index).unwrap();
                let right_child = merkle_sum_tree.get(right_child_index).unwrap();
                let node = MerkleSumNode::new_from_children_nodes(left_child, right_child);
                merkle_sum_tree.push(node);
            }
        }

        MerkleSumTree { merkle_sum_tree, tree_depth: log2_strict(accounts.len()) }
    }

    pub fn get_root(&self) -> MerkleSumNode {
        *self.merkle_sum_tree.last().unwrap()
    }

    pub fn get_from_index(&self, index: usize) -> Option<&MerkleSumNode> {
        return self.merkle_sum_tree.get(index);
    }

    /// Get the siblings for the merkle proof of inclusion given a leaf index as Merkle Sum Nodes.
    /// We get the parent index of a leaf using the formula: parent = index / 2 + num_leaves
    pub fn get_siblings(&self, mut index: usize) -> Vec<MerkleSumNode> {
        let mut siblings = Vec::new();
        let num_leaves = 1 << self.tree_depth;
        while index < self.merkle_sum_tree.len() - 1 {
            if index % 2 == 1 {
                let sibling_index = index - 1;
                let sibling = self.merkle_sum_tree.get(sibling_index).unwrap();
                siblings.push(*sibling);
            } else {
                let sibling_index = index + 1;
                let sibling = self.merkle_sum_tree.get(sibling_index).unwrap();
                siblings.push(*sibling);
            }

            let parent = index / 2 + num_leaves;
            index = parent;
        }
        return siblings;
    }

    /// Get siblings as just the hashes of the merkle sum tree
    pub fn get_siblings_hashes(&self, index: usize) -> Vec<HashOut<F>> {
        let siblings = self.get_siblings(index);
        siblings.iter().map(|x| x.hash).collect()
    }
}

#[cfg(test)]
pub mod test {
    use crate::{parser::read_json_into_accounts_vec, types::F};
    use plonky2_field::types::Field;

    use super::{MerkleSumNode, MerkleSumTree};

    #[test]
    pub fn test_new_from_account() {
        let path = "../../test-data/batch0.json";
        let accounts = read_json_into_accounts_vec(path);

        let account = accounts.get(0).unwrap();
        let node = MerkleSumNode::new_from_account(account);
        assert_eq!(node.sum_equity, F::from_canonical_u64(133876586));
        assert_eq!(node.sum_debt, F::from_canonical_u64(0));
    }

    #[test]
    pub fn test_new_from_children_nodes() {
        let path = "../../test-data/batch0.json";
        let accounts = read_json_into_accounts_vec(path);

        let account1 = accounts.get(0).unwrap();
        let node1 = MerkleSumNode::new_from_account(account1);
        let account2 = accounts.get(1).unwrap();
        let node2 = MerkleSumNode::new_from_account(account2);
        let node3 = MerkleSumNode::new_from_children_nodes(&node1, &node2);
        assert_eq!(
            node3.sum_equity,
            F::from_canonical_u64(138512215) + F::from_canonical_u64(133876586)
        );
        assert_eq!(node3.sum_debt, F::from_canonical_u64(0));
    }

    #[test]
    pub fn test_new_tree_from_accounts() {
        let path = "../../test-data/batch0.json";
        let accounts = read_json_into_accounts_vec(path);
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
    fn test_get_siblings() {
        let path = "../../test-data/batch0.json";
        let accounts = read_json_into_accounts_vec(path);

        let merkle_sum_tree = MerkleSumTree::new_tree_from_accounts(&accounts);

        let mut siblings_calculated: Vec<MerkleSumNode> = Vec::new();
        siblings_calculated.push(*merkle_sum_tree.get_from_index(0).unwrap());
        siblings_calculated.push(*merkle_sum_tree.get_from_index(17).unwrap());
        siblings_calculated.push(*merkle_sum_tree.get_from_index(25).unwrap());
        siblings_calculated.push(*merkle_sum_tree.get_from_index(29).unwrap());

        let siblings = merkle_sum_tree.get_siblings(1);
        assert_eq!(siblings, siblings_calculated);
    }
}
