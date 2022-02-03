// This file is part of Webb.

// Copyright (C) 2021 Webb Technologies Inc.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! # Signature Bridge Module
//!
//! A module for managing voting, resource, and maintainer composition through signature
//! verification.
//!
//! ## Overview
//!
//! The signature bridge module provides functionalight for the following:
//!
//! * Private bridging of assets governed by signature verification
//!
//! ## Interface
//!
//! ### Permissioned Functions
//!
//! * `force_set_maintainer`: Forcefully set the maintainer. This method requires the `origin` to be
//!   [T::AdminOrigin].
//! * `set_resource`: Stores a method name on chain under an associated resource ID. This method
//!   requires the `origin` to be [T::AdminOrigin].
//! * `remove_resource`: Removes a resource ID from the resource mapping. This method requires the
//!   `origin` to be [T::AdminOrigin].
//! * `whitelist_chain`: Enables a chain ID as a source or destination for a bridge transfer. This
//!   method requires the `origin` to be [T::AdminOrigin].
//!
//! ### Permissionless Functions
//!
//! * `execute_proposal`: Commits a vote in favour of the provided proposal.
//! * `set_maintainer`: Sets the maintainer.

// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
pub mod mock;
#[cfg(test)]
mod tests;

pub mod utils;

use codec::{Decode, Encode, EncodeLike};
use frame_support::{
	pallet_prelude::{ensure, DispatchResultWithPostInfo},
	traits::{EnsureOrigin, Get},
};
use frame_system::{self as system, ensure_root};
pub use pallet::*;
use scale_info::TypeInfo;
use sp_runtime::{
	traits::{AccountIdConversion, Dispatchable},
	RuntimeDebug,
};
use sp_std::prelude::*;
use webb_primitives::{signing::SigningSystem, ResourceId};

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::{
		dispatch::{DispatchResultWithPostInfo, Dispatchable, GetDispatchInfo},
		pallet_prelude::*,
		PalletId,
	};
	use frame_system::pallet_prelude::*;
	use sp_runtime::traits::AtLeast32Bit;

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T, I = ()>(_);

	#[pallet::config]
	/// The module configuration trait.
	pub trait Config<I: 'static = ()>: frame_system::Config {
		/// The overarching event type.
		type Event: From<Event<Self, I>> + IsType<<Self as frame_system::Config>::Event>;
		/// Origin used to administer the pallet
		type AdminOrigin: EnsureOrigin<Self::Origin>;
		/// Proposed dispatchable call
		type Proposal: Parameter
			+ Dispatchable<Origin = Self::Origin>
			+ EncodeLike
			+ GetDispatchInfo;
		/// ChainID for anchor edges
		type ChainId: Encode + Decode + Parameter + AtLeast32Bit + Default + Copy;
		/// Proposal nonce type
		type ProposalNonce: Encode + Decode + Parameter + AtLeast32Bit + Default + Copy;
		/// Signature verification utility over public key infrastructure
		type SignatureVerifier: SigningSystem;
		/// The identifier for this chain.
		/// This must be unique and must not collide with existing IDs within a
		/// set of bridged chains.
		#[pallet::constant]
		type ChainIdentifier: Get<Self::ChainId>;

		#[pallet::constant]
		type ProposalLifetime: Get<Self::BlockNumber>;

		#[pallet::constant]
		type BridgeAccountId: Get<PalletId>;
	}

	/// The parameter maintainer who can change the parameters
	#[pallet::storage]
	#[pallet::getter(fn maintainer)]
	pub type Maintainer<T: Config<I>, I: 'static = ()> = StorageValue<_, Vec<u8>, ValueQuery>;

	/// All whitelisted chains and their respective transaction counts
	#[pallet::storage]
	#[pallet::getter(fn chains)]
	pub type ChainNonces<T: Config<I>, I: 'static = ()> =
		StorageMap<_, Blake2_256, T::ChainId, T::ProposalNonce>;

	/// Utilized by the bridge software to map resource IDs to actual methods
	#[pallet::storage]
	#[pallet::getter(fn resources)]
	pub type Resources<T: Config<I>, I: 'static = ()> =
		StorageMap<_, Blake2_256, ResourceId, Vec<u8>>;

	// Pallets use events to inform users when important changes are made.
	#[pallet::event]
	#[pallet::generate_deposit(pub fn deposit_event)]
	pub enum Event<T: Config<I>, I: 'static = ()> {
		/// Maintainer is set
		MaintainerSet { old_maintainer: Vec<u8>, new_maintainer: Vec<u8> },
		/// Chain now available for transfers (chain_id)
		ChainWhitelisted { chain_id: T::ChainId },
		/// Proposal has been approved
		ProposalApproved { chain_id: T::ChainId, proposal_nonce: T::ProposalNonce },
		/// Execution of call succeeded
		ProposalSucceeded { chain_id: T::ChainId, proposal_nonce: T::ProposalNonce },
		/// Execution of call failed
		ProposalFailed { chain_id: T::ChainId, proposal_nonce: T::ProposalNonce },
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T, I = ()> {
		/// Account does not have correct permissions
		InvalidPermissions,
		/// Provided chain Id is not valid
		InvalidChainId,
		/// Interactions with this chain is not permitted
		ChainNotWhitelisted,
		/// Chain has already been enabled
		ChainAlreadyWhitelisted,
		/// Resource ID provided isn't mapped to anything
		ResourceDoesNotExist,
		/// Provided signature is not from the active maintainer
		SignatureInvalid,
		/// Protected operation, must be performed by relayer
		MustBeMaintainer,
		/// A proposal with these parameters has already been submitted
		ProposalAlreadyExists,
	}

	#[pallet::hooks]
	impl<T: Config<I>, I: 'static> Hooks<BlockNumberFor<T>> for Pallet<T, I> {}

	#[pallet::call]
	impl<T: Config<I>, I: 'static> Pallet<T, I> {
		/// Sets the maintainer.
		#[pallet::weight(0)]
		pub fn set_maintainer(
			origin: OriginFor<T>,
			new_maintainer: Vec<u8>,
			signature: Vec<u8>,
		) -> DispatchResultWithPostInfo {
			let origin = ensure_signed(origin)?;
			let old_maintainer = <Maintainer<T, I>>::get();
			// ensure parameter setter is the maintainer
			ensure!(
				T::SignatureVerifier::verify(
					&Self::maintainer().encode()[..],
					&new_maintainer.encode()[..],
					&signature
				)
				.unwrap_or(false),
				Error::<T, I>::InvalidPermissions
			);
			// set the new maintainer
			Maintainer::<T, I>::try_mutate(|maintainer| {
				*maintainer = new_maintainer.clone();
				Self::deposit_event(Event::MaintainerSet { old_maintainer, new_maintainer });
				Ok(().into())
			})
		}

		// Forcefully set the maintainer.
		#[pallet::weight(0)]
		pub fn force_set_maintainer(
			origin: OriginFor<T>,
			new_maintainer: Vec<u8>,
		) -> DispatchResultWithPostInfo {
			Self::ensure_admin(origin)?;
			// set the new maintainer
			Maintainer::<T, I>::try_mutate(|maintainer| {
				let old_maintainer = maintainer.clone();
				*maintainer = new_maintainer.clone();
				Self::deposit_event(Event::MaintainerSet { old_maintainer, new_maintainer });
				Ok(().into())
			})
		}

		/// Stores a method name on chain under an associated resource ID.
		///
		/// # <weight>
		/// - O(1) write
		/// # </weight>
		#[pallet::weight(195_000_000)]
		pub fn set_resource(
			origin: OriginFor<T>,
			id: ResourceId,
			method: Vec<u8>,
		) -> DispatchResultWithPostInfo {
			Self::ensure_admin(origin)?;
			Self::register_resource(id, method)
		}

		/// Removes a resource ID from the resource mapping.
		///
		/// After this call, bridge transfers with the associated resource ID
		/// will be rejected.
		///
		/// # <weight>
		/// - O(1) removal
		/// # </weight>
		#[pallet::weight(195_000_000)]
		pub fn remove_resource(origin: OriginFor<T>, id: ResourceId) -> DispatchResultWithPostInfo {
			Self::ensure_admin(origin)?;
			Self::unregister_resource(id)
		}

		/// Enables a chain ID as a source or destination for a bridge transfer.
		///
		/// # <weight>
		/// - O(1) lookup and insert
		/// # </weight>
		#[pallet::weight(195_000_000)]
		pub fn whitelist_chain(origin: OriginFor<T>, id: T::ChainId) -> DispatchResultWithPostInfo {
			Self::ensure_admin(origin)?;
			Self::whitelist(id)
		}

		/// Commits a vote in favour of the provided proposal.
		///
		/// If a proposal with the given nonce and source chain ID does not
		/// already exist, it will be created with an initial vote in favour
		/// from the caller.
		///
		/// # <weight>
		/// - weight of proposed call, regardless of whether execution is performed
		/// # </weight>
		#[pallet::weight((call.get_dispatch_info().weight + 195_000_000, call.get_dispatch_info().class, Pays::Yes))]
		pub fn execute_proposal(
			origin: OriginFor<T>,
			nonce: T::ProposalNonce,
			src_id: T::ChainId,
			r_id: ResourceId,
			call: Box<<T as Config<I>>::Proposal>,
			signature: Vec<u8>,
		) -> DispatchResultWithPostInfo {
			let _ = ensure_signed(origin)?;
			ensure!(
				T::SignatureVerifier::verify(&Self::maintainer(), &call.encode()[..], &signature)
					.unwrap_or(false),
				Error::<T, I>::InvalidPermissions,
			);
			ensure!(Self::chain_whitelisted(src_id), Error::<T, I>::ChainNotWhitelisted);
			ensure!(Self::resource_exists(r_id), Error::<T, I>::ResourceDoesNotExist);

			Self::finalize_execution(src_id, nonce, call)
		}
	}
}

