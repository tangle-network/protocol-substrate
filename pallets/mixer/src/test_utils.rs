use ark_bn254::Bn254;
use arkworks_setups::{
	common::{Leaf, MixerProof},
	r1cs::mixer::MixerR1CSProver,
	Curve, MixerProver,
};
use webb_primitives::ElementTrait;

use crate::mock::Element;

pub const DEFAULT_LEAF: [u8; 32] = [
	47, 229, 76, 96, 211, 172, 171, 243, 52, 58, 53, 182, 235, 161, 93, 180, 130, 27, 52, 15, 118,
	231, 65, 226, 36, 150, 133, 237, 72, 153, 175, 108,
];
const TREE_HEIGHT: usize = 30;
type MixerR1csproverBn254_30 = MixerR1CSProver<Bn254, TREE_HEIGHT>;

pub fn setup_zk_circuit(
	curve: Curve,
	recipient_bytes: Vec<u8>,
	relayer_bytes: Vec<u8>,
	pk_bytes: Vec<u8>,
	fee_value: u128,
	refund_value: u128,
) -> (
	Vec<u8>, // proof bytes
	Element, // root
	Element, // nullifier_hash
	Element, // leaf
) {
	let rng = &mut ark_std::test_rng();

	match curve {
		Curve::Bn254 => {
			// fit inputs to the curve.
			let Leaf { secret_bytes, nullifier_bytes, leaf_bytes, nullifier_hash_bytes, .. } =
				MixerR1csproverBn254_30::create_random_leaf(curve, rng).unwrap();

			let leaves = vec![leaf_bytes.clone()];
			let index = 0;
			let MixerProof { proof, root_raw, .. } = MixerR1csproverBn254_30::create_proof(
				curve,
				secret_bytes,
				nullifier_bytes,
				leaves,
				index,
				recipient_bytes,
				relayer_bytes,
				fee_value,
				refund_value,
				pk_bytes,
				DEFAULT_LEAF,
				rng,
			)
			.unwrap();

			let leaf_element = Element::from_bytes(&leaf_bytes);
			let nullifier_hash_element = Element::from_bytes(&nullifier_hash_bytes);
			let root_element = Element::from_bytes(&root_raw);

			(proof, root_element, nullifier_hash_element, leaf_element)
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
