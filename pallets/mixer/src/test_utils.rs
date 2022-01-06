use ark_bn254::Bn254;
use ark_ff::{BigInteger, FromBytes, PrimeField};
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use ark_std::UniformRand;
use arkworks_circuits::setup::mixer::MixerProverSetup;
use arkworks_gadgets::{
	arbitrary::mixer_data::{constraints::InputVar as ArbitraryInputVar, Input as ArbitraryInput},
	leaf::mixer::{
		constraints::{MixerLeafGadget, PrivateVar as LeafPrivateVar},
		Private as LeafPrivate,
	},
	merkle_tree::{
		constraints::{NodeVar, PathVar},
		Config as MerkleConfig, Path,
	},
	prelude::ark_groth16::ProvingKey,
};
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

pub const LEN: usize = 30;
pub type MixerProverSetupBn254_30 = MixerProverSetup<Bn254Fr, LEN>;

const TREE_DEPTH: usize = 30;
const M: usize = 2;

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

			let params3 = setup_params_x5_3::<Bn254Fr>(curve);
			let params5 = setup_params_x5_5::<Bn254Fr>(curve);
			let prover = MixerProverSetupBn254_30::new(params3, params5);
			let (leaf_private, leaf, nullifier_hash) = prover.setup_leaf(rng);

			// Invalid nullifier
			let leaf_private = LeafPrivate::new(leaf_private.secret(), leaf_private.nullifier());

			let arbitrary_input = MixerProverSetupBn254_30::setup_arbitrary_data(recipient, relayer, fee, refund);
			let (tree, path) = prover.setup_tree_and_create_path(&[leaf], 0);
			let root = tree.root().inner();
			let mut roots = [Bn254Fr::default(); M];
			roots[0] = root; // local root.

			let circuit = prover.create_circuit(arbitrary_input.clone(), leaf_private, path, root, nullifier_hash);

			let mut public_inputs = Vec::new();
			public_inputs.push(nullifier_hash);
			public_inputs.push(root);
			public_inputs.push(arbitrary_input.recipient);
			public_inputs.push(arbitrary_input.relayer);
			public_inputs.push(arbitrary_input.fee);
			public_inputs.push(arbitrary_input.refund);

			let proof_bytes = MixerProverSetupBn254_30::prove::<Bn254, _>(circuit, &pk_bytes, rng);

			let roots_element = roots
				.iter()
				.map(|v| Element::from_bytes(&v.into_repr().to_bytes_le()))
				.collect::<Vec<Element>>();

			let nullifier_hash_element = Element::from_bytes(&nullifier_hash.into_repr().to_bytes_le());
			let leaf_element = Element::from_bytes(&leaf.into_repr().to_bytes_le());

			(proof_bytes, roots_element, nullifier_hash_element, leaf_element)
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
