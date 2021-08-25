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

use codec::{Decode, Encode, EncodeLike};
use frame_support::{
	pallet_prelude::{ensure, DispatchResultWithPostInfo},
	traits::{EnsureOrigin, Get},
	weights::{GetDispatchInfo, Pays},
	PalletId, Parameter,
};
use frame_system::{self as system, ensure_root, ensure_signed};
use sp_runtime::traits::{AccountIdConversion, Dispatchable};
use sp_std::prelude::*;

pub mod types;
use crate::types::{ChainId, DepositNonce, ProposalStatus, ProposalVotes, ResourceId};

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use crate::types::{ChainId, DepositNonce, ProposalVotes, ResourceId};
	use codec::EncodeLike;
	use frame_support::{
		dispatch::{DispatchResultWithPostInfo, Dispatchable, GetDispatchInfo},
		pallet_prelude::*,
		PalletId,
	};
	use frame_system::pallet_prelude::*;

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::config]
	/// The module configuration trait.
	pub trait Config: frame_system::Config {
		/// The overarching event type.
		type Event: IsType<<Self as frame_system::Config>::Event> + From<Event<Self>>;
		/// Origin used to administer the pallet
		type AdminOrigin: EnsureOrigin<Self::Origin>;
		/// Proposed dispatchable call
		type Proposal: Parameter + Dispatchable<Origin = Self::Origin> + EncodeLike + GetDispatchInfo;
		/// The identifier for this chain.
		/// This must be unique and must not collide with existing IDs within a
		/// set of bridged chains.
		type ChainId: Get<u8>;

		type ProposalLifetime: Get<Self::BlockNumber>;

		#[pallet::constant]
		type PalletId: Get<PalletId>;
	}

	/// All whitelisted chains and their respective transaction counts
	#[pallet::storage]
	#[pallet::getter(fn chains)]
	pub type ChainNonces<T: Config> = StorageMap<_, Blake2_128Concat, ChainId, Option<DepositNonce>, ValueQuery>;

	/// Number of votes required for a proposal to execute
	#[pallet::storage]
	#[pallet::getter(fn relayer_threshold)]
	pub type RelayerThreshold<T: Config> = StorageValue<_, u32, ValueQuery>; //TODO default value

	/// Number of votes required for a proposal to execute
	// RelayerThreshold get(fn relayer_threshold): u32 = DEFAULT_RELAYER_THRESHOLD;

	/// Tracks current relayer set
	#[pallet::storage]
	#[pallet::getter(fn relayers)]
	pub type Relayers<T: Config> = StorageMap<_, Blake2_128Concat, T::AccountId, bool, ValueQuery>;

	/// Number of relayers in set
	#[pallet::storage]
	#[pallet::getter(fn relayer_count)]
	pub type RelayerCount<T: Config> = StorageValue<_, u32, ValueQuery>;

	/// All known proposals.
	/// The key is the hash of the call and the deposit ID, to ensure it's
	/// unique.
	#[pallet::storage]
	#[pallet::getter(fn votes)]
	pub(super) type Votes<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		ChainId,
		Blake2_128Concat,
		(DepositNonce, T::Proposal),
		Option<ProposalVotes<T::AccountId, T::BlockNumber>>,
		ValueQuery,
	>;

	/// Utilized by the bridge software to map resource IDs to actual methods
	#[pallet::storage]
	#[pallet::getter(fn resources)]
	pub type Resources<T: Config> = StorageMap<_, Blake2_128Concat, ResourceId, Option<Vec<u8>>, ValueQuery>;

	// Pallets use events to inform users when important changes are made.
	#[pallet::event]
	#[pallet::metadata(T::AccountId = "AccountId")]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Vote threshold has changed (new_threshold)
		RelayerThresholdChanged(u32),
		/// Chain now available for transfers (chain_id)
		ChainWhitelisted(ChainId),
		/// Relayer added to set
		RelayerAdded(T::AccountId),
		/// Relayer removed from set
		RelayerRemoved(T::AccountId),
		/// Vote submitted in favour of proposal
		VoteFor(ChainId, DepositNonce, T::AccountId),
		/// Vot submitted against proposal
		VoteAgainst(ChainId, DepositNonce, T::AccountId),
		/// Voting successful for a proposal
		ProposalApproved(ChainId, DepositNonce),
		/// Voting rejected a proposal
		ProposalRejected(ChainId, DepositNonce),
		/// Execution of call succeeded
		ProposalSucceeded(ChainId, DepositNonce),
		/// Execution of call failed
		ProposalFailed(ChainId, DepositNonce),
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
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
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		// FIXME
		//const BridgeAccountId: T::AccountId = MODULE_ID.into_account();
		//const ChainIdentity: ChainId = T::ChainId::get();
		//const ProposalLifetime: T::BlockNumber = T::ProposalLifetime::get();

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
		pub fn set_resource(origin: OriginFor<T>, id: ResourceId, method: Vec<u8>) -> DispatchResultWithPostInfo {
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
		pub fn whitelist_chain(origin: OriginFor<T>, id: ChainId) -> DispatchResultWithPostInfo {
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
		/// - weight of proposed call, regardless of whether execution is
		///   performed
		/// # </weight>
		// FIXME #[pallet::weight(call.get_dispatch_info().weight + 195_000_000, call.get_dispatch_info().class,
		// Pays::Yes)]
		#[pallet::weight(call.get_dispatch_info().weight + 195_000_000)]
		pub fn acknowledge_proposal(
			origin: OriginFor<T>,
			nonce: DepositNonce,
			src_id: ChainId,
			r_id: ResourceId,
			call: Box<<T as Config>::Proposal>,
		) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;
			ensure!(Self::is_relayer(&who), Error::<T>::MustBeRelayer);
			ensure!(Self::chain_whitelisted(src_id), Error::<T>::ChainNotWhitelisted);
			ensure!(Self::resource_exists(r_id), Error::<T>::ResourceDoesNotExist);

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
			src_id: ChainId,
			r_id: ResourceId,
			call: Box<<T as Config>::Proposal>,
		) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;
			ensure!(Self::is_relayer(&who), Error::<T>::MustBeRelayer);
			ensure!(Self::chain_whitelisted(src_id), Error::<T>::ChainNotWhitelisted);
			ensure!(Self::resource_exists(r_id), Error::<T>::ResourceDoesNotExist);

			Self::vote_against(who, nonce, src_id, call)
		}

		/// Evaluate the state of a proposal given the current vote threshold.
		///
		/// A proposal with enough votes will be either executed or cancelled,
		/// and the status will be updated accordingly.
		///
		/// # <weight>
		/// - weight of proposed call, regardless of whether execution is
		///   performed
		/// # </weight>
		// FIXME #[pallet::weight(prop.get_dispatch_info().weight + 195_000_000, prop.get_dispatch_info().class,
		// Pays::Yes)]
		#[pallet::weight(prop.get_dispatch_info().weight + 195_000_000)]
		pub fn eval_vote_state(
			origin: OriginFor<T>,
			nonce: DepositNonce,
			src_id: ChainId,
			prop: Box<<T as Config>::Proposal>,
		) -> DispatchResultWithPostInfo {
			ensure_signed(origin)?;

			Self::try_resolve_proposal(nonce, src_id, prop)
		}
	}
}

