use ark_ff::{BigInteger, FromBytes, PrimeField};
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use arkworks_circuits::setup::{
	common::setup_tree_and_create_path_tree_x5,
	mixer::{
		prove_groth16_circuit_x5, setup_arbitrary_data, setup_groth16_random_circuit_x5, setup_leaf_x5, Circuit_x5,
	},
};
use arkworks_gadgets::prelude::ark_groth16::ProvingKey;

use arkworks_utils::{
	poseidon::PoseidonParameters,
	utils::common::{setup_params_x5_3, setup_params_x5_5, Curve},
};
use darkwebb_primitives::ElementTrait;

use crate::mock::Element;

type Bn254Fr = ark_bn254::Fr;
type Bls12_381Fr = ark_bls12_381::Fr;

type ProofBytes = Vec<u8>;
type RootsElement = Vec<Element>;
type NullifierHashElement = Element;
type LeafElement = Element;

const TREE_DEPTH: usize = 30;
const M: usize = 2;

pub fn get_hash_params<T: PrimeField>(curve: Curve) -> (Vec<u8>, Vec<u8>) {
	(
		setup_params_x5_3::<T>(curve).to_bytes(),
		setup_params_x5_5::<T>(curve).to_bytes(),
	)
}

pub fn get_keys(curve: Curve, pk_bytes: &mut Vec<u8>, vk_bytes: &mut Vec<u8>) {
	let rng = &mut ark_std::test_rng();
	match curve {
		Curve::Bn254 => {
			let (pk, vk) = setup_groth16_random_circuit_x5::<_, ark_bn254::Bn254, TREE_DEPTH>(rng, curve);
			vk.serialize(vk_bytes).unwrap();
			pk.serialize(pk_bytes).unwrap();
		}
		Curve::Bls381 => {
			let (pk, vk) = setup_groth16_random_circuit_x5::<_, ark_bls12_381::Bls12_381, TREE_DEPTH>(rng, curve);
			vk.serialize(vk_bytes).unwrap();
			pk.serialize(pk_bytes).unwrap();
		}
	};
}

pub fn setup_zk_circuit(
	curve: Curve,
	recipient_bytes: Vec<u8>,
	relayer_bytes: Vec<u8>,
	pk_bytes: Vec<u8>,
	fee_value: u32,
	refund_value: u32,
) -> (ProofBytes, RootsElement, NullifierHashElement, LeafElement) {
	let rng = &mut ark_std::test_rng();

	match curve {
		Curve::Bn254 => {
			// fit inputs to the curve.
			let recipient = Bn254Fr::read(&recipient_bytes[..]).unwrap();
			let relayer = Bn254Fr::read(&relayer_bytes[..]).unwrap();
			let fee = Bn254Fr::from(fee_value);
			let refund = Bn254Fr::from(refund_value);

			let (params3, params5) = get_hash_params::<Bn254Fr>(curve);
			let params3_deserialized = PoseidonParameters::<Bn254Fr>::from_bytes(&*params3).unwrap();
			let params5_deserialized = PoseidonParameters::<Bn254Fr>::from_bytes(&*params5).unwrap();
			let (leaf_private, leaf, nullifier_hash) = setup_leaf_x5(&params5_deserialized, rng);

			let (mt, path) = setup_tree_and_create_path_tree_x5(&[leaf], 0, &params3_deserialized);
			let root = mt.root().inner();

			let mut roots = [Bn254Fr::default(); M];
			roots[0] = root; // local root.

			let arbitrary_input = setup_arbitrary_data(recipient, relayer, fee, refund);

			// setup the circuit.
			let circuit = Circuit_x5::new(
				arbitrary_input,
				leaf_private,
				params5_deserialized,
				path,
				root,
				nullifier_hash,
			);

			let pk = ProvingKey::<ark_bn254::Bn254>::deserialize(&*pk_bytes).unwrap();

			// generate the proof.
			let proof = prove_groth16_circuit_x5::<_, ark_bn254::Bn254, TREE_DEPTH>(&pk, circuit, rng);

			// format the input for the pallet.
			let mut proof_bytes = Vec::new();
			proof.serialize(&mut proof_bytes).unwrap();

			let roots_element = roots
				.iter()
				.map(|v| Element::from_bytes(&v.into_repr().to_bytes_le()))
				.collect::<Vec<Element>>();

			let nullifier_hash_element = Element::from_bytes(&nullifier_hash.into_repr().to_bytes_le());
			let leaf_element = Element::from_bytes(&leaf.into_repr().to_bytes_le());

			(proof_bytes, roots_element, nullifier_hash_element, leaf_element)
		}
		Curve::Bls381 => {
			// fit inputs to the curve.
			let recipient = Bls12_381Fr::read(&recipient_bytes[..]).unwrap();
			let relayer = Bls12_381Fr::read(&relayer_bytes[..]).unwrap();
			let fee = Bls12_381Fr::from(fee_value);
			let refund = Bls12_381Fr::from(refund_value);

			let (params3, params5) = get_hash_params::<Bls12_381Fr>(curve);

			let params3_deserialized = PoseidonParameters::<Bls12_381Fr>::from_bytes(&*params3).unwrap();
			let params5_deserialized = PoseidonParameters::<Bls12_381Fr>::from_bytes(&*params5).unwrap();

			let (leaf_private, leaf, nullifier_hash) = setup_leaf_x5(&params5_deserialized, rng);

			// the withdraw process..
			// we setup the inputs to our proof generator.
			let (mt, path) = setup_tree_and_create_path_tree_x5::<_, TREE_DEPTH>(&[leaf], 0, &params3_deserialized);
			let root = mt.root().inner();

			let mut roots = [Bls12_381Fr::default(); M];
			roots[0] = root; // local root.

			let arbitrary_input = setup_arbitrary_data(recipient, relayer, fee, refund);

			// setup the circuit.
			let circuit = Circuit_x5::new(
				arbitrary_input,
				leaf_private,
				params5_deserialized,
				path,
				root,
				nullifier_hash,
			);
			let pk = ProvingKey::<ark_bls12_381::Bls12_381>::deserialize(&*pk_bytes).unwrap();
			// generate the proof.
			let proof = prove_groth16_circuit_x5(&pk, circuit, rng);

			// format the input for the pallet.
			let mut proof_bytes = Vec::new();
			proof.serialize(&mut proof_bytes).unwrap();

			let roots_element = roots
				.iter()
				.map(|v| Element::from_bytes(&v.into_repr().to_bytes_le()))
				.collect::<Vec<Element>>();

			let nullifier_hash_element = Element::from_bytes(&nullifier_hash.into_repr().to_bytes_le());

			let leaf_element = Element::from_bytes(&leaf.into_repr().to_bytes_le());

			(proof_bytes, roots_element, nullifier_hash_element, leaf_element)
		}
	}
}

/// Truncate and pad 256 bit slice in reverse
pub fn truncate_and_pad_reverse(t: &[u8]) -> Vec<u8> {
	let mut truncated_bytes = t[12..].to_vec();
	truncated_bytes.extend_from_slice(&[0u8; 12]);
	truncated_bytes
}
