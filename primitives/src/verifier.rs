use ark_crypto_primitives::Error;
use frame_support::pallet_prelude::DispatchError;

use ark_ec::PairingEngine;
use ark_groth16::{Proof, VerifyingKey};
use ark_serialize::CanonicalDeserialize;
use arkworks_gadgets::{setup::common::verify_groth16, utils::to_field_elements};

// A trait meant to be implemented over a zero-knowledge verifier function.
pub trait InstanceVerifier {
	fn pack_public_inputs(inputs: &[Vec<u8>]) -> Vec<u8>;
	fn pack_public_inputs_and_verify(inputs: &[Vec<u8>], proof_bytes: &[u8], vk_bytes: &[u8]) -> Result<bool, Error>;
	fn verify<E: PairingEngine>(public_inp_bytes: &[u8], proof_bytes: &[u8], vk_bytes: &[u8]) -> Result<bool, Error> {
		let public_input_field_elts = to_field_elements::<E::Fr>(public_inp_bytes)?;
		let vk = VerifyingKey::<E>::deserialize(vk_bytes)?;
		let proof = Proof::<E>::deserialize(proof_bytes)?;
		let res = verify_groth16::<E>(&vk, &public_input_field_elts, &proof);
		Ok(res)
	}
}

// A trait meant to be implemented by a pallet
pub trait VerifierModule {
	fn pack_public_inputs(inputs: &[Vec<u8>]) -> Vec<u8>;
	fn pack_public_inputs_and_verify(inputs: &[Vec<u8>], proof_bytes: &[u8]) -> Result<bool, DispatchError>;
}
