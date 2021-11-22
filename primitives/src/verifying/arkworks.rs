use crate::*;
use ark_crypto_primitives::Error;
use ark_ec::PairingEngine;
use codec::Encode;
use sp_std::marker::PhantomData;

pub struct ArkworksMixerVerifierGroth16<E: PairingEngine>(PhantomData<E>);
pub struct ArkworksAnchorVerifierGroth16<E: PairingEngine, const M: usize>(PhantomData<E>);
pub struct ArkworksVAnchorVerifierGroth16<E: PairingEngine, const I: usize, const O: usize, const M: usize>(
	PhantomData<E>,
);

impl<E: PairingEngine> InstanceVerifier for ArkworksMixerVerifierGroth16<E> {
	fn pack_public_inputs(inputs: &[Vec<u8>]) -> Vec<u8> {
		let recipient = &inputs[0];
		let relayer = &inputs[1];
		let fee = &inputs[2];
		let refund = &inputs[3];
		let nullifier_hash = &inputs[4];
		let root = &inputs[5];

		let mut bytes = vec![];
		let recipient_bytes = truncate_and_pad(&recipient.using_encoded(element_encoder)[..]);
		let relayer_bytes = truncate_and_pad(&relayer.using_encoded(element_encoder)[..]);
		let fee_bytes = fee.using_encoded(element_encoder);
		let refund_bytes = refund.using_encoded(element_encoder);
		bytes.extend_from_slice(&nullifier_hash.encode());
		bytes.extend_from_slice(&root.encode());
		bytes.extend_from_slice(&recipient_bytes);
		bytes.extend_from_slice(&relayer_bytes);
		bytes.extend_from_slice(&fee_bytes);
		bytes.extend_from_slice(&refund_bytes);

		bytes
	}

	fn pack_public_inputs_and_verify(inputs: &[Vec<u8>], proof_bytes: &[u8], vk_bytes: &[u8]) -> Result<bool, Error> {
		let inputs = Self::pack_public_inputs(inputs);
		Self::verify::<E>(&inputs, proof_bytes, vk_bytes)
	}
}

impl<E: PairingEngine, const M: usize> InstanceVerifier for ArkworksAnchorVerifierGroth16<E, M> {
	fn pack_public_inputs(inputs: &[Vec<u8>]) -> Vec<u8> {
		let recipient = &inputs[0];
		let relayer = &inputs[1];
		let fee = &inputs[2];
		let refund = &inputs[3];
		let chain_id = &inputs[4];
		let nullifier_hash = &inputs[5];
		let roots = &inputs[6..];

		let mut bytes = vec![];
		let recipient_bytes = truncate_and_pad(&recipient.using_encoded(element_encoder)[..]);
		let relayer_bytes = truncate_and_pad(&relayer.using_encoded(element_encoder)[..]);
		let fee_bytes = fee.using_encoded(element_encoder);
		let refund_bytes = refund.using_encoded(element_encoder);
		let chain_id_bytes = chain_id.using_encoded(element_encoder);

		bytes.extend_from_slice(&nullifier_hash.encode());
		bytes.extend_from_slice(&recipient_bytes);
		bytes.extend_from_slice(&relayer_bytes);
		bytes.extend_from_slice(&fee_bytes);
		bytes.extend_from_slice(&refund_bytes);
		bytes.extend_from_slice(&chain_id_bytes);
		for root in roots {
			bytes.extend_from_slice(&root.encode());
		}
		bytes
	}

	fn pack_public_inputs_and_verify(inputs: &[Vec<u8>], proof_bytes: &[u8], vk_bytes: &[u8]) -> Result<bool, Error> {
		let inputs = Self::pack_public_inputs(inputs);
		Self::verify::<E>(&inputs, proof_bytes, vk_bytes)
	}
}

impl<E: PairingEngine, const IN: usize, const OUT: usize, const M: usize> InstanceVerifier
	for ArkworksVAnchorVerifierGroth16<E, IN, OUT, M>
{
	fn pack_public_inputs(inputs: &[Vec<u8>]) -> Vec<u8> {
		let chain_id = &inputs[0];
		let public_amount = &inputs[1];
		let ext_data_hash = &inputs[2];

		// roots
		let roots_offset = M + 3;
		let roots = &inputs[3..roots_offset];

		// input_nullifiers
		let input_nullifiers_offset = roots_offset + IN;
		let input_nullifiers = &inputs[roots_offset..input_nullifiers_offset];
		// output commitments
		let output_commitments_offset = input_nullifiers_offset + OUT;
		let output_commitments = &inputs[input_nullifiers_offset..output_commitments_offset];

		let mut bytes = vec![];

		bytes.extend_from_slice(&chain_id.using_encoded(element_encoder));
		bytes.extend_from_slice(&public_amount.using_encoded(element_encoder));
		bytes.extend_from_slice(&ext_data_hash.using_encoded(element_encoder));
		for root in roots {
			bytes.extend_from_slice(&root.encode());
		}
		for in_null in input_nullifiers {
			bytes.extend_from_slice(&in_null.encode());
		}
		for out_comm in output_commitments {
			bytes.extend_from_slice(&out_comm.encode());
		}
		bytes
	}

	fn pack_public_inputs_and_verify(inputs: &[Vec<u8>], proof_bytes: &[u8], vk_bytes: &[u8]) -> Result<bool, Error> {
		let inputs = Self::pack_public_inputs(inputs);
		Self::verify::<E>(&inputs, proof_bytes, vk_bytes)
	}
}

/// Truncate and pad 256 bit slice
pub fn truncate_and_pad(t: &[u8]) -> Vec<u8> {
	let mut truncated_bytes = t[..20].to_vec();
	truncated_bytes.extend_from_slice(&[0u8; 12]);
	truncated_bytes
}

pub fn element_encoder(v: &[u8]) -> [u8; 32] {
	let mut output = [0u8; 32];
	output.iter_mut().zip(v).for_each(|(b1, b2)| *b1 = *b2);
	output
}

use ark_bls12_381::Bls12_381;
pub type ArkworksBls381MixerVerifier = ArkworksMixerVerifierGroth16<Bls12_381>;
pub type ArkworksBls381BridgeVerifier = ArkworksAnchorVerifierGroth16<Bls12_381, 2>;
pub type ArkworksBls381VAnchor2x2Verifier = ArkworksVAnchorVerifierGroth16<Bls12_381, 2, 2, 2>;

use ark_bn254::Bn254;
pub type ArkworksBn254MixerVerifier = ArkworksMixerVerifierGroth16<Bn254>;
pub type ArkworksBn254BridgeVerifier = ArkworksAnchorVerifierGroth16<Bn254, 2>;
pub type ArkworksBn254VAnchor2x2Verifier = ArkworksVAnchorVerifierGroth16<Bn254, 2, 2, 2>;
