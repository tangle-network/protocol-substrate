use ark_bn254::Bn254;
use ark_ff::{BigInteger, FromBytes, PrimeField};
use arkworks_circuits::setup::{common::{prove_unchecked, prove}, mixer::{MixerProverSetup, setup_leaf_x5_5, setup_proof_x5_5}};
use arkworks_utils::utils::common::{Curve};
use webb_primitives::ElementTrait;

use crate::mock::Element;

type Bn254Fr = ark_bn254::Fr;
type Bls12_381Fr = ark_bls12_381::Fr;

pub const LEN: usize = 30;
pub type MixerProverSetupBn254_30 = MixerProverSetup<Bn254Fr, LEN>;

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
			let (secret, nullifier, leaf, nullifier_hash) = setup_leaf_x5_5::<Bn254Fr, _>(curve, rng).unwrap();

			let leaves = vec![leaf.clone()];
			let index = 0;
			let (proof, _, _, root, public_inputs) = setup_proof_x5_5::<Bn254, _>(curve, secret, nullifier, leaves, index, recipient_bytes, relayer_bytes, fee_value, refund_value, pk_bytes, rng).unwrap();

			let leaf_element = Element::from_bytes(&leaf);
			let nullifier_hash_element = Element::from_bytes(&nullifier_hash);
			let root_element = Element::from_bytes(&root);

			(proof, root_element, nullifier_hash_element, leaf_element)
		}
		Curve::Bls381 => {
			unimplemented!()
		}
	}
}

/// Truncate and pad 256 bit slice in reverse
pub fn truncate_and_pad_reverse(t: &[u8]) -> Vec<u8> {
	let mut truncated_bytes = t[12..].to_vec();
	truncated_bytes.extend_from_slice(&[0u8; 12]);
	truncated_bytes
}
