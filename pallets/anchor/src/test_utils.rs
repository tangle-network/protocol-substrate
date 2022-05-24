use ark_bn254::Bn254;
use ark_ff::{BigInteger, PrimeField};
use arkworks_native_gadgets::poseidon::Poseidon;
use arkworks_setups::{
	common::{setup_params, setup_tree_and_create_path, AnchorProof, Leaf},
	r1cs::anchor::AnchorR1CSProver,
	AnchorProver, Curve,
};
use std::convert::TryInto;
use webb_primitives::ElementTrait;

use wasm_utils::{
	proof::{generate_proof_js, AnchorProofInput, JsProofInput, ProofInput},
	types::{Backend, Curve as WasmCurve},
};

use crate::mock::Element;

type Bn254Fr = ark_bn254::Fr;

type ProofBytes = Vec<u8>;
type RootsElement = Vec<Element>;
type NullifierHashElement = Element;
type LeafElement = Element;

pub const DEFAULT_LEAF: [u8; 32] = [
	108, 175, 153, 72, 237, 133, 150, 36, 226, 65, 231, 118, 15, 52, 27, 130, 180, 93, 161, 235,
	182, 53, 58, 52, 243, 171, 172, 211, 96, 76, 229, 47,
];
// merkle proof path legth
// TreeConfig_x5, x7 HEIGHT is hardcoded to 30
pub const TREE_DEPTH: usize = 30;
pub const ANCHOR_CT: usize = 2;
#[allow(non_camel_case_types)]
pub type AnchorR1CSProver_Bn254_30_2 = AnchorR1CSProver<Bn254, TREE_DEPTH, ANCHOR_CT>;

pub fn setup_zk_circuit(
	curve: Curve,
	recipient_bytes: Vec<u8>,
	relayer_bytes: Vec<u8>,
	commitment_bytes: Vec<u8>,
	pk_bytes: Vec<u8>,
	chain_id: u64,
	fee_value: u128,
	refund_value: u128,
	neighbor_roots: [Element; ANCHOR_CT - 1],
) -> (ProofBytes, RootsElement, NullifierHashElement, LeafElement) {
	let rng = &mut ark_std::test_rng();

	match curve {
		Curve::Bn254 => {
			let Leaf { secret_bytes, nullifier_bytes, leaf_bytes, nullifier_hash_bytes, .. } =
				AnchorR1CSProver_Bn254_30_2::create_random_leaf(Curve::Bn254, chain_id.into(), rng)
					.unwrap();
			let leaves = vec![leaf_bytes.clone()];
			let leaves_f = vec![Bn254Fr::from_le_bytes_mod_order(&leaf_bytes)];
			let index = 0;

			let params3 = setup_params::<Bn254Fr>(curve, 5, 3);
			let poseidon3 = Poseidon::new(params3);
			let (tree, _) = setup_tree_and_create_path::<Bn254Fr, Poseidon<Bn254Fr>, TREE_DEPTH>(
				&poseidon3,
				&leaves_f,
				index,
				&DEFAULT_LEAF,
			)
			.unwrap();
			let roots_f: [Bn254Fr; ANCHOR_CT] = vec![tree.root()]
				.iter()
				.chain(
					neighbor_roots
						.iter()
						.map(|r| Bn254Fr::from_le_bytes_mod_order(r.to_bytes()))
						.collect::<Vec<Bn254Fr>>()
						.iter(),
				)
				.cloned()
				.collect::<Vec<Bn254Fr>>()
				.try_into()
				.unwrap();
			let roots_raw = roots_f.map(|x| x.into_repr().to_bytes_le());

			let AnchorProof { proof, roots_raw, .. } = AnchorR1CSProver_Bn254_30_2::create_proof(
				curve,
				chain_id.into(),
				secret_bytes,
				nullifier_bytes,
				leaves,
				index,
				roots_raw.clone(),
				recipient_bytes,
				relayer_bytes,
				fee_value,
				refund_value,
				commitment_bytes,
				pk_bytes,
				DEFAULT_LEAF,
				rng,
			)
			.unwrap();

			let roots_element = roots_raw.iter().map(|x| Element::from_bytes(&x)).collect();
			let nullifier_hash_element = Element::from_bytes(&nullifier_hash_bytes);
			let leaf_element = Element::from_bytes(&leaf_bytes);

			(proof, roots_element, nullifier_hash_element, leaf_element)
		},
		Curve::Bls381 => {
			unimplemented!()
		},
	}
}

