use codec::{Decode, Encode};
use frame_support::pallet_prelude::*;
use scale_info::TypeInfo;
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
