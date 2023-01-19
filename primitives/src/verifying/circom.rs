use crate::*;
use ark_circom::circom;
use ark_crypto_primitives::{Error, SNARK};
use ark_ec::PairingEngine;
use ark_groth16::{Groth16, Proof as ArkProof, VerifyingKey as ArkVerifyingKey, PreparedVerifyingKey};
use ark_groth16::{
    prepare_verifying_key, verify_proof,
};
use ark_serialize::CanonicalDeserialize;
use arkworks_native_gadgets::to_field_elements;
use ark_circom::ethereum::Proof;
use ark_circom::ethereum::VerifyingKey;
use sp_std::marker::PhantomData;
pub struct CircomVerifierGroth16<E: PairingEngine>(PhantomData<E>);

pub fn verify_groth16<E: PairingEngine>(
	vk: &PreparedVerifyingKey<E>,
	public_inputs: &[E::Fr],
	proof: &ArkProof<E>,
) -> Result<bool, Error> {
	let res = verify_proof(vk, proof, public_inputs)?;
	Ok(res)
}

impl<E: PairingEngine> InstanceVerifier for CircomVerifierGroth16<E> {
	fn verify(public_inp_bytes: &[u8], proof_bytes: &[u8], vk_bytes: &[u8]) -> Result<bool, Error> {
		let public_input_field_elts = to_field_elements::<E::Fr>(public_inp_bytes)?;
        let circom_vk = VerifyingKey::from(vk_bytes);
        let circom_proof = Proof::from(proof_bytes);
		let vk = ArkVerifyingKey::<E>::from(circom_vk)?;
		let proof = ArkProof::<E>::from(circom_proof)?;
		let res = verify_groth16::<E>(&vk.into(), &public_input_field_elts, &proof)?;
		Ok(res)
	}
}

use ark_bn254::Bn254;
pub type CircomVerifierBn254 = CircomVerifierGroth16<Bn254>;

