use crate::*;
use ark_crypto_primitives::{Error, commitment};
use ark_ec::PairingEngine;
use ark_ff::Zero;
use ark_groth16::{Proof, VerifyingKey};
use ark_serialize::CanonicalDeserialize;
use arkworks_utils::utils::{common::verify_groth16, to_field_elements};
use sp_std::marker::PhantomData;
use ark_std::{vec, vec::Vec};
pub struct ArkworksMixerVerifierGroth16<E: PairingEngine>(PhantomData<E>);
pub struct ArkworksBridgeVerifierGroth16<E: PairingEngine>(PhantomData<E>);

impl<E: PairingEngine> InstanceVerifier for ArkworksMixerVerifierGroth16<E> {
	fn verify(public_inp_bytes: &[u8], proof_bytes: &[u8], vk_bytes: &[u8]) -> Result<bool, Error> {
		let public_input_field_elts = to_field_elements::<E::Fr>(public_inp_bytes).unwrap();
		let vk = VerifyingKey::<E>::deserialize(vk_bytes)?;
		let proof = Proof::<E>::deserialize(proof_bytes)?;
		let res = verify_groth16::<E>(&vk, &public_input_field_elts, &proof);
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
		let commitment = public_input_field_elts[6];

		const M: usize = 2;
		let mut roots = [E::Fr::zero(); M];
		for (i, root) in roots.iter_mut().enumerate() {
			*root = *public_input_field_elts.get(i + 7).unwrap_or(&E::Fr::zero());
		}

		let public_inputs = get_public_inputs::<E::Fr, M>(
			chain_id,
			nullifier_hash,
			roots,
			roots[0],
			recipient,
			relayer,
			fee,
			refund,
			commitment,
		);
		let vk = VerifyingKey::<E>::deserialize(vk_bytes)?;
		let proof = Proof::<E>::deserialize(proof_bytes)?;
		let res = verify_groth16::<E>(&vk, &public_inputs, &proof);
		Ok(res)
	}
}

use ark_ff::PrimeField;
#[allow(clippy::too_many_arguments)]
pub fn get_public_inputs<F: PrimeField, const M: usize>(
	chain_id: F,
	nullifier_hash: F,
	roots: [F; M],
	root: F,
	recipient: F,
	relayer: F,
	fee: F,
	refund: F,
	commitment: F,
) -> Vec<F> {
	vec![
		chain_id,
		nullifier_hash,
		roots[M - 2],
		roots[M - 1],
		root,
		recipient,
		relayer,
		fee,
		refund,
		commitment,
	]
}

use ark_bls12_381::Bls12_381;
pub type ArkworksBls381MixerVerifier = ArkworksMixerVerifierGroth16<Bls12_381>;
pub type ArkworksBls381BridgeVerifier = ArkworksBridgeVerifierGroth16<Bls12_381>;

use ark_bn254::Bn254;
pub type ArkworksBn254MixerVerifier = ArkworksMixerVerifierGroth16<Bn254>;
pub type ArkworksBn254BridgeVerifier = ArkworksBridgeVerifierGroth16<Bn254>;
