pub trait InstanceHasher {
	fn hash(data: &[u8], params: &[u8]) -> Vec<u8>;
}

pub trait HasherModule {
	fn hash(data: &[u8]) -> Vec<u8>;
}