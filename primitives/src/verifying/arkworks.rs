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
use sp_std::marker::PhantomData;

pub struct ArkworksMixerVerifierGroth16<E: PairingEngine>(PhantomData<E>);
pub struct ArkworksBridgeVerifierGroth16<E: PairingEngine>(PhantomData<E>);
pub struct ArkworksVAnchorVerifierGroth16<
	E: PairingEngine,
	I: const usize,
	O: const usize
>(PhantomData<E>);

impl<E: PairingEngine> InstanceVerifier for ArkworksMixerVerifierGroth16<E> {
	fn verify(public_inp_bytes: &[u8], proof_bytes: &[u8], vk_bytes: &[u8]) -> Result<bool, Error> {
		let public_input_field_elts = to_field_elements::<E::Fr>(public_inp_bytes).unwrap();
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

impl<E: PairingEngine> InstanceVerifier for ArkworksBridgeVerifierGroth16<E> {
	fn verify(public_inp_bytes: &[u8], proof_bytes: &[u8], vk_bytes: &[u8]) -> Result<bool, Error> {
		let public_input_field_elts = to_field_elements::<E::Fr>(public_inp_bytes).unwrap();
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

impl<E: PairingEngine, IN: const usize, OUT: const usize, M: const usize> InstanceVerifier for
ArkworksVAnchorVerifierGroth16<E> {
	fn verify(public_inp_bytes: &[u8], proof_bytes: &[u8], vk_bytes: &[u8]) -> Result<bool, Error> {
		let public_input_field_elts = to_field_elements::<E::Fr>(public_inp_bytes)?;
		// let public_amount = public_input_field_elts[0];
		// let ext_data = public_input_field_elts[1];
		
		// let input_nullifier_offset = 2;
		// let mut input_nullifiers = [E::Fr::zero(); IN];
		// for (i, nullifier) in input_nullifiers.iter_mut().enumerate() {
		// 	*nullifier = *public_input_field_elts.get(i + input_nullifier_offset).unwrap_or(&E::Fr::zero());
		// }

		// let output_commitment_offset = input_nullifier_offset + input_nullifiers.len();
		// let mut output_commitments = [E::Fr::zero(); OUT];
		// for (i, out_commitment) in output_commitments.iter_mut().enumerate() {
		// 	*out_commitment = *public_input_field_elts.get(i + output_commitment_offset).unwrap_or(&E::Fr::zero());
		// }

		// let roots_offset = output_commitment_offset + output_commitments.len();
		// let mut roots = [E::Fr::zero(); M];
		// for (i, root) in roots.iter_mut().enumerate() {
		// 	*root = *public_input_field_elts.get(i + roots_offset).unwrap_or(&E::Fr::zero());
		// }

		let mut public_inputs = Vec::new();
		let vk = VerifyingKey::<E>::deserialize(vk_bytes)?;
		let proof = Proof::<E>::deserialize(proof_bytes)?;
		let res = verify_groth16::<E>(&vk, &public_input_field_elts, &proof);
		Ok(res)
	}
}

use ark_bls12_381::Bls12_381;
pub type ArkworksBls381MixerVerifier = ArkworksMixerVerifierGroth16<Bls12_381>;
pub type ArkworksBls381BridgeVerifier = ArkworksBridgeVerifierGroth16<Bls12_381>;
pub type ArkworksBls381VAnchor2x2Verifier = ArkworksVAnchorVerifierGroth16<Bls12_381, 2, 2>;

use ark_bn254::Bn254;
pub type ArkworksBn254MixerVerifier = ArkworksMixerVerifierGroth16<Bn254>;
pub type ArkworksBn254BridgeVerifier = ArkworksBridgeVerifierGroth16<Bn254>;
pub type ArkworksBn254VAnchorVerifier = ArkworksVAnchorVerifierGroth16<Bn254, 2, 2>;