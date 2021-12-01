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

//! # Hasher Module
//!
//! A simple module for abstracting over arbitrary hash functions primarily
//! for zero-knowledge friendly hash functions that have potentially large
//! parameters to deal with.
//!
//! ## Overview
//!
//! The Hasher module provides functionality for hash function management
//! including:
//!
//! * Setting parameters for hash functions
//! * Setting the maintainer of the parameters
//!
//! To use it in your runtime, you need to implement the hasher [`Config`].
//! Additionally, you will want to implement the hash traits defined in the
//! darkwebb_primitives::hasher module.
//!
//! The supported dispatchable functions are documented in the [`Call`] enum.
//!
//! ### Terminology
//!
//! ### Goals
//!
//! The hasher system in Webb is designed to make the following possible:
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

mod benchmarking;
#[cfg(test)]
pub mod mock;
#[cfg(test)]
mod tests;
pub mod weights;

use sp_runtime::traits::{Saturating, Zero};
use sp_std::{prelude::*, vec};

use darkwebb_primitives::{hasher::*, types::DepositDetails};
use frame_support::{
	pallet_prelude::{ensure, DispatchError},
	traits::{Currency, ReservableCurrency},
};
use frame_system::Config as SystemConfig;

type DepositBalanceOf<T, I = ()> = <<T as Config<I>>::Currency as Currency<<T as SystemConfig>::AccountId>>::Balance;

