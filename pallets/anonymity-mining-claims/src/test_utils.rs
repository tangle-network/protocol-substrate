use ark_ff::{BigInteger, PrimeField};
use webb_primitives::ElementTrait;

use crate::mock::Element;

type Bn254Fr = ark_bn254::Fr;
type Bn254 = ark_bn254::Bn254;

pub fn deconstruct_public_inputs_reward_proof(
	max_edges: usize,
	public_inputs: &Vec<Bn254Fr>,
) -> (
	Bn254Fr,
	Bn254Fr,
	Bn254Fr,
	Bn254Fr,
	Bn254Fr,
	Bn254Fr,
	Bn254Fr,
	Bn254Fr,
	Bn254Fr,
	Vec<Bn254Fr>,
	Vec<Bn254Fr>,
) {
	let rate = public_inputs[0];
	let fee = public_inputs[1];
	let reward_nullifier = public_inputs[2];
	let note_ak_alpha_x = public_inputs[3];
	let note_ak_alpha_y = public_inputs[4];
	let ext_data_hash = public_inputs[5];
	let input_root = public_inputs[6];
	let input_nullifier = public_inputs[7];
	let output_commitment = public_inputs[8];
	let unspent_roots = public_inputs[9..9+max_edges].to_vec();
	let spent_roots = public_inputs[9+max_edges..9+(2*max_edges)].to_vec();
	return (
		rate,
		fee,
		reward_nullifier,
		note_ak_alpha_x,
		note_ak_alpha_y,
		ext_data_hash,
		input_root,
		input_nullifier,
		output_commitment,
		unspent_roots,
		spent_roots,
	);
}

pub fn deconstruct_public_inputs_reward_proof_el(
	max_edges: u8,
	public_inputs_f: &Vec<Bn254Fr>,
) -> (
	Element,
	Element,
	Element,
	Element,
	Element,
	Element,
	Element,
	Element,
	Element,
	Vec<Element>,
	Vec<Element>,
) {
	let (rate, fee, reward_nullifier, note_ak_alpha_x, note_ak_alpha_y, ext_data_hash, input_root, input_nullifier, output_commitment, spent_roots, unspent_roots) =
		deconstruct_public_inputs_reward_proof(max_edges as usize, public_inputs_f);

	let rate_el = Element::from_bytes(&rate.into_repr().to_bytes_be());
	let fee_el = Element::from_bytes(&fee.into_repr().to_bytes_be());
	let reward_nullifier_el = Element::from_bytes(&reward_nullifier.into_repr().to_bytes_be());
	let reward_nullifier_el = Element::from_bytes(&reward_nullifier.into_repr().to_bytes_be());
	let  note_ak_alpha_x_el = Element::from_bytes(&note_ak_alpha_x.into_repr().to_bytes_be());
	let note_ak_alpha_y_el = Element::from_bytes(&note_ak_alpha_y.into_repr().to_bytes_be());
	let ext_data_hash_el = Element::from_bytes(&ext_data_hash.into_repr().to_bytes_be());
	let input_root_el = Element::from_bytes(&input_root.into_repr().to_bytes_be());
	let input_nullifier_el = Element::from_bytes(&input_nullifier.into_repr().to_bytes_be());
	let output_commitment_el = Element::from_bytes(&output_commitment.into_repr().to_bytes_be());

	let unspent_roots_el = unspent_roots
		.iter()
		.map(|x| Element::from_bytes(&x.into_repr().to_bytes_be()))
		.collect();

	let spent_roots_el = spent_roots
		.iter()
		.map(|x| Element::from_bytes(&x.into_repr().to_bytes_be()))
		.collect();

	return (
		rate_el,
		fee_el,
		reward_nullifier_el,
		note_ak_alpha_x_el,
		note_ak_alpha_y_el,
		ext_data_hash_el,
		input_root_el,
		input_nullifier_el,
		output_commitment_el,
		unspent_roots_el,
		spent_roots_el,
	);
}
