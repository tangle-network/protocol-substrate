use super::ethereum_circom::{Proof, VerifyingKey, G1, G2};
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

#[cfg(test)]
mod tests {
	use super::*;
	use crate::verifying::ethereum_circom::*;
	use sp_core::U256;

	#[test]
	fn verifying_key_serialize_deserialize() {
		let vk = VerifyingKey {
			alpha1: G1 { x: U256::from(1), y: U256::from(2) },
			beta2: G2 { x: [U256::from(3), U256::from(4)], y: [U256::from(5), U256::from(6)] },
			gamma2: G2 { x: [U256::from(7), U256::from(8)], y: [U256::from(9), U256::from(10)] },
			delta2: G2 { x: [U256::from(11), U256::from(12)], y: [U256::from(13), U256::from(14)] },
			ic: vec![
				G1 { x: U256::from(15), y: U256::from(16) },
				G1 { x: U256::from(17), y: U256::from(18) },
				G1 { x: U256::from(19), y: U256::from(20) },
				G1 { x: U256::from(21), y: U256::from(22) },
				G1 { x: U256::from(23), y: U256::from(24) },
			],
		};

		let vk_bytes = vk.to_bytes();
		let vk2 = parse_vk_bytes_to_circom_vk(&vk_bytes).unwrap();
		assert_eq!(vk.alpha1, vk2.alpha1);
		assert_eq!(vk.beta2, vk2.beta2);
		assert_eq!(vk.gamma2, vk2.gamma2);
		assert_eq!(vk.delta2, vk2.delta2);
		assert_eq!(vk.ic, vk2.ic);
	}
}
