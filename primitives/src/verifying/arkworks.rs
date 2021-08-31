use crate::*;
use ark_crypto_primitives::Error;
use ark_ec::PairingEngine;
use ark_groth16::{Proof, VerifyingKey};
use ark_serialize::CanonicalDeserialize;
use arkworks_gadgets::{
	setup::{common::verify_groth16, mixer::get_public_inputs},
	utils::to_field_elements,
};
use sp_std::marker::PhantomData;

pub struct ArkworksMixerVerifierGroth16<E: PairingEngine>(PhantomData<E>);

impl<E: PairingEngine> InstanceVerifier for ArkworksMixerVerifierGroth16<E> {
	fn verify(public_inp_bytes: &[u8], proof_bytes: &[u8], vk_bytes: &[u8]) -> Result<bool, Error> {
		let public_input_field_elts = to_field_elements::<E::Fr>(public_inp_bytes).unwrap();
		let public_inputs = get_public_inputs::<E::Fr>(
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

use ark_bls12_381::Bls12_381;
pub type ArkworksBls381Verifier = ArkworksMixerVerifierGroth16<Bls12_381>;

use ark_bn254::Bn254;
pub type ArkworksBn254Verifier = ArkworksMixerVerifierGroth16<Bn254>;
