pub fn get_recursive_hash_nums(num_of_batches: usize, hyper_tree_leaf_size: usize) -> usize {
    assert!(num_of_batches > 0);
    if num_of_batches <= hyper_tree_leaf_size {
        return 1;
    }

    let mut next = num_of_batches.div_ceil(hyper_tree_leaf_size); 
    let mut num_of_hashes = 0;
    while next > 1 {
        num_of_hashes = num_of_hashes + next;
        next = next.div_ceil(hyper_tree_leaf_size);
    }
    num_of_hashes +1
}

/// node level is the level from tree root; the root node has level of 0; 
/// `node_idx` is the index of the nodes in a vector; the root node has the largest ndoe_idx
pub fn get_node_level(batch_size: usize, node_idx: usize) -> usize {
    let total_nums = 2*batch_size -1;
    ((total_nums- node_idx) as f64).log(2.0).floor() as usize
}

#[cfg(test)]
mod test {
    use plonky2::util::log2_strict;

    use super::*;
    
    #[test]
    fn test_get_recursive_hash_nums() {
        assert_eq!(get_recursive_hash_nums(2, 4), 1);
        assert_eq!(get_recursive_hash_nums(4, 4), 1);
        assert_eq!(get_recursive_hash_nums(6, 4), 3);
        assert_eq!(get_recursive_hash_nums(100, 4), 35);
    }
    #[test]
    fn test_get_node_level() {
        
        assert_eq!(get_node_level(8,14),0);
        assert_eq!(get_node_level(8,13),1);
        assert_eq!(get_node_level(8,9),2);
        assert_eq!(get_node_level(8,11),2);
        assert_eq!(get_node_level(8,0),3);
        assert_eq!(get_node_level(8,4),3);
        assert_eq!(get_node_level(8,7),3);

        let batch_size = 8;
        let num_of_batches = 6;
        let level_from_bottom = 3;
        let ret = 2 * batch_size * num_of_batches- 2 * batch_size* num_of_batches/(1 << level_from_bottom);
        println!("ret: {:?}", ret);
    }
}
