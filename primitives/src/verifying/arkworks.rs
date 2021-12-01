use crate::*;
use ark_crypto_primitives::Error;
use ark_ec::PairingEngine;
use ark_ff::{to_bytes, BigInteger, PrimeField, ToBytes, UniformRand, Zero};
use ark_groth16::{Proof, VerifyingKey};
use ark_serialize::CanonicalDeserialize;
use arkworks_gadgets::{
	setup::{bridge, common::verify_groth16, mixer},
	utils::to_field_elements,
};
use sp_std::marker::PhantomData;

pub struct ArkworksMixerVerifierGroth16<E: PairingEngine>(PhantomData<E>);
pub struct ArkworksAnchorVerifierGroth16<E: PairingEngine, const M: usize>(PhantomData<E>);
pub struct ArkworksVAnchorVerifierGroth16<E: PairingEngine, const I: usize, const O: usize, const M: usize>(
	PhantomData<E>,
);

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

impl<E: PairingEngine, const M: usize> InstanceVerifier for ArkworksAnchorVerifierGroth16<E, M> {
	fn verify(public_inp_bytes: &[u8], proof_bytes: &[u8], vk_bytes: &[u8]) -> Result<bool, Error> {
		let public_input_field_elts = to_field_elements::<E::Fr>(public_inp_bytes).unwrap();
		let nullifier_hash = public_input_field_elts[0];
		let recipient = public_input_field_elts[1];
		let relayer = public_input_field_elts[2];
		let fee = public_input_field_elts[3];
		let refund = public_input_field_elts[4];
		let chain_id = public_input_field_elts[5];

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
	fn verify(public_inp_bytes: &[u8], proof_bytes: &[u8], vk_bytes: &[u8]) -> Result<bool, Error> {
		let public_input_field_elts = to_field_elements::<E::Fr>(public_inp_bytes).unwrap();
		let vk = VerifyingKey::<E>::deserialize(vk_bytes)?;
		let proof = Proof::<E>::deserialize(proof_bytes)?;
		let res = verify_groth16::<E>(&vk, &public_input_field_elts, &proof);
		Ok(res)
	}
}

use ark_bls12_381::Bls12_381;
pub type ArkworksBls381MixerVerifier = ArkworksMixerVerifierGroth16<Bls12_381>;
pub type ArkworksBls381BridgeVerifier = ArkworksAnchorVerifierGroth16<Bls12_381, 2>;
pub type ArkworksBls381VAnchor2x2Verifier = ArkworksVAnchorVerifierGroth16<Bls12_381, 2, 2, 2>;

use ark_bn254::Bn254;
pub type ArkworksBn254MixerVerifier = ArkworksMixerVerifierGroth16<Bn254>;
pub type ArkworksBn254BridgeVerifier = ArkworksAnchorVerifierGroth16<Bn254, 2>;
pub type ArkworksBn254VAnchor2x2Verifier = ArkworksVAnchorVerifierGroth16<Bn254, 2, 2, 2>;
