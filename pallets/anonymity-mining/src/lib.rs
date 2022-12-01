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

//! # Anonymity Mining Module
//!
//! ## Overview
//!
//! The supported dispatchable functions are documented in the [`Call`] enum.

// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
pub mod mock;
#[cfg(test)]
mod tests;

use frame_support::{
	pallet_prelude::{ensure, DispatchError},
	sp_runtime::traits::AccountIdConversion,
	traits::Get,
	PalletId,
};
use orml_traits::{currency::transactional, MultiCurrency};
use pallet_vanchor::VAnchorConfigration;
use sp_std::{convert::TryInto, prelude::*, vec};
use webb_primitives::traits::vanchor::{VAnchorInspector, VAnchorInterface};

pub use pallet::*;

/// Type alias for the orml_traits::MultiCurrency::Balance type
pub type BalanceOf<T, I> =
	<<T as Config<I>>::Currency as MultiCurrency<<T as frame_system::Config>::AccountId>>::Balance;
/// Type alias for the orml_traits::MultiCurrency::CurrencyId type
pub type CurrencyIdOf<T, I> = <<T as pallet::Config<I>>::Currency as MultiCurrency<
	<T as frame_system::Config>::AccountId,
>>::CurrencyId;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::{dispatch::DispatchResultWithPostInfo, pallet_prelude::*};
	use frame_system::pallet_prelude::*;

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	#[pallet::without_storage_info]
	pub struct Pallet<T, I = ()>(_);

	#[pallet::config]
	/// The module configuration trait.
	pub trait Config<I: 'static = ()>:
		frame_system::Config + pallet_balances::Config + pallet_vanchor::Config<I>
	{
		/// The overarching event type.
		type RuntimeEvent: From<Event<Self, I>>
			+ IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// Account Identifier from which the internal Pot is generated.
		type PotId: Get<PalletId>;

		/// Currency type for taking deposits
		type Currency: MultiCurrency<Self::AccountId>;

		/// VAnchor Interface
		type VAnchor: VAnchorInterface<VAnchorConfigration<Self, I>>
			+ VAnchorInspector<VAnchorConfigration<Self, I>>;
		
		#[pallet::constant]
		type AnonymityPointsAssetId: Get<CurrencyIdOf<Self, I>>;

		/// Native currency id
		#[pallet::constant]
		type NativeCurrencyId: Get<CurrencyIdOf<Self, I>>;

		/// The origin which may forcibly reset parameters or otherwise alter
		/// privileged attributes.
		type ForceOrigin: EnsureOrigin<Self::RuntimeOrigin>;
	}

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config<I>, I: 'static = ()> {
		pub phantom: (PhantomData<T>, PhantomData<I>),
	}

	#[cfg(feature = "std")]
	impl<T: Config<I>, I: 'static> Default for GenesisConfig<T, I> {
		fn default() -> Self {
			Self { phantom: Default::default() }
		}
	}

	#[pallet::genesis_build]
	impl<T: Config<I>, I: 'static> GenesisBuild<T, I> for GenesisConfig<T, I> {
		fn build(&self) {}
	}

	#[pallet::storage]
	#[pallet::getter(fn parameters)]
	/// Details of the module's parameters
	pub(super) type Parameters<T: Config<I>, I: 'static = ()> =
		StorageValue<_, Vec<u8>, ValueQuery>;

	#[pallet::event]
	pub enum Event<T: Config<I>, I: 'static = ()> {}

	#[pallet::error]
	pub enum Error<T, I = ()> {
		/// Parameters haven't been initialized
		ParametersNotInitialized,
		/// Error during hashing
		HashError,
	}

	#[pallet::call]
	impl<T: Config<I>, I: 'static> Pallet<T, I> {
		#[pallet::weight(0)]
		pub fn swap(
			origin: OriginFor<T>,
			recipient: T::AccountId,
			amount: BalanceOf<T, I>,
		) -> DispatchResultWithPostInfo {
			Ok(().into())
		}
	}
}

impl<T: Config<I>, I: 'static> Pallet<T, I> {
	/// Get a unique, inaccessible account id from the `PotId`.
	pub fn account_id() -> T::AccountId {
		T::PotId::get().into_account_truncating()
	}
}
