#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use frame_support::pallet_prelude::*;

// Deposit details used in hasher / verifier pallets for
// tracking the reserved deposits of maintainers of various
// parameters
#[derive(Clone, Default, Encode, Decode, Eq, PartialEq, RuntimeDebug)]
pub struct DepositDetails<AccountId, Balance> {
	pub depositor: AccountId,
	pub deposit: Balance,
}

/// Hash functions for MerkleTree
#[cfg_attr(feature = "std", derive(Debug))]
#[derive(Clone, Encode, Decode, PartialEq)]
pub enum HashFunction {
	PoseidonDefault,
	// Poseidon hash - (width, exponentiation)
	Poseidon(u8, u8),
	MiMC,
}

/// Different curve types
#[cfg_attr(feature = "std", derive(Debug))]
#[derive(Clone, Encode, Decode, PartialEq)]
pub enum Curve {
	Bls381,
	Bn254,
	Curve25519,
}

/// Different curve types
#[cfg_attr(feature = "std", derive(Debug))]
#[derive(Clone, Encode, Decode, PartialEq)]
pub enum Snark {
	Groth16,
	Marlin,
	Plonk,
}

#[cfg_attr(feature = "std", derive(Debug))]
#[derive(Clone, Encode, Decode, PartialEq)]
pub enum Backend {
	Arkworks(Curve, Snark),
	Bulletproofs(Curve),
}

#[cfg_attr(feature = "std", derive(Debug))]
#[derive(Clone, Encode, Decode, PartialEq)]
pub struct Setup {
	pub hasher: HashFunction,
	pub backend: Backend,
}
