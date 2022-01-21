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

//! Pallet to show how we can use XCM/UMP to exchange messages between
//! parachains.

#![cfg_attr(not(feature = "std"), no_std)]

use cumulus_pallet_xcm::{ensure_sibling_para, Origin as CumulusOrigin};
use cumulus_primitives_core::ParaId;
use frame_system::Config as SystemConfig;
use sp_std::prelude::*;
use xcm::latest::prelude::*;

pub use pallet::*;

#[allow(clippy::large_enum_variant)]
#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	/// The module configuration trait.
	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// The overarching event type.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		type Origin: From<<Self as SystemConfig>::Origin>
			+ Into<Result<CumulusOrigin, <Self as Config>::Origin>>;

		/// The overarching call type; we assume sibling chains use the same
		/// type.
		type Call: From<Call<Self>> + Encode;

		type XcmSender: SendXcm;
	}

	/// The target parachains to exchange messages with.
	#[pallet::storage]
	pub(super) type Targets<T: Config> = StorageValue<_, Vec<ParaId>, ValueQuery>;

	/// Keeps track of block numbers.
	///
	/// A simple storage to show how we can share state between parachains.
	#[pallet::storage]
	pub(super) type MyBlockNumber<T: Config> = StorageValue<_, T::BlockNumber, ValueQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		BlockNumberUpdateSent { to: ParaId, value: T::BlockNumber },
		BlockNumberUpdated { from: ParaId, value: T::BlockNumber },
		ErrorSendingUpdateBlockNumber { error: SendError, to: ParaId },
		UnknownParachain(ParaId),
	}

	#[pallet::error]
	pub enum Error<T> {}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_finalize(n: T::BlockNumber) {
			for para in Targets::<T>::get().into_iter() {
				let update_block_number = Transact {
					origin_type: OriginKind::Native,
					require_weight_at_most: 1_000,
					call: <T as Config>::Call::from(Call::<T>::update_block_number { n })
						.encode()
						.into(),
				};
				let dest = (1, Junction::Parachain(para.into()));
				let result = T::XcmSender::send_xcm(dest, Xcm(vec![update_block_number]));
				match result {
					Ok(()) => {
						Self::deposit_event(Event::BlockNumberUpdateSent { to: para, value: n });
					},
					Err(e) => {
						Self::deposit_event(Event::ErrorSendingUpdateBlockNumber {
							error: e,
							to: para,
						});
					},
				}
			}
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(0)]
		pub fn start(origin: OriginFor<T>, para: ParaId) -> DispatchResult {
			ensure_signed(origin)?;
			Targets::<T>::mutate(|t| t.push(para));
			Ok(())
		}

		#[pallet::weight(0)]
		pub fn stop(origin: OriginFor<T>, para: ParaId) -> DispatchResult {
			ensure_signed(origin)?;
			Targets::<T>::mutate(|t| {
				if let Some(p) = t.iter().position(|p| p == &para) {
					t.swap_remove(p);
				}
			});
			Ok(())
		}

		#[pallet::weight(0)]
		pub fn update_block_number(origin: OriginFor<T>, n: T::BlockNumber) -> DispatchResult {
			// Only accept this call from other chains.
			let para = ensure_sibling_para(<T as Config>::Origin::from(origin))?;
			MyBlockNumber::<T>::mutate(|v| *v = n);
			Self::deposit_event(Event::BlockNumberUpdated { from: para, value: n });
			Ok(())
		}
	}
}
