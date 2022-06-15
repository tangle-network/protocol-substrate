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
//! The signature bridge module provides functionality for the following:
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
//! * `execute_proposal`: Executes proposal if the proposal data is well-formed and signed by DKG
//!   (see the function below for more documentation)
//! * `set_maintainer`: Sets the maintainer.

// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
pub mod mock;
#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

mod weights;
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
	DispatchError, RuntimeDebug,
};
use sp_std::{convert::TryInto, prelude::*};
use webb_primitives::{signing::SigningSystem, utils::compute_chain_id_type, ResourceId};
pub use weights::WeightInfo;

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
	#[pallet::without_storage_info]
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
			+ Decode
			+ GetDispatchInfo;
		/// ChainID for anchor edges
		type ChainId: Encode
			+ Decode
			+ Parameter
			+ AtLeast32Bit
			+ Default
			+ Copy
			+ From<u64>
			+ From<u32>;
		/// Proposal nonce type
		type ProposalNonce: Encode + Decode + Parameter + AtLeast32Bit + Default + Copy;
		/// Maintainer nonce type
		type MaintainerNonce: Encode + Decode + Parameter + AtLeast32Bit + Default + Copy;

		/// Signature verification utility over public key infrastructure
		type SignatureVerifier: SigningSystem;
		/// The identifier for this chain.
		/// This must be unique and must not collide with existing IDs within a
		/// set of bridged chains.
		#[pallet::constant]
		type ChainIdentifier: Get<Self::ChainId>;
		/// The chain type for this chain.
		/// This is either a standalone Substrate chain, relay chain, or parachain
		#[pallet::constant]
		type ChainType: Get<[u8; 2]>;

		#[pallet::constant]
		type ProposalLifetime: Get<Self::BlockNumber>;

		#[pallet::constant]
		type BridgeAccountId: Get<PalletId>;

		type WeightInfo: WeightInfo;
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
	pub type Resources<T: Config<I>, I: 'static = ()> = StorageMap<_, Blake2_256, ResourceId, ()>;

	/// The proposal nonce used to prevent replay attacks on execute_proposal
	#[pallet::storage]
	pub type ProposalNonce<T: Config<I>, I: 'static = ()> =
		StorageValue<_, T::ProposalNonce, ValueQuery>;

	#[pallet::storage]
	pub type MaintainerNonce<T: Config<I>, I: 'static = ()> =
		StorageValue<_, T::MaintainerNonce, ValueQuery>;

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
		/// Resource ID provided is already mapped to anchor
		ResourceAlreadyExists,
		/// Provided signature is not from the active maintainer
		SignatureInvalid,
		/// Protected operation, must be performed by relayer
		MustBeMaintainer,
		/// A proposal with these parameters has already been submitted
		ProposalAlreadyExists,
		/// Call does not match parsed call from proposal data
		CallNotConsistentWithProposalData,
		/// Call does not match resource id according to resources mapping
		CallDoesNotMatchResourceId,
		/// Chain Id Type from the r_id does not match this chain
		IncorrectExecutionChainIdType,
		/// Invalid nonce
		InvalidNonce,
		/// Invalid proposal data
		InvalidProposalData,
	}

	#[pallet::hooks]
	impl<T: Config<I>, I: 'static> Hooks<BlockNumberFor<T>> for Pallet<T, I> {}

	#[pallet::call]
	impl<T: Config<I>, I: 'static> Pallet<T, I> {
		/// Sets the maintainer.
		#[pallet::weight(T::WeightInfo::set_maintainer())]
		pub fn set_maintainer(
			origin: OriginFor<T>,
			// message contains the nonce as the first 4 bytes and the laste bytes of the message
			// is the new_maintainer
			message: Vec<u8>,
			signature: Vec<u8>,
		) -> DispatchResultWithPostInfo {
			let _origin = ensure_signed(origin)?;
			let old_maintainer = <Maintainer<T, I>>::get();
			let maintainer_nonce = MaintainerNonce::<T, I>::get();
			let nonce = maintainer_nonce + 1u32.into();
			// nonce should be the first 4 bytes of this message
			let nonce_from_maintainer = T::MaintainerNonce::decode(&mut &message[..4])
				.map_err(|_| Error::<T, I>::InvalidNonce)?;

			// Nonce should increment by 1
			ensure!(nonce_from_maintainer == nonce, Error::<T, I>::InvalidNonce);

			// ensure parameter setter is the maintainer
			ensure!(
				T::SignatureVerifier::verify(&Self::maintainer(), &message, &signature)
					.unwrap_or(false),
				Error::<T, I>::InvalidPermissions
			);
			// set the new maintainer nonce
			MaintainerNonce::<T, I>::put(&nonce);
			// set the new maintainer
			Maintainer::<T, I>::try_mutate(|maintainer| {
				*maintainer = message[4..].to_vec();
				Self::deposit_event(Event::MaintainerSet {
					old_maintainer,
					new_maintainer: message,
				});
				Ok(().into())
			})
		}

		// Forcefully set the maintainer.
		#[pallet::weight(T::WeightInfo::force_set_maintainer())]
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
		#[pallet::weight(T::WeightInfo::set_resource())]
		pub fn set_resource(origin: OriginFor<T>, id: ResourceId) -> DispatchResultWithPostInfo {
			Self::ensure_admin(origin)?;
			Self::register_resource(id)
		}

		/// Removes a resource ID from the resource mapping.
		///
		/// After this call, bridge transfers with the associated resource ID
		/// will be rejected.
		///
		/// # <weight>
		/// - O(1) removal
		/// # </weight>
		#[pallet::weight(T::WeightInfo::remove_resource())]
		pub fn remove_resource(origin: OriginFor<T>, id: ResourceId) -> DispatchResultWithPostInfo {
			Self::ensure_admin(origin)?;
			Self::unregister_resource(id)
		}

		/// Enables a chain ID as a source or destination for a bridge transfer.
		///
		/// # <weight>
		/// - O(1) lookup and insert
		/// # </weight>
		#[pallet::weight(T::WeightInfo::whitelist_chain())]
		pub fn whitelist_chain(origin: OriginFor<T>, id: T::ChainId) -> DispatchResultWithPostInfo {
			Self::ensure_admin(origin)?;
			Self::whitelist(id)
		}

		/// @param origin
		/// @param src_id
		/// @param call: the dispatchable call corresponding to a
		/// handler function
		/// @param proposal_data: (r_id, nonce, 4 bytes of zeroes, call)
		/// @param signature: a signature over the proposal_data
		///
		/// We check:
		/// 1. That the signature is actually over the proposal data
		/// 2. Add ResourceId to the Storage
		/// 3. That the call from the proposal data and the call input parameter to the function are
		/// consistent with each other 4. That the execution chain id type parsed from the r_id is
		/// indeed this chain's id type
		///
		/// If all these checks pass then we call finalize_execution which actually executes the
		/// dispatchable call. The dispatchable call is usually a handler function, for instance in
		/// the anchor-handler or token-wrapper-handler pallet.
		///
		/// There are a few TODOs left in the function.
		///
		/// In the set_resource_with_signature
		/// # <weight>
		/// - weight of proposed call, regardless of whether execution is performed
		/// # </weight>
		#[pallet::weight((call.get_dispatch_info().weight + T::WeightInfo::set_resource_with_signature(), call.get_dispatch_info().class, Pays::Yes))]
		pub fn set_resource_with_signature(
			origin: OriginFor<T>,
			src_id: T::ChainId,
			call: Box<<T as Config<I>>::Proposal>,
			proposal_data: Vec<u8>,
			signature: Vec<u8>,
		) -> DispatchResultWithPostInfo {
			let _ = ensure_signed(origin)?;
			let r_id = Self::parse_r_id_from_proposal_data(&proposal_data)?;
			let nonce = Self::parse_nonce_from_proposal_data(&proposal_data)?;
			let parsed_call = Self::parse_call_from_proposal_data(&proposal_data);

			// Nonce should be greater than the proposal nonce in storage
			let proposal_nonce = ProposalNonce::<T, I>::get();
			ensure!(proposal_nonce < nonce, Error::<T, I>::InvalidNonce);

			// Nonce should increment by 1
			ensure!(
				nonce <= proposal_nonce + T::ProposalNonce::from(1u32),
				Error::<T, I>::InvalidNonce
			);

			// Verify proposal signature
			ensure!(
				T::SignatureVerifier::verify(&Self::maintainer(), &proposal_data[..], &signature)
					.unwrap_or(false),
				Error::<T, I>::InvalidPermissions,
			);
			// ChainId should be whitelisted
			ensure!(Self::chain_whitelisted(src_id), Error::<T, I>::ChainNotWhitelisted);

			// Ensure that call is consistent with parsed_call
			let encoded_call = call.encode();
			ensure!(encoded_call == parsed_call, Error::<T, I>::CallNotConsistentWithProposalData);

			// Ensure this chain id matches the r_id
			let execution_chain_id_type = Self::parse_chain_id_type_from_r_id(r_id);
			let this_chain_id_type =
				compute_chain_id_type(T::ChainIdentifier::get(), T::ChainType::get());

			ensure!(
				this_chain_id_type == execution_chain_id_type,
				Error::<T, I>::IncorrectExecutionChainIdType
			);
			// check if resource already exists
			ensure!(!Self::resource_exists(r_id), Error::<T, I>::ResourceAlreadyExists);
			//add resource
			_ = Self::register_resource(r_id);

			Self::finalize_execution(src_id, nonce, call)
		}

		/// @param origin
		/// @param src_id
		/// @param call: the dispatchable call corresponding to a
		/// handler function
		/// @param proposal_data: (r_id, nonce, 4 bytes of zeroes, call)
		/// @param signature: a signature over the proposal_data
		///
		/// We check:
		/// 1. That the signature is actually over the proposal data
		/// 2. That the r_id parsed from the proposal data exists
		/// 3. That the call from the proposal data and the call input parameter to the function are
		/// consistent with each other 4. That the execution chain id type parsed from the r_id is
		/// indeed this chain's id type
		///
		/// If all these checks pass then we call finalize_execution which actually executes the
		/// dispatchable call. The dispatchable call is usually a handler function, for instance in
		/// the anchor-handler or token-wrapper-handler pallet.
		///
		/// There are a few TODOs left in the function.
		///
		/// In the execute_proposal
		/// # <weight>
		/// - weight of proposed call, regardless of whether execution is performed
		/// # </weight>
		#[pallet::weight((T::WeightInfo::execute_proposal() + call.get_dispatch_info().weight, call.get_dispatch_info().class, Pays::Yes))]
		pub fn execute_proposal(
			origin: OriginFor<T>,
			src_id: T::ChainId,
			call: Box<<T as Config<I>>::Proposal>,
			proposal_data: Vec<u8>,
			signature: Vec<u8>,
		) -> DispatchResultWithPostInfo {
			let _ = ensure_signed(origin)?;
			let r_id = Self::parse_r_id_from_proposal_data(&proposal_data)?;
			let nonce = Self::parse_nonce_from_proposal_data(&proposal_data)?;
			let parsed_call = Self::parse_call_from_proposal_data(&proposal_data);

			// Nonce should be greater than the proposal nonce in storage
			let proposal_nonce = ProposalNonce::<T, I>::get();
			ensure!(proposal_nonce < nonce, Error::<T, I>::InvalidNonce);

			// Nonce should increment by 1
			ensure!(
				nonce <= proposal_nonce + T::ProposalNonce::from(1u32),
				Error::<T, I>::InvalidNonce
			);

			ensure!(
				T::SignatureVerifier::verify(&Self::maintainer(), &proposal_data[..], &signature)
					.unwrap_or(false),
				Error::<T, I>::InvalidPermissions,
			);
			ensure!(Self::chain_whitelisted(src_id), Error::<T, I>::ChainNotWhitelisted);
			ensure!(Self::resource_exists(r_id), Error::<T, I>::ResourceDoesNotExist);

			// Ensure that call is consistent with parsed_call
			let encoded_call = call.encode();
			ensure!(encoded_call == parsed_call, Error::<T, I>::CallNotConsistentWithProposalData);

			// Ensure this chain id matches the r_id
			let execution_chain_id_type = Self::parse_chain_id_type_from_r_id(r_id);
			let this_chain_id_type =
				compute_chain_id_type(T::ChainIdentifier::get(), T::ChainType::get());

			ensure!(
				this_chain_id_type == execution_chain_id_type,
				Error::<T, I>::IncorrectExecutionChainIdType
			);

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
		T::BridgeAccountId::get().into_account_truncating()
	}

	/// Asserts if a resource is registered
	pub fn resource_exists(id: ResourceId) -> bool {
		Self::resources(id) != None
	}

	/// Checks if a chain exists as a whitelisted destination
	pub fn chain_whitelisted(id: T::ChainId) -> bool {
		Self::chains(id) != None
	}

	pub fn parse_r_id_from_proposal_data(proposal_data: &[u8]) -> Result<[u8; 32], DispatchError> {
		ensure!(proposal_data.len() >= 40, Error::<T, I>::InvalidProposalData);
		Ok(proposal_data[0..32].try_into().unwrap_or_default())
	}

	pub fn parse_nonce_from_proposal_data(
		proposal_data: &[u8],
	) -> Result<T::ProposalNonce, DispatchError> {
		ensure!(proposal_data.len() >= 40, Error::<T, I>::InvalidProposalData);
		let nonce_bytes = proposal_data[36..40].try_into().unwrap_or_default();
		let nonce = u32::from_be_bytes(nonce_bytes);
		Ok(T::ProposalNonce::from(nonce))
	}

	pub fn parse_call_from_proposal_data(proposal_data: &[u8]) -> Vec<u8> {
		// Not [36..] because there are 4 byte of zero padding to match Solidity side
		proposal_data[40..].to_vec()
	}

	pub fn parse_method_from_call(parsed_call: Vec<u8>) -> Vec<u8> {
		parsed_call
	}

	pub fn parse_chain_id_type_from_r_id(r_id: ResourceId) -> u64 {
		let mut chain_id_type = [0u8; 8];
		chain_id_type[2] = r_id[26];
		chain_id_type[3] = r_id[27];
		chain_id_type[4] = r_id[28];
		chain_id_type[5] = r_id[29];
		chain_id_type[6] = r_id[30];
		chain_id_type[7] = r_id[31];

		u64::from_be_bytes(chain_id_type)
	}

	// *** Admin methods ***

	/// Register a method for a resource Id, enabling associated transfers
	pub fn register_resource(id: ResourceId) -> DispatchResultWithPostInfo {
		Resources::<T, I>::insert(id, ());
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
		ensure!(
			id != T::ChainId::from(compute_chain_id_type(
				T::ChainIdentifier::get(),
				T::ChainType::get()
			)),
			Error::<T, I>::InvalidChainId
		);
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
		// Increment the nonce once the proposal succeeds
		ProposalNonce::<T, I>::put(nonce);
		Ok(().into())
	}
}

/// Simple ensure origin for the bridge account
#[derive(Encode, Decode, Clone, Eq, PartialEq, TypeInfo, RuntimeDebug)]
pub struct EnsureBridge<T, I>(sp_std::marker::PhantomData<(T, I)>);
impl<T: Config<I>, I: 'static> EnsureOrigin<T::Origin> for EnsureBridge<T, I> {
	type Success = T::AccountId;

	fn try_origin(o: T::Origin) -> Result<Self::Success, T::Origin> {
		let bridge_id = T::BridgeAccountId::get().into_account_truncating();
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
		T::Origin::from(frame_system::RawOrigin::Signed(
			T::BridgeAccountId::get().into_account_truncating(),
		))
	}
}
