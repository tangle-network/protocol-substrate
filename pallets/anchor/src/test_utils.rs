use ark_bn254::Bn254;
use ark_ff::{BigInteger, PrimeField};
use arkworks_circuits::setup::{
	anchor::{setup_leaf_x5_4, setup_proof_x5_4, AnchorProverSetup},
};

use arkworks_utils::utils::common::{setup_params_x5_3, setup_params_x5_4, Curve};
use webb_primitives::ElementTrait;

use crate::mock::Element;

type Bn254Fr = ark_bn254::Fr;

type ProofBytes = Vec<u8>;
type RootsElement = Vec<Element>;
type NullifierHashElement = Element;
type LeafElement = Element;

// merkle proof path legth
// TreeConfig_x5, x7 HEIGHT is hardcoded to 30
pub const TREE_DEPTH: usize = 30;
pub const M: usize = 2;
pub type AnchorSetup30_2 = AnchorProverSetup<Bn254Fr, M, TREE_DEPTH>;

pub fn setup_zk_circuit(
	curve: Curve,
	recipient_bytes: Vec<u8>,
	relayer_bytes: Vec<u8>,
	commitment_bytes: Vec<u8>,
	pk_bytes: Vec<u8>,
	src_chain_id: u128,
	fee_value: u128,
	refund_value: u128,
) -> (ProofBytes, RootsElement, NullifierHashElement, LeafElement) {
	let rng = &mut ark_std::test_rng();

	match curve {
		Curve::Bn254 => {
			let (secret, nullifier, leaf, nullifier_hash) = setup_leaf_x5_4::<Bn254Fr, _>(Curve::Bn254, src_chain_id, rng).unwrap();
			let leaves = vec![leaf.clone()];
			let leaves_f = vec![Bn254Fr::from_le_bytes_mod_order(&leaf)];
			let index = 0;

			let params3 = setup_params_x5_3::<Bn254Fr>(curve);
			let params4 = setup_params_x5_4::<Bn254Fr>(curve);
			let anchor_setup = AnchorSetup30_2::new(params3, params4);
			let (tree, _) = anchor_setup.setup_tree_and_path(&leaves_f, index).unwrap();
			let roots_f = [tree.root().inner(); M];
			let roots_raw = roots_f.map(|x| x.into_repr().to_bytes_le());

			let (proof, ..) = setup_proof_x5_4::<Bn254, _>(curve, src_chain_id, secret, nullifier, leaves, index, roots_raw.clone(), recipient_bytes, relayer_bytes, commitment_bytes, fee_value, refund_value, pk_bytes, rng).unwrap();

			let roots_element = roots_raw.map(|x| Element::from_bytes(&x)).to_vec();
			let nullifier_hash_element = Element::from_bytes(&nullifier_hash);
			let leaf_element = Element::from_bytes(&leaf);

			(proof, roots_element, nullifier_hash_element, leaf_element)
		},
		Curve::Bls381 => {
			unimplemented!()
		},
	}
}

/// Truncate and pad 256 bit slice in reverse
pub fn truncate_and_pad_reverse(t: &[u8]) -> Vec<u8> {
	let mut truncated_bytes = t[12..].to_vec();
	truncated_bytes.extend_from_slice(&[0u8; 12]);
	truncated_bytes
}
