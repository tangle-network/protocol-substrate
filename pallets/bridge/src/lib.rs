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

//! # Bridge Module
//!
//! Add description #TODO
//!
//! ## Overview
//!
//!
//! ### Terminology
//!
//! ### Goals
//!
//! The bridge system in Webb is designed to make the following
//! possible:
//!
//! * Define.
//!
//! ## Interface
//!
//! ## Related Modules
//!
//! * [`System`](../frame_system/index.html)
//! * [`Support`](../frame_support/index.html)

// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
pub mod mock;
#[cfg(test)]
mod tests;
pub mod types;

use crate::types::{DepositNonce, ProposalStatus, ProposalVotes};
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
use webb_primitives::{ResourceId, utils::{compute_chain_id_type}};

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use crate::types::{DepositNonce, ProposalVotes, WEBB_DEFAULT_RELAYER_THRESHOLD};
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
	}

	/// All whitelisted chains and their respective transaction counts
	#[pallet::storage]
	#[pallet::getter(fn chains)]
	pub type ChainNonces<T: Config<I>, I: 'static = ()> =
		StorageMap<_, Blake2_256, T::ChainId, DepositNonce>;

	#[pallet::type_value]
	pub fn DefaultForRelayerThreshold() -> u32 {
		WEBB_DEFAULT_RELAYER_THRESHOLD
	}

	/// Number of votes required for a proposal to execute
	#[pallet::storage]
	#[pallet::getter(fn relayer_threshold)]
	pub type RelayerThreshold<T: Config<I>, I: 'static = ()> =
		StorageValue<_, u32, ValueQuery, DefaultForRelayerThreshold>;

	/// Tracks current relayer set
	#[pallet::storage]
	#[pallet::getter(fn relayers)]
	pub type Relayers<T: Config<I>, I: 'static = ()> =
		StorageMap<_, Blake2_256, T::AccountId, bool, OptionQuery>;

	/// Number of relayers in set
	#[pallet::storage]
	#[pallet::getter(fn relayer_count)]
	pub type RelayerCount<T: Config<I>, I: 'static = ()> = StorageValue<_, u32, ValueQuery>;

	/// All known proposals.
	/// The key is the hash of the call and the deposit ID, to ensure it's
	/// unique.
	#[pallet::storage]
	#[pallet::getter(fn votes)]
	pub type Votes<T: Config<I>, I: 'static = ()> = StorageDoubleMap<
		_,
		Blake2_256,
		T::ChainId,
		Blake2_256,
		(DepositNonce, T::Proposal),
		ProposalVotes<T::AccountId, T::BlockNumber>,
	>;

	/// Utilized by the bridge software to map resource IDs to actual methods
	#[pallet::storage]
	#[pallet::getter(fn resources)]
	pub type Resources<T: Config<I>, I: 'static = ()> =
		StorageMap<_, Blake2_256, ResourceId, Vec<u8>>;

	// Pallets use events to inform users when important changes are made.
	#[pallet::event]
	#[pallet::generate_deposit(pub fn deposit_event)]
	pub enum Event<T: Config<I>, I: 'static = ()> {
		/// Vote threshold has changed (new_threshold)
		RelayerThresholdChanged { new_threshold: u32 },
		/// Chain now available for transfers (chain_id)
		ChainWhitelisted { chain_id: T::ChainId },
		/// Relayer added to set
		RelayerAdded { relayer_id: T::AccountId },
		/// Relayer removed from set
		RelayerRemoved { relayer_id: T::AccountId },
		/// Vote submitted in favour of proposal
		VoteFor { chain_id: T::ChainId, deposit_nonce: DepositNonce, who: T::AccountId },
		/// Vot submitted against proposal
		VoteAgainst { chain_id: T::ChainId, deposit_nonce: DepositNonce, who: T::AccountId },
		/// Voting successful for a proposal
		ProposalApproved { chain_id: T::ChainId, deposit_nonce: DepositNonce },
		/// Voting rejected a proposal
		ProposalRejected { chain_id: T::ChainId, deposit_nonce: DepositNonce },
		/// Execution of call succeeded
		ProposalSucceeded { chain_id: T::ChainId, deposit_nonce: DepositNonce },
		/// Execution of call failed
		ProposalFailed { chain_id: T::ChainId, deposit_nonce: DepositNonce },
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T, I = ()> {
		/// Account does not have correct permissions
		InvalidPermissions,
		/// Relayer threshold not set
		ThresholdNotSet,
		/// Provided chain Id is not valid
		InvalidChainId,
		/// Relayer threshold cannot be 0
		InvalidThreshold,
		/// Interactions with this chain is not permitted
		ChainNotWhitelisted,
		/// Chain has already been enabled
		ChainAlreadyWhitelisted,
		/// Resource ID provided isn't mapped to anything
		ResourceDoesNotExist,
		/// Relayer already in set
		RelayerAlreadyExists,
		/// Provided accountId is not a relayer
		RelayerInvalid,
		/// Protected operation, must be performed by relayer
		MustBeRelayer,
		/// Relayer has already submitted some vote for this proposal
		RelayerAlreadyVoted,
		/// A proposal with these parameters has already been submitted
		ProposalAlreadyExists,
		/// No proposal with the ID was found
		ProposalDoesNotExist,
		/// Cannot complete proposal, needs more votes
		ProposalNotComplete,
		/// Proposal has either failed or succeeded
		ProposalAlreadyComplete,
		/// Lifetime of proposal has been exceeded
		ProposalExpired,
	}

	#[pallet::hooks]
	impl<T: Config<I>, I: 'static> Hooks<BlockNumberFor<T>> for Pallet<T, I> {}

	#[pallet::call]
	impl<T: Config<I>, I: 'static> Pallet<T, I> {
		/// Sets the vote threshold for proposals.
		///
		/// This threshold is used to determine how many votes are required
		/// before a proposal is executed.
		///
		/// # <weight>
		/// - O(1) lookup and insert
		/// # </weight>
		#[pallet::weight(195_000_000)]
		pub fn set_threshold(origin: OriginFor<T>, threshold: u32) -> DispatchResultWithPostInfo {
			Self::ensure_admin(origin)?;
			Self::set_relayer_threshold(threshold)
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

		/// Adds a new relayer to the relayer set.
		///
		/// # <weight>
		/// - O(1) lookup and insert
		/// # </weight>
		#[pallet::weight(195_000_000)]
		pub fn add_relayer(origin: OriginFor<T>, v: T::AccountId) -> DispatchResultWithPostInfo {
			Self::ensure_admin(origin)?;
			Self::register_relayer(v)
		}

		/// Removes an existing relayer from the set.
		///
		/// # <weight>
		/// - O(1) lookup and removal
		/// # </weight>
		#[pallet::weight(195_000_000)]
		pub fn remove_relayer(origin: OriginFor<T>, v: T::AccountId) -> DispatchResultWithPostInfo {
			Self::ensure_admin(origin)?;
			Self::unregister_relayer(v)
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
		pub fn acknowledge_proposal(
			origin: OriginFor<T>,
			nonce: DepositNonce,
			src_id: T::ChainId,
			r_id: ResourceId,
			call: Box<<T as Config<I>>::Proposal>,
		) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;
			ensure!(Self::is_relayer(&who), Error::<T, I>::MustBeRelayer);
			ensure!(Self::chain_whitelisted(src_id), Error::<T, I>::ChainNotWhitelisted);
			ensure!(Self::resource_exists(r_id), Error::<T, I>::ResourceDoesNotExist);

			Self::vote_for(who, nonce, src_id, call)
		}

		/// Commits a vote against a provided proposal.
		///
		/// # <weight>
		/// - Fixed, since execution of proposal should not be included
		/// # </weight>
		#[pallet::weight(195_000_000)]
		pub fn reject_proposal(
			origin: OriginFor<T>,
			nonce: DepositNonce,
			src_id: T::ChainId,
			r_id: ResourceId,
			call: Box<<T as Config<I>>::Proposal>,
		) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;
			ensure!(Self::is_relayer(&who), Error::<T, I>::MustBeRelayer);
			ensure!(Self::chain_whitelisted(src_id), Error::<T, I>::ChainNotWhitelisted);
			ensure!(Self::resource_exists(r_id), Error::<T, I>::ResourceDoesNotExist);

			Self::vote_against(who, nonce, src_id, call)
		}

		/// Evaluate the state of a proposal given the current vote threshold.
		///
		/// A proposal with enough votes will be either executed or cancelled,
		/// and the status will be updated accordingly.
		///
		/// # <weight>
		/// - weight of proposed call, regardless of whether execution is performed
		/// # </weight>
		#[pallet::weight((prop.get_dispatch_info().weight + 195_000_000, prop.get_dispatch_info().class, Pays::Yes))]
		pub fn eval_vote_state(
			origin: OriginFor<T>,
			nonce: DepositNonce,
			src_id: T::ChainId,
			prop: Box<<T as Config<I>>::Proposal>,
		) -> DispatchResultWithPostInfo {
			ensure_signed(origin)?;

			Self::try_resolve_proposal(nonce, src_id, prop)
		}
	}
}