impl<T: Config> Pallet<T> {
	// *** Utility methods ***

	pub fn ensure_admin(o: T::Origin) -> DispatchResultWithPostInfo {
		T::AdminOrigin::try_origin(o).map(|_| ()).or_else(ensure_root)?;
		Ok(().into())
	}

	/// Checks if who is a relayer
	pub fn is_relayer(who: &T::AccountId) -> bool {
		Self::relayers(who)
	}

	/// Provides an AccountId for the pallet.
	/// This is used both as an origin check and deposit/withdrawal account.
	pub fn account_id() -> T::AccountId {
		T::PalletId::get().into_account()
	}

	/// Asserts if a resource is registered
	pub fn resource_exists(id: ResourceId) -> bool {
		return Self::resources(id) != None;
	}

	/// Checks if a chain exists as a whitelisted destination
	pub fn chain_whitelisted(id: ChainId) -> bool {
		return Self::chains(id) != None;
	}

	// *** Admin methods ***

	/// Set a new voting threshold
	pub fn set_relayer_threshold(threshold: u32) -> DispatchResultWithPostInfo {
		ensure!(threshold > 0, Error::<T>::InvalidThreshold);
		RelayerThreshold::<T>::put(threshold);
		Self::deposit_event(Event::RelayerThresholdChanged(threshold));
		Ok(().into())
	}

	/// Register a method for a resource Id, enabling associated transfers
	pub fn register_resource(id: ResourceId, method: Vec<u8>) -> DispatchResultWithPostInfo {
		Resources::<T>::insert(id, method);
		Ok(().into())
	}

	/// Removes a resource ID, disabling associated transfer
	pub fn unregister_resource(id: ResourceId) -> DispatchResultWithPostInfo {
		Resources::<T>::remove(id);
		Ok(().into())
	}

	/// Whitelist a chain ID for transfer
	pub fn whitelist(id: ChainId) -> DispatchResultWithPostInfo {
		// Cannot whitelist this chain
		ensure!(id != T::ChainId::get(), Error::<T>::InvalidChainId);
		// Cannot whitelist with an existing entry
		ensure!(!Self::chain_whitelisted(id), Error::<T>::ChainAlreadyWhitelisted);
		ChainNonces::<T>::insert(&id, 0);
		Self::deposit_event(Event::ChainWhitelisted(id));
		Ok(().into())
	}

	/// Adds a new relayer to the set
	pub fn register_relayer(relayer: T::AccountId) -> DispatchResultWithPostInfo {
		ensure!(!Self::is_relayer(&relayer), Error::<T>::RelayerAlreadyExists);
		Relayers::<T>::insert(&relayer, true);
		RelayerCount::<T>::mutate(|i| *i += 1);

		Self::deposit_event(Event::RelayerAdded(relayer));
		Ok(().into())
	}

