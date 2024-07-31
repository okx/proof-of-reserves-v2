use super::prove::ProofTuple;

use itertools::Itertools;
use plonky2::{
    field::{extension::Extendable, types::Field},
    hash::{
        hash_types::{HashOut, RichField},
        merkle_proofs::MerkleProof,
        merkle_tree::MerkleTree,
        poseidon::PoseidonHash,
    },
    plonk::{
        circuit_data::VerifierOnlyCircuitData,
        config::{GenericConfig, GenericHashOut},
    },
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_with::serde_as;
use std::collections::HashMap;

//TODO: Deserialize
#[derive(Serialize, Deserialize, Debug, Clone)]
// #[serde(bound(deserialize = "'de: 'static + serde::de::Deserialize<'de>"))]
#[serde(bound = "F: Serialize + DeserializeOwned")]
pub struct VDProof<F: RichField + Extendable<D>, const D: usize> {
    pub merkle_proof: MerkleProof<F, PoseidonHash>,
    pub index: F,
    pub root: HashOut<F>,
}

/// key is vd digest, values it the merkle proof of that vd digest
#[serde_as]
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(bound = "F: Serialize + DeserializeOwned")]
pub struct VdMap<F: RichField + Extendable<D>, const D: usize> {
    #[serde_as(as = "Vec<(_, _)>")]
    vd_map: HashMap<Vec<F>, VDProof<F, D>>,
}

impl<F: RichField + Extendable<D>, const D: usize> VdMap<F, D> {
    pub fn new() -> Self {
        Self { vd_map: HashMap::new() }
    }

    pub fn insert(&mut self, digest: Vec<F>, vd_proof: VDProof<F, D>) {
        self.vd_map.insert(digest, vd_proof);
    }

    pub fn get(&self, digest: &Vec<F>) -> Option<&VDProof<F, D>> {
        self.vd_map.get(digest)
    }

    pub fn contains_key(&self, digest: &Vec<F>) -> bool {
        self.vd_map.contains_key(digest)
    }
}

pub trait VerifierDataDigest<F: RichField + Extendable<D>, const D: usize> {
    fn digest(&self) -> Vec<F>;
}

impl<F: RichField + Extendable<D>, C: GenericConfig<D, F = F>, const D: usize>
    VerifierDataDigest<F, D> for VerifierOnlyCircuitData<C, D>
{
    fn digest(&self) -> Vec<F> {
        let vd_digest = [
            self.constants_sigmas_cap.flatten().as_slice(),
            self.circuit_digest.to_vec().as_slice(),
        ]
        .concat();
        vd_digest
    }
}

pub trait PrettyDisplayable {
    fn pretty(&self) -> String;
}

impl<F: Field> PrettyDisplayable for HashOut<F> {
    fn pretty(&self) -> String {
        format!("HashOut({})", self.elements.map(|x| x.to_string()).join(", "))
    }
}

pub trait Copyable {
    fn copy(&self) -> Self;
}

impl<F: RichField + Extendable<D>, C: GenericConfig<D, F = F>, const D: usize> Copyable
    for VerifierOnlyCircuitData<C, D>
{
    fn copy(&self) -> Self {
        VerifierOnlyCircuitData {
            constants_sigmas_cap: self.constants_sigmas_cap.clone(),
            circuit_digest: self.circuit_digest.clone(),
        }
    }
}

#[derive(Debug)]
pub struct VdTree<F: RichField + Extendable<D>, const D: usize> {
    inner_tree: MerkleTree<F, PoseidonHash>,
    pub vd_proof_map: VdMap<F, D>,
    next_vd_index: usize,
    leaf_size: usize,
}

impl<F: RichField + Extendable<D>, const D: usize> VdTree<F, D> {
    /// vd_digests is only used to initialize. its len might be smaller than `size`
    pub fn new(vd_digests: Vec<Vec<F>>, leaf_size: usize) -> Self {
        let mut leaves = vec![vec![]; leaf_size];
        vd_digests.iter().unique().enumerate().for_each(|(i, vd)| {
            leaves[i] = vd.to_vec();
        });
        let innter_vd_tree: MerkleTree<F, PoseidonHash> = MerkleTree::new_from_2d(leaves, 0);

        let vd_map = VdMap::<F, D>::new();
        let mut vd_tree = Self {
            inner_tree: innter_vd_tree,
            vd_proof_map: vd_map,
            next_vd_index: vd_digests.len(),
            leaf_size: leaf_size,
        };
        vd_tree.update_vd_proof_map();
        vd_tree
    }

    pub fn update_vd_digests_by_proofs<C: GenericConfig<D, F = F>>(
        &mut self,
        proofs: Vec<&ProofTuple<F, C, D>>,
    ) {
        let vd_digests = proofs.iter().map(|p| p.1.clone().digest()).collect::<Vec<Vec<F>>>();
        self.update_vd_digests::<C>(vd_digests);
    }

    pub fn update_vd_digests<C: GenericConfig<D, F = F>>(
        &mut self,
        vd_digests: Vec<Vec<F>>,
    ) {
        for vd in vd_digests {
            if !self.vd_proof_map.contains_key(&vd) {
                self.inner_tree.change_leaf_and_update( vd, self.next_vd_index);
                self.next_vd_index = self.next_vd_index + 1;
            }
        }
        self.update_vd_proof_map();
    }

    fn update_vd_proof_map(&mut self) {
        let vd_proofs = self.compute_vd_proof();
        for i in 0..self.next_vd_index {
            let vd_digest = &self.inner_tree.get_leaves_2d()[i];
            self.vd_proof_map.insert(vd_digest.clone(), vd_proofs[i].clone());
        }
    }

    pub fn get_vd_proofs(
        &self,
        vds: &Vec<Vec<F>>,
    ) -> Vec<VDProof<F, D>> {
        let vd_proofs = vds.iter().map(|proof| self.get_vd_proof(proof)).collect::<Vec<_>>();
        vd_proofs
    }

    pub fn get_vd_proof(
        &self,
        vd: &Vec<F>,
    ) -> VDProof<F, D> {
        self.vd_proof_map.get(&vd).unwrap().clone()
    }

    /// Given a merkle tree with vd at each leaf, we make merkle proofs for each one of the leaf and returns a vd proof for each leaf as a vec of vd proofs
    fn compute_vd_proof(&self) -> Vec<VDProof<F, D>> {
        let vd_root = self.inner_tree.cap.0[0];
        let merkle_proofs =
            (0..self.leaf_size).into_iter().map(|i| self.inner_tree.prove(i)).collect::<Vec<_>>();

        let vd_proofs: Vec<VDProof<F, D>> = merkle_proofs
            .into_iter()
            .enumerate()
            .map(|(i, x)| VDProof {
                merkle_proof: x,
                index: F::from_canonical_u64(i as u64),
                root: vd_root,
            })
            .collect();
        vd_proofs
    }
}

#[allow(dead_code)]
fn calculate_hash(t: String) -> u64 {
    let mut s = std::collections::hash_map::DefaultHasher::new();
    std::hash::Hash::hash(&t, &mut s);
    std::hash::Hasher::finish(&s)
}