pub fn setup_wasm_utils_zk_circuit(
	curve: Curve,
	recipient_bytes: Vec<u8>,
	relayer_bytes: Vec<u8>,
	commitment_bytes: [u8; 32],
	pk_bytes: Vec<u8>,
	chain_id: u64,
	fee_value: u128,
	refund_value: u128,
	neighbor_roots: [Element; ANCHOR_CT - 1],
) -> (
	Vec<u8>,      // proof bytes
	Vec<Element>, // roots
	Element,      // nullifier_hash
	Element,      // leaf
) {
	match curve {
		Curve::Bn254 => {
			let note_secret = "7e0f4bfa263d8b93854772c94851c04b3a9aba38ab808a8d081f6f5be9758110b7147c395ee9bf495734e4703b1f622009c81712520de0bbd5e7a10237c7d829bf6bd6d729cca778ed9b6fb172bbb12b01927258aca7e0a66fd5691548f8717";
			let raw = hex::decode(&note_secret).unwrap();

			let secret = &raw[0..32];
			let nullifier = &raw[0..64];
			let leaf = AnchorR1CSProver_Bn254_30_2::create_leaf_with_privates(
				curve,
				chain_id,
				secret.to_vec(),
				nullifier.to_vec(),
			)
			.unwrap();

			let leaves = vec![leaf.leaf_bytes.clone()];
			let leaves_f = vec![Bn254Fr::from_le_bytes_mod_order(&leaf.leaf_bytes)];
			let index = 0;

			let params3 = setup_params::<Bn254Fr>(curve, 5, 3);
			let poseidon3 = Poseidon::new(params3);

			let (tree, _) = setup_tree_and_create_path::<Bn254Fr, Poseidon<Bn254Fr>, TREE_DEPTH>(
				&poseidon3,
				&leaves_f,
				index,
				&DEFAULT_LEAF,
			)
			.unwrap();
			let roots_f: [Bn254Fr; ANCHOR_CT] = vec![tree.root()]
				.iter()
				.chain(
					neighbor_roots
						.iter()
						.map(|r| Bn254Fr::from_le_bytes_mod_order(r.to_bytes()))
						.collect::<Vec<Bn254Fr>>()
						.iter(),
				)
				.cloned()
				.collect::<Vec<Bn254Fr>>()
				.try_into()
				.unwrap();
			let roots_raw = roots_f.map(|x| x.into_repr().to_bytes_le());

			let mixer_proof_input = AnchorProofInput {
				exponentiation: 5,
				width: 4,
				curve: WasmCurve::Bn254,
				backend: Backend::Arkworks,
				secret: secret.to_vec(),
				nullifier: nullifier.to_vec(),
				recipient: recipient_bytes,
				relayer: relayer_bytes,
				pk: pk_bytes,
				refund: refund_value,
				fee: fee_value,
				chain_id,
				leaves,
				leaf_index: index,
				roots: roots_raw.to_vec(),
				refresh_commitment: commitment_bytes,
			};
			let js_proof_inputs = JsProofInput { inner: ProofInput::Anchor(mixer_proof_input) };
			let proof = generate_proof_js(js_proof_inputs).unwrap();

			let root_elements = proof.roots.iter().map(|root| Element::from_bytes(&root)).collect();
			let nullifier_hash_element = Element::from_bytes(&proof.nullifier_hash);
			let leaf_element = Element::from_bytes(&proof.leaf);

			(proof.proof, root_elements, nullifier_hash_element, leaf_element)
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
