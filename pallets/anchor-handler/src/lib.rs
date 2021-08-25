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

//! # Anchor Handler Module
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
//! The anchor handler system in Webb is designed to make the following
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

use sp_std::prelude::*;

use frame_support::pallet_prelude::ensure;

pub use pallet::*;

type ResourceId = pallet_bridge::types::ResourceId;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::{dispatch::DispatchResultWithPostInfo, pallet_prelude::*};
	use frame_system::pallet_prelude::*;

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::config]
	/// The module configuration trait.
	pub trait Config: frame_system::Config + pallet_bridge::Config {
		/// The overarching event type.
		type Event: IsType<<Self as frame_system::Config>::Event> + From<Event<Self>>;

		/// Specifies the origin check provided by the bridge for calls that can
		/// only be called by the bridge pallet
		type BridgeOrigin: EnsureOrigin<Self::Origin, Success = Self::AccountId>;
	}

	#[pallet::storage]
	#[pallet::getter(fn maintainer)]
	/// The parameter maintainer who can change the parameters
	pub(super) type Maintainer<T: Config> = StorageValue<_, T::AccountId, ValueQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	#[pallet::metadata(T::AccountId = "AccountId")]
	pub enum Event<T: Config> {
		MaintainerSet(T::AccountId, T::AccountId),
	}

	#[pallet::error]
	pub enum Error<T> {
		InvalidPermissions,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(195_000_000)]
		pub fn set_maintainer(origin: OriginFor<T>, new_maintainer: T::AccountId) -> DispatchResultWithPostInfo {
			T::BridgeOrigin::ensure_origin(origin.clone())?;
			let origin = ensure_signed(origin)?;
			// ensure parameter setter is the maintainer
			ensure!(origin == Self::maintainer(), Error::<T>::InvalidPermissions);
			// set the new maintainer
			Maintainer::<T>::try_mutate(|maintainer| {
				*maintainer = new_maintainer.clone();
				Self::deposit_event(Event::MaintainerSet(origin, new_maintainer));
				Ok(().into())
			})
		}

		#[pallet::weight(195_000_000)]
		pub fn force_set_maintainer(origin: OriginFor<T>, new_maintainer: T::AccountId) -> DispatchResultWithPostInfo {
			T::BridgeOrigin::ensure_origin(origin)?;
			// set the new maintainer
			Maintainer::<T>::try_mutate(|maintainer| {
				*maintainer = new_maintainer.clone();
				Self::deposit_event(Event::MaintainerSet(Default::default(), T::AccountId::default()));
				Ok(().into())
			})
		}
	}
}
