use core::convert::TryInto;

use crate::*;
use ark_crypto_primitives::Error;
use ark_ec::PairingEngine;
use ark_ff::{BigInteger, PrimeField, Zero};
use ark_groth16::{Proof, VerifyingKey};
use ark_serialize::CanonicalDeserialize;
use arkworks_gadgets::{
	setup::{bridge, common::verify_groth16, mixer},
	utils::to_field_elements,
};
use sp_std::{marker::PhantomData, mem, prelude::*};

pub struct ArkworksMixerVerifierGroth16<E: PairingEngine>(PhantomData<E>);
pub struct ArkworksBridgeVerifierGroth16<E: PairingEngine>(PhantomData<E>);

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

impl<E: PairingEngine> InstanceVerifier for ArkworksBridgeVerifierGroth16<E> {
	fn verify(public_inp_bytes: &[u8], proof_bytes: &[u8], vk_bytes: &[u8]) -> Result<bool, Error> {
		let public_input_field_elts = to_field_elements::<E::Fr>(public_inp_bytes).unwrap();
		let nullifier_hash = public_input_field_elts[0];
		let recipient = public_input_field_elts[1];
		let relayer = public_input_field_elts[2];
		let fee = public_input_field_elts[3];
		let refund = public_input_field_elts[4];
		let chain_id = public_input_field_elts[5];
		let roots_len = public_input_field_elts[6]
			.into_repr()
			.to_bytes_le()
			.into_iter()
			.take(mem::size_of::<u64>())
			.collect::<Vec<_>>()
			.try_into()
			.unwrap();
		let roots_len = u64::from_le_bytes(roots_len) as usize;
		let mut roots = vec![E::Fr::zero(); roots_len];
		roots.copy_from_slice(&public_input_field_elts[7..roots_len]);
		// FIXME: why we do need to have a fixed size array of roots?
		//
		// TODO(@shekohex): change this to use the values form above
		// instead of the hardcoded roots.
		let public_inputs = bridge::get_public_inputs::<E::Fr, 1>(
			chain_id,
			nullifier_hash,
			[E::Fr::zero()],
			E::Fr::zero(),
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

use ark_bls12_381::Bls12_381;
pub type ArkworksBls381MixerVerifier = ArkworksMixerVerifierGroth16<Bls12_381>;
pub type ArkworksBls381BridgeVerifier = ArkworksBridgeVerifierGroth16<Bls12_381>;

use ark_bn254::Bn254;
pub type ArkworksBn254MixerVerifier = ArkworksMixerVerifierGroth16<Bn254>;
pub type ArkworksBn254BridgeVerifier = ArkworksBridgeVerifierGroth16<Bn254>;
