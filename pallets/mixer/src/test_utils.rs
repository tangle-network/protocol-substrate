use ark_bn254::Bn254;
use ark_ff::{BigInteger, FromBytes, PrimeField};
use arkworks_circuits::setup::mixer::MixerProverSetup;
use arkworks_gadgets::leaf::mixer::Private as LeafPrivate;
use arkworks_utils::utils::common::{setup_params_x5_3, setup_params_x5_5, Curve};
use darkwebb_primitives::ElementTrait;

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
	fee_value: u32,
	refund_value: u32,
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
			let recipient = Bn254Fr::read(&recipient_bytes[..]).unwrap();
			let relayer = Bn254Fr::read(&relayer_bytes[..]).unwrap();
			let fee = Bn254Fr::from(fee_value);
			let refund = Bn254Fr::from(refund_value);

			let params3 = setup_params_x5_3::<Bn254Fr>(curve);
			let params5 = setup_params_x5_5::<Bn254Fr>(curve);
			let prover = MixerProverSetupBn254_30::new(params3, params5);

			let (circuit, leaf, nullifier_hash, root, ..) =
				prover.setup_circuit(&[], 0, recipient, relayer, fee, refund, rng);

			let proof_bytes = MixerProverSetupBn254_30::prove::<Bn254, _>(circuit, &pk_bytes, rng);

			let leaf_element = Element::from_bytes(&leaf.into_repr().to_bytes_le());
			let nullifier_hash_element = Element::from_bytes(&nullifier_hash.into_repr().to_bytes_le());
			let root_element = Element::from_bytes(&root.into_repr().to_bytes_le());

			(proof_bytes, root_element, nullifier_hash_element, leaf_element)
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
