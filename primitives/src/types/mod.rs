pub mod runtime;
pub mod vanchor;

use codec::{Decode, Encode, MaxEncodedLen};
pub use ethabi::{encode, Token};
use frame_support::pallet_prelude::*;
use scale_info::TypeInfo;
use sp_runtime::traits::MaybeSerializeDeserialize;
use sp_std::vec::Vec;

// Deposit details used in hasher / verifier pallets for
// tracking the reserved deposits of maintainers of various
// parameters
#[derive(Clone, Default, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub struct DepositDetails<AccountId, Balance> {
	pub depositor: AccountId,
	pub deposit: Balance,
}

/// Hash functions for MerkleTree
#[cfg_attr(feature = "std", derive(Debug))]
#[derive(Clone, Encode, Decode, PartialEq, Eq, TypeInfo)]
pub enum HashFunction {
	PoseidonDefault,
	// Poseidon hash - (width, exponentiation)
	Poseidon(u8, u8),
	MiMC,
}

/// Different curve types
#[cfg_attr(feature = "std", derive(Debug))]
#[derive(Clone, Encode, Decode, PartialEq, Eq, TypeInfo)]
pub enum Curve {
	Bls381,
	Bn254,
	Curve25519,
}

/// Different curve types
#[cfg_attr(feature = "std", derive(Debug))]
#[derive(Clone, Encode, Decode, PartialEq, Eq, TypeInfo)]
pub enum Snark {
	Groth16,
	Marlin,
	Plonk,
}

#[cfg_attr(feature = "std", derive(Debug))]
#[derive(Clone, Encode, Decode, PartialEq, Eq, TypeInfo)]
pub enum Backend {
	Arkworks(Curve, Snark),
	Bulletproofs(Curve),
}

#[cfg_attr(feature = "std", derive(Debug))]
#[derive(Clone, Encode, Decode, PartialEq, Eq, TypeInfo)]
pub struct Setup {
	pub hasher: HashFunction,
	pub backend: Backend,
}

pub trait ElementTrait:
	Encode + Decode + Parameter + Default + Copy + TypeInfo + MaybeSerializeDeserialize
{
	/// converts type to byte slice
	fn to_bytes(&self) -> &[u8];
	/// converts type to Vec
	fn to_vec(&self) -> Vec<u8> {
		self.to_bytes().to_vec()
	}
	/// converts slice to type
	fn from_bytes(bytes: &[u8]) -> Self;
	/// converts Vec to type
	fn from_vec(vec: Vec<u8>) -> Self {
		Self::from_bytes(&vec)
	}

	fn is_zero(&self) -> bool {
		if self.to_vec().is_empty() {
			true
		} else {
			let vec = self.to_vec();
			let length = vec.len();
			let buf: Vec<u8> = Vec::with_capacity(length);
			buf == vec
		}
	}
}

#[allow(clippy::wrong_self_convention)]
pub trait IntoAbiToken {
	fn into_abi(&self) -> Token;
	fn encode_abi(&self) -> Vec<u8> {
		let token = self.into_abi();

		encode(&[token])
	}
}

impl IntoAbiToken for i128 {
	fn into_abi(&self) -> Token {
		let bytes = self.encode();
		let mut bytes32: [u8; 32] = [0; 32];
		for (i, byte) in bytes.iter().enumerate() {
			bytes32[i] = *byte;
		}
		Token::Int(bytes32.into())
	}
}

impl IntoAbiToken for u128 {
	fn into_abi(&self) -> Token {
		let bytes = self.encode();
		let mut bytes32: [u8; 32] = [0; 32];
		for (i, byte) in bytes.iter().enumerate() {
			bytes32[i] = *byte;
		}
		Token::Uint(bytes32.into())
	}
}

impl IntoAbiToken for [u8; 32] {
	fn into_abi(&self) -> Token {
		Token::Bytes(self.to_vec())
	}
}
