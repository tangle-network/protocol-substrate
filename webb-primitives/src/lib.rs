// Copyright (C) 2020-2021 Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

#![cfg_attr(not(feature = "std"), no_std)]
#![warn(missing_docs)]
// NOTE: needed to silence warnings about generated code in `decl_runtime_apis`
#![allow(clippy::too_many_arguments, clippy::unnecessary_mut_passed)]

//! Primitives for WEBB protocol.
//!
//! The crate contains shared data types used by WEBB protocol and documentation (in a form of
//! code) for building a WEBB light client.
//!
//! WEBB is a gadget that runs alongside another finality gadget (for instance GRANDPA).
//! For simplicity (and the initially intended use case) the documentation says GRANDPA in places
//! where a more abstract "Finality Gadget" term could be used, but there is no reason why WEBB
//! wouldn't run with some other finality scheme.
//! WEBB validator set is supposed to be tracking the Finality Gadget validator set, but note that
//! it will use a different set of keys. For Polkadot use case we plan to use `secp256k1` for WEBB,
//! while GRANDPA uses `ed25519`.

mod commitment;
pub mod mmr;
pub mod witness;
pub mod dkg;

pub use commitment::{Commitment, SignedCommitment, VersionedCommitment};

use codec::{Codec, Decode, Encode};
use sp_core::H256;
use sp_std::prelude::*;

/// Key type for WEBB module.
pub const KEY_TYPE: sp_application_crypto::KeyTypeId = sp_application_crypto::KeyTypeId(*b"webb");

/// WEBB cryptographic types
///
/// This module basically introduces three crypto types:
/// - `crypto::Pair`
/// - `crypto::Public`
/// - `crypto::Signature`
///
/// Your code should use the above types as concrete types for all crypto related
/// functionality.
///
/// The current underlying crypto scheme used is ECDSA. This can be changed,
/// without affecting code restricted against the above listed crypto types.
pub mod crypto {
	use sp_keystore::{SyncCryptoStore, Error};
	use sc_keystore::LocalKeystore;
	use crate::dkg::keystore::{MultiPartyECDSAKeyStore, Keys};
	use curv::{BigInt, elliptic::curves::{secp256_k1::Secp256k1Scalar, traits::{ECPoint, ECScalar}}};
	use curv::arithmetic::Converter;
	use sp_application_crypto::{app_crypto, ecdsa};
	app_crypto!(ecdsa, crate::KEY_TYPE);

	/// Identity of a WEBB authority using ECDSA as its crypto.
	pub type AuthorityId = Public;

	/// Signature for a WEBB authority using ECDSA as its crypto.
	pub type AuthoritySignature = Signature;

	impl MultiPartyECDSAKeyStore for LocalKeystore {
		fn generate(&self, index: usize) -> Result<Keys, Error> {
			let public: ecdsa::Public = SyncCryptoStore::ecdsa_generate_new(self, crate::KEY_TYPE, None)?;
			let key_pair = self.key_pair::<Pair>(&public.into())?
				.unwrap_or_else(|| panic!("No key pair found for public key"));
			let secret_key = key_pair.0.seed();
			let secret_key_slice: &[u8] = &secret_key[..];
			let secret_key_bigint = BigInt::from_bytes(secret_key_slice);
			let secret_key_scalar: Secp256k1Scalar = <Secp256k1Scalar as ECScalar>::from(&secret_key_bigint);
			Ok(Keys::create_from(secret_key_scalar, index))
		}
	}
}

/// The `ConsensusEngineId` of WEBB.
pub const WEBB_ENGINE_ID: sp_runtime::ConsensusEngineId = *b"WEBB";

/// Authority set id starts with zero at genesis
pub const GENESIS_AUTHORITY_SET_ID: u64 = 0;

/// A typedef for validator set id.
pub type ValidatorSetId = u64;

/// A set of WEBB authorities, a.k.a. validators.
#[derive(Decode, Encode, Debug, PartialEq, Clone)]
#[cfg_attr(feature = "scale-info", derive(scale_info::TypeInfo))]
pub struct ValidatorSet<AuthorityId> {
	/// Public keys of the validator set elements
	pub validators: Vec<AuthorityId>,
	/// Identifier of the validator set
	pub id: ValidatorSetId,
}

impl<AuthorityId> ValidatorSet<AuthorityId> {
	/// Return an empty validator set with id of 0.
	pub fn empty() -> Self {
		Self {
			validators: Default::default(),
			id: Default::default(),
		}
	}
}

/// The index of an authority.
pub type AuthorityIndex = u32;

/// The type used to represent an MMR root hash.
pub type MmrRootHash = H256;

/// A consensus log item for WEBB.
#[derive(Decode, Encode)]
#[cfg_attr(feature = "scale-info", derive(scale_info::TypeInfo))]
pub enum ConsensusLog<AuthorityId: Codec> {
	/// The authorities have changed.
	#[codec(index = 1)]
	AuthoritiesChange(ValidatorSet<AuthorityId>),
	/// Disable the authority with given index.
	#[codec(index = 2)]
	OnDisabled(AuthorityIndex),
	/// MMR root hash.
	#[codec(index = 3)]
	MmrRoot(MmrRootHash),
}

/// WEBB vote message.
///
/// A vote message is a direct vote created by a WEBB node on every voting round
/// and is gossiped to its peers.
#[derive(Debug, Decode, Encode)]
#[cfg_attr(feature = "scale-info", derive(scale_info::TypeInfo))]
pub struct VoteMessage<Hash, Number, Id, Signature> {
	/// Commit to information extracted from a finalized block
	pub commitment: Commitment<Number, Hash>,
	/// Node authority id
	pub id: Id,
	/// Node signature
	pub signature: Signature,
}

sp_api::decl_runtime_apis! {
	/// API necessary for WEBB voters.
	pub trait WebbApi
	{
		/// Return the current active WEBB validator set
		fn validator_set() -> ValidatorSet<crypto::AuthorityId>;
	}
}
