use ark_crypto_primitives::Error;
use codec::Encode;
use frame_support::pallet_prelude::DispatchError;

// A trait meant to be implemented over a zero-knowledge verifier function.
pub trait InstanceVerifier {
	fn encode_public_inputs<E: Encode>(inputs: &[E]) -> Vec<u8>;
	fn verify(pub_inps: &[u8], proof: &[u8], params: &[u8]) -> Result<bool, Error>;
}

// A trait meant to be implemented by a pallet
pub trait VerifierModule {
	fn encode_public_inputs<E: Encode>(inputs: &[E]) -> Vec<u8>;
	fn verify(pub_inps: &[u8], data: &[u8]) -> Result<bool, DispatchError>;
}
