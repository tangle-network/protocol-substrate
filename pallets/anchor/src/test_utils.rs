use ark_bn254::Bn254;
use ark_ff::{BigInteger, FromBytes, PrimeField};
use arkworks_circuits::setup::{
	anchor::{
		AnchorProverSetup,
	},
	common::{prove},
};

use arkworks_utils::{
	utils::common::{setup_params_x5_3, Curve, setup_params_x5_4},
};
use darkwebb_primitives::ElementTrait;

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
	pk_bytes: Vec<u8>,
	src_chain_id: u32,
	fee_value: u32,
	refund_value: u32,
) -> (ProofBytes, RootsElement, NullifierHashElement, LeafElement) {
	let rng = &mut ark_std::test_rng();

	match curve {
		Curve::Bn254 => {
			// fit inputs to the curve.
			let chain_id = Bn254Fr::from(src_chain_id);
			let recipient = Bn254Fr::read(&recipient_bytes[..]).unwrap();
			let relayer = Bn254Fr::read(&relayer_bytes[..]).unwrap();
			let fee = Bn254Fr::from(fee_value);
			let refund = Bn254Fr::from(refund_value);
			let commitment = Bn254Fr::from(0);
			let params3 = setup_params_x5_3::<Bn254Fr>(curve);
			let params4 = setup_params_x5_4::<Bn254Fr>(curve);
			let anchor_setup = AnchorSetup30_2::new(params3, params4);

			let mut neighboring_roots = [Bn254Fr::default(); M - 1];
			let (leaf_privates, leaf_public, leaf_hash, ..) = anchor_setup.setup_leaf(chain_id, rng).unwrap();
			let secret = leaf_privates.secret();
			let nullifier = leaf_privates.nullifier();
			let leaves = vec![leaf_hash];
			let index = 0;
			let (circuit, leaf, nullifier_hash, root, public_inputs) = anchor_setup
				.setup_circuit_with_privates(chain_id, secret, nullifier, &leaves, index, &neighboring_roots, recipient, relayer, fee, refund, commitment)
				.unwrap();

			let mut roots = [Bn254Fr::default(); M];
			roots[1..].copy_from_slice(&neighboring_roots);
			roots[0] = root;
	
			let proof_bytes = prove::<Bn254, _, _>(circuit, &pk_bytes, rng).unwrap();

			let leaf_element = Element::from_bytes(&leaf.into_repr().to_bytes_le());
			let nullifier_hash_element = Element::from_bytes(&nullifier_hash.into_repr().to_bytes_le());
			let roots_element = roots.iter().map(|x| Element::from_bytes(&x.into_repr().to_bytes_le())).collect();

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
