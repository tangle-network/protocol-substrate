// A trait meant to be implemented over a zero-knowledge
// verifier function.
pub trait InstanceVerifier {
	fn verify(proof: &[u8], params: &[u8]) -> bool;
}

// A trait meant to be implemented by a pallet
pub trait VerifierModule {
	fn verify(data: &[u8]) ->bool;
}