impl<T: Config<I>, I: 'static> Pallet<T, I> {
	// *** Utility methods ***

	pub fn ensure_admin(o: T::Origin) -> DispatchResultWithPostInfo {
		T::AdminOrigin::try_origin(o).map(|_| ()).or_else(ensure_root)?;
		Ok(().into())
	}

	/// Checks if who is a relayer
	pub fn is_relayer(who: &T::AccountId) -> bool {
		Self::relayers(who).is_some()
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

	/// Set a new voting threshold
	pub fn set_relayer_threshold(threshold: u32) -> DispatchResultWithPostInfo {
		ensure!(threshold > 0, Error::<T, I>::InvalidThreshold);
		RelayerThreshold::<T, I>::put(threshold);
		Self::deposit_event(Event::RelayerThresholdChanged { new_threshold: threshold });
		Ok(().into())
	}

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
		ensure!(
			id != T::ChainId::try_from(compute_chain_id_type(
				T::ChainIdentifier::get(),
				T::ChainType::get())
			).unwrap_or_default(),
			Error::<T, I>::InvalidChainId
		);
		// Cannot whitelist with an existing entry
		ensure!(!Self::chain_whitelisted(id), Error::<T, I>::ChainAlreadyWhitelisted);
		ChainNonces::<T, I>::insert(&id, 0);
		Self::deposit_event(Event::ChainWhitelisted { chain_id: id });
		Ok(().into())
	}

	/// Adds a new relayer to the set
	pub fn register_relayer(relayer: T::AccountId) -> DispatchResultWithPostInfo {
		ensure!(!Self::is_relayer(&relayer), Error::<T, I>::RelayerAlreadyExists);
		Relayers::<T, I>::insert(&relayer, true);
		RelayerCount::<T, I>::mutate(|i| *i += 1);

		Self::deposit_event(Event::RelayerAdded { relayer_id: relayer });
		Ok(().into())
	}

	/// Removes a relayer from the set
	pub fn unregister_relayer(relayer: T::AccountId) -> DispatchResultWithPostInfo {
		ensure!(Self::is_relayer(&relayer), Error::<T, I>::RelayerInvalid);
		Relayers::<T, I>::remove(&relayer);
		RelayerCount::<T, I>::mutate(|i| *i -= 1);
		Self::deposit_event(Event::RelayerRemoved { relayer_id: relayer });
		Ok(().into())
	}

	// *** Proposal voting and execution methods ***

	/// Commits a vote for a proposal. If the proposal doesn't exist it will be
	/// created.
	fn commit_vote(
		who: T::AccountId,
		nonce: DepositNonce,
		src_id: T::ChainId,
		prop: Box<T::Proposal>,
		in_favour: bool,
	) -> DispatchResultWithPostInfo {
		let now = <frame_system::Pallet<T>>::block_number();
		let mut votes = match Votes::<T, I>::get(src_id, (nonce, prop.clone())) {
			Some(v) => v,
			None =>
				ProposalVotes { expiry: now + T::ProposalLifetime::get(), ..Default::default() },
		};

		// Ensure the proposal isn't complete and relayer hasn't already voted
		ensure!(!votes.is_complete(), Error::<T, I>::ProposalAlreadyComplete);
		ensure!(!votes.is_expired(now), Error::<T, I>::ProposalExpired);
		ensure!(!votes.has_voted(&who), Error::<T, I>::RelayerAlreadyVoted);

		if in_favour {
			votes.votes_for.push(who.clone());
			Self::deposit_event(Event::VoteFor { chain_id: src_id, deposit_nonce: nonce, who });
		} else {
			votes.votes_against.push(who.clone());
			Self::deposit_event(Event::VoteAgainst { chain_id: src_id, deposit_nonce: nonce, who });
		}

		Votes::<T, I>::insert(src_id, (nonce, prop), votes.clone());

		Ok(().into())
	}

	/// Attempts to finalize or cancel the proposal if the vote count allows.
	fn try_resolve_proposal(
		nonce: DepositNonce,
		src_id: T::ChainId,
		prop: Box<T::Proposal>,
	) -> DispatchResultWithPostInfo {
		if let Some(mut votes) = Votes::<T, I>::get(src_id, (nonce, prop.clone())) {
			let now = <frame_system::Pallet<T>>::block_number();
			ensure!(!votes.is_complete(), Error::<T, I>::ProposalAlreadyComplete);
			ensure!(!votes.is_expired(now), Error::<T, I>::ProposalExpired);

			let status =
				votes.try_to_complete(RelayerThreshold::<T, I>::get(), RelayerCount::<T, I>::get());
			Votes::<T, I>::insert(src_id, (nonce, prop.clone()), votes.clone());

			match status {
				ProposalStatus::Approved => Self::finalize_execution(src_id, nonce, prop),
				ProposalStatus::Rejected => Self::cancel_execution(src_id, nonce),
				_ => Ok(().into()),
			}
		} else {
			Err(Error::<T, I>::ProposalDoesNotExist.into())
		}
	}

	/// Commits a vote in favour of the proposal and executes it if the vote
	/// threshold is met.
	fn vote_for(
		who: T::AccountId,
		nonce: DepositNonce,
		src_id: T::ChainId,
		prop: Box<T::Proposal>,
	) -> DispatchResultWithPostInfo {
		Self::commit_vote(who, nonce, src_id, prop.clone(), true)?;
		Self::try_resolve_proposal(nonce, src_id, prop)
	}

	/// Commits a vote against the proposal and cancels it if more than
	/// (relayers.len() - threshold) votes against exist.
	fn vote_against(
		who: T::AccountId,
		nonce: DepositNonce,
		src_id: T::ChainId,
		prop: Box<T::Proposal>,
	) -> DispatchResultWithPostInfo {
		Self::commit_vote(who, nonce, src_id, prop.clone(), false)?;
		Self::try_resolve_proposal(nonce, src_id, prop)
	}

	#[allow(clippy::boxed_local)]
	/// Execute the proposal and signals the result as an event
	fn finalize_execution(
		src_id: T::ChainId,
		nonce: DepositNonce,
		call: Box<T::Proposal>,
	) -> DispatchResultWithPostInfo {
		Self::deposit_event(Event::ProposalApproved { chain_id: src_id, deposit_nonce: nonce });
		call.dispatch(frame_system::RawOrigin::Signed(Self::account_id()).into())
			.map(|_| ())
			.map_err(|e| e.error)?;
		Self::deposit_event(Event::ProposalSucceeded { chain_id: src_id, deposit_nonce: nonce });
		Ok(().into())
	}

	/// Cancels a proposal.
	fn cancel_execution(src_id: T::ChainId, nonce: DepositNonce) -> DispatchResultWithPostInfo {
		Self::deposit_event(Event::ProposalRejected { chain_id: src_id, deposit_nonce: nonce });
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
