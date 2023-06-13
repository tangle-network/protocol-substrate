use crate::*;
use ark_bn254::{Bn254, Fr, G1Affine, G2Affine};
use ark_crypto_primitives::Error;
use ark_ff::{BigInteger, FromBytes, PrimeField};
use ark_groth16::{
	verify_proof as ark_verify_proof, PreparedVerifyingKey, Proof as ArkProof,
	VerifyingKey as ArkVerifyingKey,
};
use ark_serialize::CanonicalDeserialize;
use ark_std::Zero;
use arkworks_native_gadgets::to_field_elements;
use ethabi::ethereum_types::U256;

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
	let res = ark_verify_proof(vk, proof, public_inputs)?;
	Ok(res)
}

impl InstanceVerifier for CircomVerifierBn254 {
	fn verify(public_inp_bytes: &[u8], proof_bytes: &[u8], vk_bytes: &[u8]) -> Result<bool, Error> {
		let public_input_field_elts = match to_field_elements::<Fr>(public_inp_bytes) {
			Ok(v) => v,
			Err(e) => {
				frame_support::log::error!(
					"Failed to convert public input bytes to field elements: {e:?}",
				);
				return Err(e)
			},
		};
		let vk = match ArkVerifyingKey::deserialize(vk_bytes) {
			Ok(v) => v,
			Err(e) => {
				frame_support::log::error!("Failed to deserialize verifying key: {e:?}");
				return Err(e.into())
			},
		};
		let proof = match ArkProof::deserialize(proof_bytes) {
			Ok(v) => v,
			Err(e) => {
				frame_support::log::error!("Failed to deserialize proof: {e:?}");
				return Err(e.into())
			},
		};
		let res = match verify_groth16(&vk.into(), &public_input_field_elts, &proof) {
			Ok(v) => v,
			Err(e) => {
				frame_support::log::error!("Failed to verify proof: {e:?}");
				return Err(e)
			},
		};

		Ok(res)
	}
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct G1 {
	pub x: U256,
	pub y: U256,
}

impl From<G1> for G1Affine {
	fn from(src: G1) -> G1Affine {
		let x: ark_bn254::Fq = u256_to_point(src.x);
		let y: ark_bn254::Fq = u256_to_point(src.y);
		let inf = x.is_zero() && y.is_zero();
		G1Affine::new(x, y, inf)
	}
}

type G1Tup = (U256, U256);

impl G1 {
	pub fn as_tuple(&self) -> (U256, U256) {
		(self.x, self.y)
	}
}

impl From<&G1Affine> for G1 {
	fn from(p: &G1Affine) -> Self {
		Self { x: point_to_u256(p.x), y: point_to_u256(p.y) }
	}
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct G2 {
	pub x: [U256; 2],
	pub y: [U256; 2],
}

impl From<G2> for G2Affine {
	fn from(src: G2) -> G2Affine {
		let c0 = u256_to_point(src.x[0]);
		let c1 = u256_to_point(src.x[1]);
		let x = ark_bn254::Fq2::new(c0, c1);

		let c0 = u256_to_point(src.y[0]);
		let c1 = u256_to_point(src.y[1]);
		let y = ark_bn254::Fq2::new(c0, c1);

		let inf = x.is_zero() && y.is_zero();
		G2Affine::new(x, y, inf)
	}
}

type G2Tup = ([U256; 2], [U256; 2]);

impl G2 {
	// NB: Serialize the c1 limb first.
	pub fn as_tuple(&self) -> G2Tup {
		([self.x[1], self.x[0]], [self.y[1], self.y[0]])
	}
}

impl From<&G2Affine> for G2 {
	fn from(p: &G2Affine) -> Self {
		Self {
			x: [point_to_u256(p.x.c0), point_to_u256(p.x.c1)],
			y: [point_to_u256(p.y.c0), point_to_u256(p.y.c1)],
		}
	}
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Proof {
	pub a: G1,
	pub b: G2,
	pub c: G1,
}

impl Proof {
	pub fn as_tuple(&self) -> (G1Tup, G2Tup, G1Tup) {
		(self.a.as_tuple(), self.b.as_tuple(), self.c.as_tuple())
	}
}

impl From<ark_groth16::Proof<Bn254>> for Proof {
	fn from(proof: ark_groth16::Proof<Bn254>) -> Self {
		Self { a: G1::from(&proof.a), b: G2::from(&proof.b), c: G1::from(&proof.c) }
	}
}

impl From<Proof> for ark_groth16::Proof<Bn254> {
	fn from(src: Proof) -> ark_groth16::Proof<Bn254> {
		ark_groth16::Proof { a: src.a.into(), b: src.b.into(), c: src.c.into() }
	}
}

// Helper for converting a PrimeField to its U256 representation for Ethereum compatibility
fn u256_to_point<F: PrimeField>(point: U256) -> F {
	let mut buf = [0; 32];
	point.to_little_endian(&mut buf);
	let bigint = F::BigInt::read(&buf[..]).expect("always works");
	F::from_repr(bigint).expect("alwasy works")
}

// Helper for converting a PrimeField to its U256 representation for Ethereum compatibility
// (U256 reads data as big endian)
fn point_to_u256<F: PrimeField>(point: F) -> U256 {
	let point = point.into_repr();
	let point_bytes = point.to_bytes_be();
	U256::from(&point_bytes[..])
}