impl<T: Config<I>, I: 'static> Pallet<T, I> {
	// *** Utility methods ***

	pub fn ensure_admin(o: T::Origin) -> DispatchResultWithPostInfo {
		T::AdminOrigin::try_origin(o).map(|_| ()).or_else(ensure_root)?;
		Ok(().into())
	}

	/// Provides an AccountId for the pallet.
	/// This is used both as an origin check and deposit/withdrawal account.
	pub fn account_id() -> T::AccountId {
		T::BridgeAccountId::get().into_account()
	}

	/// Asserts if a resource is registered
	pub fn resource_exists(id: ResourceId) -> bool {
		Self::resources(id) != None
	}

	/// Checks if a chain exists as a whitelisted destination
	pub fn chain_whitelisted(id: T::ChainId) -> bool {
		Self::chains(id) != None
	}

	// *** Admin methods ***

	/// Register a method for a resource Id, enabling associated transfers
	pub fn register_resource(id: ResourceId, method: Vec<u8>) -> DispatchResultWithPostInfo {
		Resources::<T, I>::insert(id, method);
		Ok(().into())
	}

	/// Removes a resource ID, disabling associated transfer
	pub fn unregister_resource(id: ResourceId) -> DispatchResultWithPostInfo {
		Resources::<T, I>::remove(id);
		Ok(().into())
	}

	/// Whitelist a chain ID for transfer
	pub fn whitelist(id: T::ChainId) -> DispatchResultWithPostInfo {
		// Cannot whitelist this chain
		ensure!(id != T::ChainIdentifier::get(), Error::<T, I>::InvalidChainId);
		// Cannot whitelist with an existing entry
		ensure!(!Self::chain_whitelisted(id), Error::<T, I>::ChainAlreadyWhitelisted);
		ChainNonces::<T, I>::insert(&id, T::ProposalNonce::from(0u32));
		Self::deposit_event(Event::ChainWhitelisted { chain_id: id });
		Ok(().into())
	}

	// *** Proposal voting and execution methods ***

	#[allow(clippy::boxed_local)]
	/// Execute the proposal and signals the result as an event
	fn finalize_execution(
		src_id: T::ChainId,
		nonce: T::ProposalNonce,
		call: Box<T::Proposal>,
	) -> DispatchResultWithPostInfo {
		Self::deposit_event(Event::ProposalApproved { chain_id: src_id, proposal_nonce: nonce });
		call.dispatch(frame_system::RawOrigin::Signed(Self::account_id()).into())
			.map(|_| ())
			.map_err(|e| e.error)?;
		Self::deposit_event(Event::ProposalSucceeded { chain_id: src_id, proposal_nonce: nonce });
		Ok(().into())
	}
}

/// Simple ensure origin for the bridge account
#[derive(Encode, Decode, Clone, Eq, PartialEq, TypeInfo, RuntimeDebug)]
pub struct EnsureBridge<T, I>(sp_std::marker::PhantomData<(T, I)>);
impl<T: Config<I>, I: 'static> EnsureOrigin<T::Origin> for EnsureBridge<T, I> {
	type Success = T::AccountId;

	fn try_origin(o: T::Origin) -> Result<Self::Success, T::Origin> {
		let bridge_id = T::BridgeAccountId::get().into_account();
		o.into().and_then(|o| match o {
			system::RawOrigin::Signed(who) if who == bridge_id => Ok(bridge_id),
			r => Err(T::Origin::from(r)),
		})
	}

	/// Returns an outer origin capable of passing `try_origin` check.
	///
	/// ** Should be used for benchmarking only!!! **
	#[cfg(feature = "runtime-benchmarks")]
	fn successful_origin() -> T::Origin {
		T::Origin::from(frame_system::RawOrigin::Signed(T::BridgeAccountId::get().into_account()))
	}
}
