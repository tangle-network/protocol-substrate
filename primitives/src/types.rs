use codec::{Decode, Encode};
use frame_support::pallet_prelude::*;
use scale_info::TypeInfo;
use sp_std::vec::Vec;

// Deposit details used in hasher / verifier pallets for
// tracking the reserved deposits of maintainers of various
// parameters
#[derive(Clone, Default, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo)]
pub struct DepositDetails<AccountId, Balance> {
	pub depositor: AccountId,
	pub deposit: Balance,
}

/// Hash functions for MerkleTree
#[cfg_attr(feature = "std", derive(Debug))]
#[derive(Clone, Encode, Decode, PartialEq, TypeInfo)]
pub enum HashFunction {
	PoseidonDefault,
	// Poseidon hash - (width, exponentiation)
	Poseidon(u8, u8),
	MiMC,
}

/// Different curve types
#[cfg_attr(feature = "std", derive(Debug))]
#[derive(Clone, Encode, Decode, PartialEq, TypeInfo)]
pub enum Curve {
	Bls381,
	Bn254,
	Curve25519,
}

/// Different curve types
#[cfg_attr(feature = "std", derive(Debug))]
#[derive(Clone, Encode, Decode, PartialEq, TypeInfo)]
pub enum Snark {
	Groth16,
	Marlin,
	Plonk,
}

#[cfg_attr(feature = "std", derive(Debug))]
#[derive(Clone, Encode, Decode, PartialEq, TypeInfo)]
pub enum Backend {
	Arkworks(Curve, Snark),
	Bulletproofs(Curve),
}

#[cfg_attr(feature = "std", derive(Debug))]
#[derive(Clone, Encode, Decode, PartialEq, TypeInfo)]
pub struct Setup {
	pub hasher: HashFunction,
	pub backend: Backend,
}

pub trait ElementTrait: Encode + Decode + Parameter + Default + Copy + TypeInfo {
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
			return true;
		} else {
			let vec = self.to_vec();
			let length = vec.len();
			let buf: Vec<u8> = Vec::with_capacity(length);
			return buf == vec;
		}
	}
}