	/// Removes a relayer from the set
	pub fn unregister_relayer(relayer: T::AccountId) -> DispatchResultWithPostInfo {
		ensure!(Self::is_relayer(&relayer), Error::<T>::RelayerInvalid);
		Relayers::<T>::remove(&relayer);
		RelayerCount::<T>::mutate(|i| *i -= 1);
		Self::deposit_event(Event::RelayerRemoved(relayer));
		Ok(().into())
	}

	// *** Proposal voting and execution methods ***

	/// Commits a vote for a proposal. If the proposal doesn't exist it will be
	/// created.
	fn commit_vote(
		who: T::AccountId,
		nonce: DepositNonce,
		src_id: ChainId,
		prop: Box<T::Proposal>,
		in_favour: bool,
	) -> DispatchResultWithPostInfo {
		let now = <frame_system::Module<T>>::block_number();
		let mut votes = match Votes::<T>::get(src_id, (nonce, prop.clone())) {
			Some(v) => v,
			None => {
				let mut v = ProposalVotes::default();
				v.expiry = now + T::ProposalLifetime::get();
				v
			}
		};

		// Ensure the proposal isn't complete and relayer hasn't already voted
		ensure!(!votes.is_complete(), Error::<T>::ProposalAlreadyComplete);
		ensure!(!votes.is_expired(now), Error::<T>::ProposalExpired);
		ensure!(!votes.has_voted(&who), Error::<T>::RelayerAlreadyVoted);

		if in_favour {
			votes.votes_for.push(who.clone());
			Self::deposit_event(Event::VoteFor(src_id, nonce, who.clone()));
		} else {
			votes.votes_against.push(who.clone());
			Self::deposit_event(Event::VoteAgainst(src_id, nonce, who.clone()));
		}

		Votes::<T>::insert(src_id, (nonce, prop.clone()), votes.clone());

		Ok(().into())
	}

	/// Attempts to finalize or cancel the proposal if the vote count allows.
	fn try_resolve_proposal(
		nonce: DepositNonce,
		src_id: ChainId,
		prop: Box<T::Proposal>,
	) -> DispatchResultWithPostInfo {
		if let Some(mut votes) = Votes::<T>::get(src_id, (nonce, prop.clone())) {
			let now = <frame_system::Module<T>>::block_number();
			ensure!(!votes.is_complete(), Error::<T>::ProposalAlreadyComplete);
			ensure!(!votes.is_expired(now), Error::<T>::ProposalExpired);

			let status = votes.try_to_complete(RelayerThreshold::<T>::get(), RelayerCount::<T>::get());
			Votes::<T>::insert(src_id, (nonce, prop.clone()), votes.clone());

			match status {
				ProposalStatus::Approved => Self::finalize_execution(src_id, nonce, prop),
				ProposalStatus::Rejected => Self::cancel_execution(src_id, nonce),
				_ => Ok(().into()),
			}
		} else {
			Err(Error::<T>::ProposalDoesNotExist)?
		}
	}

	/// Commits a vote in favour of the proposal and executes it if the vote
	/// threshold is met.
	fn vote_for(
		who: T::AccountId,
		nonce: DepositNonce,
		src_id: ChainId,
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
		src_id: ChainId,
		prop: Box<T::Proposal>,
	) -> DispatchResultWithPostInfo {
		Self::commit_vote(who, nonce, src_id, prop.clone(), false)?;
		Self::try_resolve_proposal(nonce, src_id, prop)
	}

	/// Execute the proposal and signals the result as an event
	fn finalize_execution(src_id: ChainId, nonce: DepositNonce, call: Box<T::Proposal>) -> DispatchResultWithPostInfo {
		Self::deposit_event(Event::ProposalApproved(src_id, nonce));
		call.dispatch(frame_system::RawOrigin::Signed(Self::account_id()).into())
			.map(|_| ())
			.map_err(|e| e.error)?;
		Self::deposit_event(Event::ProposalSucceeded(src_id, nonce));
		Ok(().into())
	}

	/// Cancels a proposal.
	fn cancel_execution(src_id: ChainId, nonce: DepositNonce) -> DispatchResultWithPostInfo {
		Self::deposit_event(Event::ProposalRejected(src_id, nonce));
		Ok(().into())
	}
}

/// Simple ensure origin for the bridge account
pub struct EnsureBridge<T>(sp_std::marker::PhantomData<T>);
impl<T: Config> EnsureOrigin<T::Origin> for EnsureBridge<T> {
	type Success = T::AccountId;

	fn try_origin(o: T::Origin) -> Result<Self::Success, T::Origin> {
		let bridge_id = T::PalletId::get().into_account();
		o.into().and_then(|o| match o {
			system::RawOrigin::Signed(who) if who == bridge_id => Ok(bridge_id),
			r => Err(T::Origin::from(r)),
		})
	}
}
