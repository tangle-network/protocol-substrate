use crate::*;
use ark_bn254::{Bn254, Fr};
use ark_crypto_primitives::Error;
use thiserror::Error;
use ark_groth16::{
	verify_proof as ark_verify_proof,
	PreparedVerifyingKey,
	Proof as ArkProof,
	VerifyingKey as ArkVerifyingKey,
	create_proof_with_reduction_and_matrices,
	prepare_verifying_key,
	ProvingKey,
	VerifyingKey
};
use ark_circom::{CircomReduction, WitnessCalculator};
use std::{
	sync::Mutex,
	// convert::{TryFrom, TryInto},
};
use ark_std::{rand::thread_rng, UniformRand};
use num_bigint::{BigInt};
use cfg_if::cfg_if;
use ark_relations::r1cs::{ConstraintMatrices, SynthesisError};
use ark_serialize::CanonicalDeserialize;
use wasmer::{Module, Store};
use once_cell::sync::OnceCell;
// use ark_std::vec::Vec;
use arkworks_native_gadgets::to_field_elements;
// use sp_core::U256;

#[derive(Error, Debug)]
pub enum ProofError {
	#[error("Error reading circuit key: {0}")]
	CircuitKeyError(#[from] std::io::Error),
	#[error("Error producing witness: {0}")]
	WitnessError(color_eyre::Report),
	#[error("Error producing proof: {0}")]
	SynthesisError(#[from] SynthesisError),
}

#[cfg(not(target_arch = "wasm32"))]
static WITNESS_CALCULATOR: OnceCell<Mutex<WitnessCalculator>> = OnceCell::new();
// Initializes the witness calculator using a bytes vector
#[cfg(not(target_arch = "wasm32"))]
pub fn circom_from_raw(wasm_buffer: Vec<u8>) -> &'static Mutex<WitnessCalculator> {
	WITNESS_CALCULATOR.get_or_init(|| {
		let store = Store::default();
		let module = Module::new(&store, wasm_buffer).unwrap();
		let result =
			WitnessCalculator::from_module(module).expect("Failed to create witness calculator");
		Mutex::new(result)
	})
}

// Initializes the witness calculator
#[cfg(not(target_arch = "wasm32"))]
pub fn circom_from_folder(wasm_path: &str) -> &'static Mutex<WitnessCalculator> {
	// We read the wasm file
	let wasm_buffer = std::fs::read(wasm_path).unwrap();
	circom_from_raw(wasm_buffer)
}

pub fn generate_proof<const N: usize>(
	#[cfg(not(target_arch = "wasm32"))] witness_calculator: &Mutex<WitnessCalculator>,
	#[cfg(target_arch = "wasm32")] witness_calculator: &mut WitnessCalculator,
	proving_key: &(ProvingKey<Bn254>, ConstraintMatrices<Fr>),
	witness: [(&str, Vec<BigInt>); N],
) -> Result<(ArkProof<Bn254>, Vec<Fr>), ProofError> {
	let inputs = witness
		.iter()
		.map(|(name, values)| (name.to_string(), values.clone()));

	println!("inputs {:?}", inputs);

	cfg_if! {
		if #[cfg(target_arch = "wasm32")] {
			let full_assignment = witness_calculator
			.calculate_witness_element::<Bn254, _>(inputs, false)
			.map_err(ProofError::WitnessError)?;
		} else {
			let full_assignment = witness_calculator
			.lock()
			.expect("witness_calculator mutex should not get poisoned")
			.calculate_witness_element::<Bn254, _>(inputs, false)
			.map_err(ProofError::WitnessError)?;
		}
	}

	// Random Values
	let mut rng = thread_rng();
	let r = Fr::rand(&mut rng);
	let s = Fr::rand(&mut rng);

	let proof = create_proof_with_reduction_and_matrices::<_, CircomReduction>(
		&proving_key.0,
		r,
		s,
		&proving_key.1,
		proving_key.1.num_instance_variables,
		proving_key.1.num_constraints,
		full_assignment.as_slice(),
	)?;

	Ok((proof, full_assignment))
}

/// Verifies a given RLN proof
///
/// # Errors
///
/// Returns a [`ProofError`] if verifying fails. Verification failure does not
/// necessarily mean the proof is incorrect.
pub fn verify_proof(
	verifying_key: &VerifyingKey<Bn254>,
	proof: &ArkProof<Bn254>,
	inputs: Vec<Fr>,
) -> Result<bool, ProofError> {
	// Check that the proof is valid
	let pvk = prepare_verifying_key(verifying_key);
	//let pr: ArkProof<Curve> = (*proof).into();

	let verified = ark_verify_proof(&pvk, proof, &inputs)?;

	Ok(verified)
}

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
		let public_input_field_elts = to_field_elements::<Fr>(public_inp_bytes)?;
		let vk = ArkVerifyingKey::deserialize(vk_bytes)?;
		let proof = ArkProof::deserialize(proof_bytes)?;
		let res = verify_groth16(&vk.into(), &public_input_field_elts, &proof)?;
		Ok(res)
	}
}
