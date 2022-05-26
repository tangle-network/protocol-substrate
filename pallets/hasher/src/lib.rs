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
//! webb_primitives::hasher module.
//!
//! The supported dispatchable functions are documented in the [`Call`] enum.

// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

mod benchmarking;
#[cfg(test)]
pub mod mock;
#[cfg(test)]
mod tests;
pub mod weights;

use frame_support::pallet_prelude::{ensure, DispatchError};
use sp_std::{prelude::*, vec};
use webb_primitives::hasher::*;
use core::convert::TryInto;

pub use pallet::*;
pub use weights::WeightInfo;

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
	pub trait Config<I: 'static = ()>: frame_system::Config {
		/// The overarching event type.
		type Event: From<Event<Self, I>> + IsType<<Self as frame_system::Config>::Event>;

		/// The hash instance trait
		type Hasher: InstanceHasher;

		/// The origin which may forcibly reset parameters or otherwise alter
		/// privileged attributes.
		type ForceOrigin: EnsureOrigin<Self::Origin>;

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
			Self { phantom: Default::default(), parameters: None }
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

	#[pallet::hooks]
	impl<T: Config<I>, I: 'static> Hooks<BlockNumberFor<T>> for Pallet<T, I> {}

	#[pallet::call]
	impl<T: Config<I>, I: 'static> Pallet<T, I> {
		#[pallet::weight(T::WeightInfo::force_set_parameters(parameters.len() as u32))]
		pub fn force_set_parameters(
			origin: OriginFor<T>,
			parameters: Vec<u8>,
		) -> DispatchResultWithPostInfo {
			T::ForceOrigin::ensure_origin(origin)?;
			Parameters::<T, I>::try_mutate(|params| {
				*params = parameters.clone();
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
				ensure!(false, Error::<T, I>::HashError);
				Ok(vec![])
			},
		}
	}

	fn hash_two(left: &[u8], right: &[u8]) -> Result<Vec<u8>, DispatchError> {
		let mut buf = vec![];
		buf.extend_from_slice(left);
		buf.extend_from_slice(right);
		Self::hash(&buf)
	}
}
