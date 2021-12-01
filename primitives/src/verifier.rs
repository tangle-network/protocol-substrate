use ark_crypto_primitives::Error;
use frame_support::pallet_prelude::DispatchError;

use ark_ec::PairingEngine;
use ark_groth16::{Proof, VerifyingKey};
use ark_serialize::CanonicalDeserialize;
use arkworks_gadgets::{setup::common::verify_groth16, utils::to_field_elements};

// A trait meant to be implemented over a zero-knowledge verifier function.
pub trait InstanceVerifier {
	fn verify(pub_inps: &[u8], proof: &[u8], params: &[u8]) -> Result<bool, Error>;
}

// A trait meant to be implemented by a pallet
pub trait VerifierModule {
	fn verify(pub_inps: &[u8], data: &[u8]) -> Result<bool, DispatchError>;
}
