// A trait meant to be implemented over a hash function instance
pub trait InstanceHasher {
	fn hash(data: &[u8], params: &[u8]) -> Vec<u8>;
}

// A trait meant to be implemented by a pallet
pub trait HasherModule {
	fn hash(data: &[u8]) -> Vec<u8>;
}