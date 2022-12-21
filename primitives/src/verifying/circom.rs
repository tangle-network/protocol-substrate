use super::ethereum_circom::{Proof, VerifyingKey, G1, G2};
use crate::*;
use ark_bn254::{Bn254, Fr};
use ark_crypto_primitives::{Error, SNARK};
use ark_ec::PairingEngine;
use ark_groth16::{
	prepare_verifying_key, verify_proof, Groth16, PreparedVerifyingKey, Proof as ArkProof,
	VerifyingKey as ArkVerifyingKey,
};
use ark_serialize::CanonicalDeserialize;
use ark_std::{string::ToString, vec::Vec};
use arkworks_native_gadgets::to_field_elements;
use byteorder::{BigEndian, ByteOrder};
use sp_core::U256;
use sp_std::marker::PhantomData;

pub struct CircomVerifierBn254;

#[derive(Debug)]
pub enum CircomError {
	InvalidVerifyingKeyBytes,
	InvalidProofBytes,
}

impl ark_std::error::Error for CircomError {}

impl core::fmt::Display for CircomError {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		match self {
			CircomError::InvalidVerifyingKeyBytes => write!(f, "Invalid verifying key bytes"),
			CircomError::InvalidProofBytes => write!(f, "Invalid proof bytes"),
		}
	}
}

macro_rules! read_to_u256 {
	($arr:expr) => {{
		let x0 = BigEndian::read_u64(&$arr[0..8]);
		let x1 = BigEndian::read_u64(&$arr[8..16]);
		let x2 = BigEndian::read_u64(&$arr[16..24]);
		let x3 = BigEndian::read_u64(&$arr[24..32]);
		U256([x0, x1, x2, x3])
	}};
}

macro_rules! read_to_G1 {
	($arr:expr) => {{
		let x = read_to_u256!(&$arr[0..32]);
		let y = read_to_u256!(&$arr[32..64]);
		G1 { x, y }
	}};
}

macro_rules! read_to_G2 {
	($arr:expr) => {{
		let x0 = read_to_u256!(&$arr[0..32]);
		let y0 = read_to_u256!(&$arr[32..64]);
		let x1 = read_to_u256!(&$arr[64..96]);
		let y1 = read_to_u256!(&$arr[96..128]);
		G2 { x: [x0, x1], y: [y0, y1] }
	}};
}

macro_rules! read_to_ic {
	($arr:expr) => {{
		let mut ic = Vec::new();
		let mut temp_arr = $arr.to_vec();
		while !temp_arr.is_empty() {
			let x0 = read_to_u256!(&temp_arr[0..32]);
			let y0 = read_to_u256!(&temp_arr[32..64]);
			ic.push(G1 { x: x0, y: y0 });
			temp_arr = temp_arr[64..].to_vec();
		}

		ic
	}};
}

// pub struct VerifyingKey {
//     pub alpha1: G1, (x: U256, y: U256)
//     pub beta2: G2,  [(x: U256, y: U256); 2]
//     pub gamma2: G2, [(x: U256, y: U256); 2]
//     pub delta2: G2, [(x: U256, y: U256); 2]
//     pub ic: Vec<G1>, Vec<(x: U256, y: U256)>
// }
pub fn parse_vk_bytes_to_circom_vk(vk_bytes: &[u8]) -> Result<VerifyingKey, Error> {
	if vk_bytes.len() < 448 {
		return Err(CircomError::InvalidVerifyingKeyBytes.into())
	}

	let circom_vk = VerifyingKey {
		alpha1: read_to_G1!(vk_bytes[0..64]),
		beta2: read_to_G2!(vk_bytes[64..192]),
		gamma2: read_to_G2!(vk_bytes[192..320]),
		delta2: read_to_G2!(vk_bytes[320..448]),
		ic: read_to_ic!(vk_bytes[448..]),
	};

	Ok(circom_vk)
}

// pub struct Proof {
//     pub a: G1, (x: U256, y: U256)
//     pub b: G2, [(x: U256, y: U256); 2]
//     pub c: G1, (x: U256, y: U256)
// }
pub fn parse_proof_to_circom_proof(proof_bytes: &[u8]) -> Result<Proof, Error> {
	if proof_bytes.len() != 192 {
		return Err(CircomError::InvalidProofBytes.into())
	}

	let circom_proof = Proof {
		a: read_to_G1!(proof_bytes[0..64]),
		b: read_to_G2!(proof_bytes[64..192]),
		c: read_to_G1!(proof_bytes[192..256]),
	};

	Ok(circom_proof)
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

		let circom_vk = parse_vk_bytes_to_circom_vk(vk_bytes)?;
		let circom_proof = parse_proof_to_circom_proof(proof_bytes)?;
		let vk = ArkVerifyingKey::<Bn254>::from(circom_vk);
		let proof = ArkProof::<Bn254>::from(circom_proof);
		let res = verify_groth16(&vk.into(), &public_input_field_elts, &proof)?;
		Ok(res)
	}
}
