use crate::*;
use ark_crypto_primitives::Error;
use ark_ec::PairingEngine;
use ark_ff::Zero;
use ark_groth16::{Proof, VerifyingKey};
use ark_serialize::CanonicalDeserialize;
use arkworks_gadgets::{
	setup::{bridge, common::verify_groth16, mixer},
	utils::to_field_elements,
};
use codec::Encode;
use sp_std::marker::PhantomData;

pub struct ArkworksMixerVerifierGroth16<E: PairingEngine>(PhantomData<E>);
pub struct ArkworksAnchorVerifierGroth16<E: PairingEngine, const M: usize>(PhantomData<E>);
pub struct ArkworksVAnchorVerifierGroth16<E: PairingEngine, const I: usize, const O: usize, const M: usize>(
	PhantomData<E>,
);

impl<E: PairingEngine> InstanceVerifier for ArkworksMixerVerifierGroth16<E> {
	fn encode_public_inputs<C: Encode>(inputs: &[C]) -> Vec<u8> {
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

	fn verify(public_inp_bytes: &[u8], proof_bytes: &[u8], vk_bytes: &[u8]) -> Result<bool, Error> {
		let public_input_field_elts = to_field_elements::<E::Fr>(public_inp_bytes)?;
		let public_inputs = mixer::get_public_inputs::<E::Fr>(
			public_input_field_elts[0], // nullifier_hash
			public_input_field_elts[1], // root
			public_input_field_elts[2], // recipient
			public_input_field_elts[3], // relayer
			public_input_field_elts[4], // fee
			public_input_field_elts[5], // refund
		);
		let vk = VerifyingKey::<E>::deserialize(vk_bytes)?;
		let proof = Proof::<E>::deserialize(proof_bytes)?;
		let res = verify_groth16::<E>(&vk, &public_inputs, &proof);
		Ok(res)
	}
}

impl<E: PairingEngine, const M: usize> InstanceVerifier for ArkworksAnchorVerifierGroth16<E, M> {
	fn encode_public_inputs<C: Encode>(inputs: &[C]) -> Vec<u8> {
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

	fn verify(public_inp_bytes: &[u8], proof_bytes: &[u8], vk_bytes: &[u8]) -> Result<bool, Error> {
		let public_input_field_elts = to_field_elements::<E::Fr>(public_inp_bytes)?;
		let nullifier_hash = public_input_field_elts[0];
		let recipient = public_input_field_elts[1];
		let relayer = public_input_field_elts[2];
		let fee = public_input_field_elts[3];
		let refund = public_input_field_elts[4];
		let chain_id = public_input_field_elts[5];

		const M: usize = 2;
		let mut roots = [E::Fr::zero(); M];
		for (i, root) in roots.iter_mut().enumerate() {
			*root = *public_input_field_elts.get(i + 6).unwrap_or(&E::Fr::zero());
		}

		let public_inputs = bridge::get_public_inputs::<E::Fr, M>(
			chain_id,
			nullifier_hash,
			roots,
			roots[0],
			recipient,
			relayer,
			fee,
			refund,
		);
		let vk = VerifyingKey::<E>::deserialize(vk_bytes)?;
		let proof = Proof::<E>::deserialize(proof_bytes)?;
		let res = verify_groth16::<E>(&vk, &public_inputs, &proof);
		Ok(res)
	}
}

impl<E: PairingEngine, const IN: usize, const OUT: usize, const M: usize> InstanceVerifier
	for ArkworksVAnchorVerifierGroth16<E, IN, OUT, M>
{
	fn encode_public_inputs<C: Encode>(inputs: &[C]) -> Vec<u8> {
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

	fn verify(public_inp_bytes: &[u8], proof_bytes: &[u8], vk_bytes: &[u8]) -> Result<bool, Error> {
		let public_input_field_elts = to_field_elements::<E::Fr>(public_inp_bytes)?;
		// let public_amount = public_input_field_elts[0];
		// let ext_data = public_input_field_elts[1];
		// let input_nullifier_offset = 2;
		// let mut input_nullifiers = [E::Fr::zero(); IN];
		// for (i, nullifier) in input_nullifiers.iter_mut().enumerate() {
		// 	*nullifier = *public_input_field_elts.get(i +
		// input_nullifier_offset).unwrap_or(&E::Fr::zero()); }

		// let output_commitment_offset = input_nullifier_offset +
		// input_nullifiers.len(); let mut output_commitments = [E::Fr::zero(); OUT];
		// for (i, out_commitment) in output_commitments.iter_mut().enumerate() {
		// 	*out_commitment = *public_input_field_elts.get(i +
		// output_commitment_offset).unwrap_or(&E::Fr::zero()); }

		// let roots_offset = output_commitment_offset + output_commitments.len();
		// let mut roots = [E::Fr::zero(); M];
		// for (i, root) in roots.iter_mut().enumerate() {
		// 	*root = *public_input_field_elts.get(i +
		// roots_offset).unwrap_or(&E::Fr::zero()); }

		let vk = VerifyingKey::<E>::deserialize(vk_bytes)?;
		let proof = Proof::<E>::deserialize(proof_bytes)?;
		let res = verify_groth16::<E>(&vk, &public_input_field_elts, &proof);
		Ok(res)
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