pub use pallet::*;
pub use weights::WeightInfo;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::{dispatch::DispatchResultWithPostInfo, pallet_prelude::*};
	use frame_system::pallet_prelude::*;

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T, I = ()>(_);

	#[pallet::config]
	/// The module configuration trait.
	pub trait Config<I: 'static = ()>: frame_system::Config {
		/// The overarching event type.
		type Event: From<Event<Self, I>> + IsType<<Self as frame_system::Config>::Event>;

		/// The hash instance trait
		type Hasher: InstanceHasher;

		/// The currency mechanism.
		type Currency: ReservableCurrency<Self::AccountId>;

		/// The origin which may forcibly reset parameters or otherwise alter
		/// privileged attributes.
		type ForceOrigin: EnsureOrigin<Self::Origin>;

		/// The basic amount of funds that must be reserved for an asset.
		type ParameterDeposit: Get<DepositBalanceOf<Self, I>>;

		/// The basic amount of funds that must be reserved when adding metadata
		/// to your parameters.
		type MetadataDepositBase: Get<DepositBalanceOf<Self, I>>;

		/// The additional funds that must be reserved for the number of bytes
		/// you store in your parameter metadata.
		type MetadataDepositPerByte: Get<DepositBalanceOf<Self, I>>;

		/// The maximum length of a name or symbol stored on-chain.
		type StringLimit: Get<u32>;

		/// Weightinfo for pallet.
		type WeightInfo: WeightInfo;
	}

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config<I>, I: 'static = ()> {
		pub phantom: (PhantomData<T>, PhantomData<I>),
		pub parameters: Option<Vec<u8>>,
	}

	#[cfg(feature = "std")]
	impl<T: Config<I>, I: 'static> Default for GenesisConfig<T, I> {
		fn default() -> Self {
			Self {
				phantom: Default::default(),
				parameters: None,
			}
		}
	}

	#[pallet::genesis_build]
	impl<T: Config<I>, I: 'static> GenesisBuild<T, I> for GenesisConfig<T, I> {
		fn build(&self) {
			if let Some(params) = &self.parameters {
				Parameters::<T, I>::put(params);
			}
		}
	}

	#[pallet::storage]
	#[pallet::getter(fn parameters)]
	/// Details of the module's parameters
	pub(super) type Parameters<T: Config<I>, I: 'static = ()> = StorageValue<_, Vec<u8>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn existing_deposit)]
	/// Details of the module's parameters
	pub(super) type Deposit<T: Config<I>, I: 'static = ()> =
		StorageValue<_, Option<DepositDetails<T::AccountId, DepositBalanceOf<T, I>>>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn maintainer)]
	/// The parameter maintainer who can change the parameters
	pub(super) type Maintainer<T: Config<I>, I: 'static = ()> = StorageValue<_, T::AccountId, ValueQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config<I>, I: 'static = ()> {
		ParametersSet {
			who: T::AccountId,
			parameters: Vec<u8>,
		},
		MaintainerSet {
			old_maintainer: T::AccountId,
			new_maintainer: T::AccountId,
		},
	}

	#[pallet::error]
	pub enum Error<T, I = ()> {
		/// Parameters haven't been initialized
		ParametersNotInitialized,
		/// Parameters are invalid or empty
		InvalidParameters,
		/// Account does not have correct permissions
		InvalidPermissions,
		/// Error during hashing
		HashError,
	}

	#[pallet::hooks]
	impl<T: Config<I>, I: 'static> Hooks<BlockNumberFor<T>> for Pallet<T, I> {}

	#[pallet::call]
	impl<T: Config<I>, I: 'static> Pallet<T, I> {
		#[pallet::weight(T::WeightInfo::set_parameters(parameters.len() as u32))]
		pub fn set_parameters(origin: OriginFor<T>, parameters: Vec<u8>) -> DispatchResultWithPostInfo {
			let origin = ensure_signed(origin)?;
			// ensure parameter setter is the maintainer
			ensure!(origin == Self::maintainer(), Error::<T, I>::InvalidPermissions);
			// calculate the deposit
			let deposit = T::MetadataDepositPerByte::get()
				.saturating_mul((parameters.len() as u32).into())
				.saturating_add(T::MetadataDepositBase::get());
			// get old deposit details if they exist
			let old_deposit_details = Self::existing_deposit().unwrap_or_default();
			// reserve and unreserve the currrency amounts
			if old_deposit_details.depositor == origin {
				// handle when the current origin is the same as previous depositor
				if deposit > old_deposit_details.deposit {
					T::Currency::reserve(&origin, deposit - old_deposit_details.deposit)?;
				} else {
					T::Currency::unreserve(&origin, old_deposit_details.deposit - deposit);
				}
			} else {
				// handle when the current origin is different from old depositor
				T::Currency::reserve(&origin, deposit)?;
				T::Currency::unreserve(&old_deposit_details.depositor, old_deposit_details.deposit);
			}

			Parameters::<T, I>::try_mutate(|params| {
				*params = parameters.clone();
				Self::deposit_event(Event::ParametersSet {
					who: origin,
					parameters,
				});
				Ok(().into())
			})
		}

		#[pallet::weight(T::WeightInfo::set_maintainer())]
		pub fn set_maintainer(origin: OriginFor<T>, new_maintainer: T::AccountId) -> DispatchResultWithPostInfo {
			let origin = ensure_signed(origin)?;
			// ensure parameter setter is the maintainer
			ensure!(origin == Self::maintainer(), Error::<T, I>::InvalidPermissions);
			// set the new maintainer
			Maintainer::<T, I>::try_mutate(|maintainer| {
				*maintainer = new_maintainer.clone();
				Self::deposit_event(Event::MaintainerSet {
					old_maintainer: origin,
					new_maintainer,
				});
				Ok(().into())
			})
		}

		#[pallet::weight(T::WeightInfo::force_set_parameters(parameters.len() as u32))]
		pub fn force_set_parameters(origin: OriginFor<T>, parameters: Vec<u8>) -> DispatchResultWithPostInfo {
			T::ForceOrigin::ensure_origin(origin)?;
			// get old deposit details if they exist
			let old_deposit_details = Self::existing_deposit().unwrap_or_default();
			// unreserve the currrency amounts from old depositor when force set
			if old_deposit_details.deposit > DepositBalanceOf::<T, I>::zero() {
				T::Currency::unreserve(&old_deposit_details.depositor, old_deposit_details.deposit);
			}

			Deposit::<T, I>::kill();

			Parameters::<T, I>::try_mutate(|params| {
				*params = parameters.clone();
				Self::deposit_event(Event::ParametersSet {
					who: Default::default(),
					parameters,
				});
				Ok(().into())
			})
		}

		#[pallet::weight(T::WeightInfo::force_set_maintainer())]
		pub fn force_set_maintainer(origin: OriginFor<T>, new_maintainer: T::AccountId) -> DispatchResultWithPostInfo {
			T::ForceOrigin::ensure_origin(origin)?;
			// set the new maintainer
			Maintainer::<T, I>::try_mutate(|maintainer| {
				*maintainer = new_maintainer.clone();
				Self::deposit_event(Event::MaintainerSet {
					old_maintainer: Default::default(),
					new_maintainer,
				});
				Ok(().into())
			})
		}
	}
}

impl<T: Config<I>, I: 'static> HasherModule for Pallet<T, I> {
	fn hash(data: &[u8]) -> Result<Vec<u8>, DispatchError> {
		let params = Self::parameters();
		ensure!(!params.is_empty(), Error::<T, I>::ParametersNotInitialized);
		match T::Hasher::hash(data, &params) {
			Ok(hash) => Ok(hash),
			Err(_e) => {
				println!("{:?}", _e);
				// TODO: Handle properly
				ensure!(false, Error::<T, I>::HashError);
				Ok(vec![])
			}
		}
	}

	fn hash_two(left: &[u8], right: &[u8]) -> Result<Vec<u8>, DispatchError> {
		let mut buf = vec![];
		buf.extend_from_slice(left);
		buf.extend_from_slice(right);
		Self::hash(&buf)
	}
}
