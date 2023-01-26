use crate::*;
use ark_bn254::{Bn254, Fr};
use ark_crypto_primitives::Error;
use ark_groth16::{
	verify_proof, PreparedVerifyingKey, Proof as ArkProof, VerifyingKey as ArkVerifyingKey,
};
use ark_serialize::CanonicalDeserialize;
use ark_std::vec::Vec;
use arkworks_native_gadgets::to_field_elements;
use sp_core::U256;

pub struct CircomVerifierBn254;

#[derive(Debug)]
pub enum CircomError {
	InvalidVerifyingKeyBytes,
	InvalidProofBytes,
	InvalidBuilderConfig,
	ProvingFailure,
	VerifyingFailure,
	ParameterGenerationFailure,
}

impl ark_std::error::Error for CircomError {}

impl core::fmt::Display for CircomError {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		match self {
			CircomError::InvalidVerifyingKeyBytes => write!(f, "Invalid verifying key bytes"),
			CircomError::InvalidProofBytes => write!(f, "Invalid proof bytes"),
			CircomError::InvalidBuilderConfig => write!(f, "Invalid builder config"),
			CircomError::ProvingFailure => write!(f, "Proving failure"),
			CircomError::VerifyingFailure => write!(f, "Verifying failure"),
			CircomError::ParameterGenerationFailure => write!(f, "Parameter generation failure"),
		}
	}
}

pub fn verify_groth16(
	vk: &PreparedVerifyingKey<Bn254>,
	public_inputs: &[Fr],
	proof: &ArkProof<Bn254>,
) -> Result<bool, Error> {
	let res = verify_proof(vk, proof, public_inputs)?;
	Ok(res)
}

impl InstanceVerifier for CircomVerifierBn254 {
	fn verify(public_inp_bytes: &[u8], proof_bytes: &[u8], vk_bytes: &[u8]) -> Result<bool, Error> {
		let public_input_field_elts = to_field_elements::<Fr>(public_inp_bytes)?;
		let vk = ArkVerifyingKey::deserialize(vk_bytes)?;
		let proof = ArkProof::deserialize(proof_bytes)?;
		let res = verify_groth16(&vk.into(), &public_input_field_elts, &proof)?;
		Ok(res)
	}
}
