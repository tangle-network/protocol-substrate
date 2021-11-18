use crate::types::IntoAbiToken;
use ethabi::encode;
use tiny_keccak::{Hasher, Keccak};

pub fn keccak256_abi<T: IntoAbiToken>(input: &T) -> Vec<u8> {
	let token = input.into_abi();
	let encoded_input = encode(&[token]);
	let bytes: &[u8] = &encoded_input;
	let mut hasher = Keccak::v256();
	hasher.update(bytes);
	let mut res: [u8; 32] = [0; 32];
	hasher.finalize(&mut res);
	res.to_vec()
}
