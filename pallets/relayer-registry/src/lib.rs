// This file is part of Substrate.

// Copyright (C) 2019-2022 Parity Technologies (UK) Ltd.
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

//! # Identity Pallet
//!
//! - [`Config`]
//! - [`Call`]
//!
//! ## Overview
//!
//! A federated naming system, allowing for multiple registrars to be added from a specified origin.
//! Registrars can set a fee to provide identity-verification service. Anyone can put forth a
//! proposed identity for a fixed deposit and ask for review by any number of registrars (paying
//! each of their fees). Registrar judgements are given as an `enum`, allowing for sophisticated,
//! multi-tier opinions.
//!
//! Some judgements are identified as *sticky*, which means they cannot be removed except by
//! complete removal of the identity, or by the registrar. Judgements are allowed to represent a
//! portion of funds that have been reserved for the registrar.
//!
//! A super-user can remove accounts and in doing so, slash the deposit.
//!
//! All accounts may also have a limited number of sub-accounts which may be specified by the owner;
//! by definition, these have equivalent ownership and each has an individual name.
//!
//! The number of registrars should be limited, and the deposit made sufficiently large, to ensure
//! no state-bloat attack is viable.
//!
//! ## Interface
//!
//! ### Dispatchable Functions
//!
//! #### For general users
//! * `set_resource` - Set the associated identity of an account; a small deposit is reserved if not
//!   already taken.
//! * `clear_resource` - Remove an account's associated identity; the deposit is returned.
//! * `request_judgement` - Request a judgement from a registrar, paying a fee.
//! * `cancel_request` - Cancel the previous request for a judgement.
//!
//! #### For general users with sub-identities
//! * `set_subs` - Set the sub-accounts of an identity.
//! * `add_sub` - Add a sub-identity to an identity.
//! * `remove_sub` - Remove a sub-identity of an identity.
//! * `rename_sub` - Rename a sub-identity of an identity.
//! * `quit_sub` - Remove a sub-identity of an identity (called by the sub-identity).
//!
//! #### For registrars
//! * `set_fee` - Set the fee required to be paid for a judgement to be given by the registrar.
//! * `set_fields` - Set the fields that a registrar cares about in their judgements.
//! * `provide_judgement` - Provide a judgement to an identity.
//!
//! #### For super-users
//! * `add_registrar` - Add a new registrar to the system.
//! * `kill_identity` - Forcibly remove the associated identity; the deposit is lost.
//!
//! [`Call`]: ./enum.Call.html
//! [`Config`]: ./trait.Config.html

#![cfg_attr(not(feature = "std"), no_std)]

mod benchmarking;
#[cfg(test)]
pub mod mock;
#[cfg(test)]
mod tests;

pub mod types;

mod weights;
use weights::WeightInfo;

use types::*;

use frame_support::traits::{Currency, ReservableCurrency};
use sp_runtime::traits::{AppendZerosInput, Zero};
use sp_std::{convert::TryInto, prelude::*};

pub use pallet::*;

type BalanceOf<T> =
	<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::{pallet_prelude::*, Blake2_128Concat};
	use frame_system::pallet_prelude::*;
	use webb_primitives::webb_proposals::ResourceId;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// The overarching event type.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		/// The currency trait.
		type Currency: ReservableCurrency<Self::AccountId>;

		/// The amount held on deposit for a registered identity
		#[pallet::constant]
		type BasicDeposit: Get<BalanceOf<Self>>;

		/// The amount held on deposit per additional field for a registered identity.
		#[pallet::constant]
		type FieldDeposit: Get<BalanceOf<Self>>;

		/// Maximum number of additional fields that may be stored in an ID. Needed to bound the I/O
		/// required to access an identity, but can be pretty high.
		#[pallet::constant]
		type MaxAdditionalFields: Get<u32>;

		/// The origin which may forcibly set or remove a name. Root can always do this.
		type ForceOrigin: EnsureOrigin<Self::Origin>;

		type WeightInfo: WeightInfo;
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	/// A map that allows accounts to store metadata about each resource they are interested in.
	///
	/// TWOX-NOTE: OK ― `AccountId` is a secure hash.
	#[pallet::storage]
	#[pallet::getter(fn identity)]
	pub(super) type ResourceOf<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		T::AccountId,
		Blake2_128Concat,
		ResourceId,
		ResourceRecord<BalanceOf<T>, T::MaxAdditionalFields>,
	>;

	#[pallet::error]
	pub enum Error<T> {
		/// Account isn't found.
		NotFound,
		/// Account isn't named.
		NotNamed,
		/// Too many additional fields.
		TooManyFields,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// A resource was set or reset (which will remove all judgements).
		ResourceSet { who: T::AccountId },
		/// A resource was cleared, and the given balance returned.
		ResourceCleared { who: T::AccountId, deposit: BalanceOf<T> },
	}

	#[pallet::call]
	/// Identity pallet declaration.
	impl<T: Config> Pallet<T> {
		/// Set a resource's information and reserve the appropriate deposit.
		///
		/// If the resource already has resource information, the deposit is taken as part payment
		/// for the new deposit.
		///
		/// The dispatch origin for this call must be _Signed_.
		///
		/// - `info`: The resource information.
		///
		/// Emits `ResourceSet` if successful.
		#[pallet::weight(T::WeightInfo::set_resource())]
		pub fn set_resource(
			origin: OriginFor<T>,
			resource_id: ResourceId,
			info: Box<ResourceInfo<T::MaxAdditionalFields>>,
		) -> DispatchResultWithPostInfo {
			let sender = ensure_signed(origin)?;
			let extra_fields = info.additional.len() as u32;
			ensure!(extra_fields <= T::MaxAdditionalFields::get(), Error::<T>::TooManyFields);
			let fd = <BalanceOf<T>>::from(extra_fields) * T::FieldDeposit::get();

			let mut id = match <ResourceOf<T>>::get(&sender, resource_id) {
				Some(mut id) => {
					id.info = *info;
					id
				},
				None => ResourceRecord { info: *info, deposit: Zero::zero() },
			};

			let old_deposit = id.deposit;
			id.deposit = T::BasicDeposit::get() + fd;
			if id.deposit > old_deposit {
				T::Currency::reserve(&sender, id.deposit - old_deposit)?;
			}
			if old_deposit > id.deposit {
				let err_amount = T::Currency::unreserve(&sender, old_deposit - id.deposit);
				debug_assert!(err_amount.is_zero());
			}

			<ResourceOf<T>>::insert(&sender, resource_id, id);
			Self::deposit_event(Event::ResourceSet { who: sender });

			Ok(().into())
		}

		/// Clear an account's resource record.
		///
		/// Payment: All reserved balances on the account are returned.
		///
		/// The dispatch origin for this call must be _Signed_ and the sender must have a registered
		/// resource.
		///
		/// Emits `ResourceCleared` if successful.
		#[pallet::weight(T::WeightInfo::clear_resource())]
		pub fn clear_resource(
			origin: OriginFor<T>,
			resource_id: ResourceId,
		) -> DispatchResultWithPostInfo {
			let sender = ensure_signed(origin)?;

			let id = <ResourceOf<T>>::take(&sender, resource_id).ok_or(Error::<T>::NotNamed)?;
			let deposit = id.deposit;

			let err_amount = T::Currency::unreserve(&sender, deposit);
			debug_assert!(err_amount.is_zero());

			Self::deposit_event(Event::ResourceCleared { who: sender, deposit });
			Ok(().into())
		}
	}
}
