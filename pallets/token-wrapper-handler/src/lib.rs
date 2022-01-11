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

//! # Token Wrapper Handler Module
//!
//! A module for executing wrapping fee, add/remove token proposals.
//! These functions can only be called by the bridge.
//!
//! ## Overview
//!
//! The Token Wrapper Handler module provides functionality for token wrapping
//! management including:
//!
//! * Executing proposal to change the wrapping fee
//! * Executing a proposal to add a token to the asset registry
//! * Executing a proposal to remove a token from the asset registry
//!
//!
//! The supported dispatchable functions are documented in the [`Call`] enum.
//!
//! ### Dispatchable Functions
//! * execute_wrapping_fee_proposal
//! * execute_add_token_proposal
//! * execute_remove_token_proposal
//!
//! ## Interface
//!
//! ## Related Modules
//! * Token-Wrapper Pallet
//! * Bridge Pallet
//!
//! * [`System`](../frame_system/index.html)
//! * [`Support`](../frame_support/index.html)

// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
pub mod mock;
#[cfg(test)]
mod tests;

use frame_support::{dispatch::DispatchResultWithPostInfo, ensure, traits::EnsureOrigin};
use frame_system::pallet_prelude::OriginFor;

///TODO: Define BalanceOf
use pallet_token_wrapper::BalanceOf;

use darkwebb_primitives::ResourceId;

use pallet_token_wrapper::traits::TokenWrapperInterface;

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::{dispatch::DispatchResultWithPostInfo, pallet_prelude::*};
	use frame_system::pallet_prelude::*;

	#[pallet::config]
	pub trait Config: frame_system::Config + pallet_token_wrapper::Config {
		/// The overarching event type.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		type BridgeOrigin: EnsureOrigin<Self::Origin, Success = Self::AccountId>;

		/// TokenWrapper Interface
		type TokenWrapper: TokenWrapperInterface<Self::AccountId, Self::AssetId, BalanceOf<Self>>;
	}

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		UpdatedWrappingFeePercent { wrapping_fee_percent: BalanceOf<T> },
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Access violation.
		InvalidPermissions,
		// TokenWrapperHandler already exists for specified resource Id.
		ResourceIsAlreadyAnchored,
		// TokenWrapper handler doesn't exist for specified resoure Id.
		TokenWrapperHandlerNotFound,
		/// Storage overflowed.
		StorageOverflow,
	}
	/// Execute the wrapping fee proposal by calling the update_wrapping_fee
	/// method Ensures that only the bridge can call this function
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(195_000_000)]
		pub fn execute_wrapping_fee_proposal(
			origin: OriginFor<T>,
			r_id: ResourceId,
			wrapping_fee_percent: BalanceOf<T>,
		) -> DispatchResultWithPostInfo {
			// TODO: Define and check validity conditions.
			T::BridgeOrigin::ensure_origin(origin)?;
			Self::update_wrapping_fee(r_id, wrapping_fee_percent)?;
			Ok(().into())
		}
	}
}

impl<T: Config> Pallet<T> {
	/// Updates the wrapping fee by calling the set_wrapping_fee method on the
	/// TokenWrapper Pallet
	fn update_wrapping_fee(
		r_id: ResourceId,
		wrapping_fee_percent: BalanceOf<T>,
	) -> Result<(), frame_support::dispatch::DispatchError> {
		T::TokenWrapper::set_wrapping_fee(wrapping_fee_percent)
	}
}
