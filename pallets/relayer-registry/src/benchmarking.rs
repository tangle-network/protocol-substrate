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

//! Verifier pallet benchmarking.

#![cfg(feature = "runtime-benchmarks")]

use super::*;

use frame_benchmarking::{benchmarks, impl_benchmark_test_suite, whitelisted_caller};
use frame_support::traits::Get;
use frame_system::RawOrigin;
use webb_primitives::webb_proposals::ResourceId;

fn assert_last_event<T: Config>(generic_event: <T as Config>::Event) {
	frame_system::Pallet::<T>::assert_last_event(generic_event.into());
}

benchmarks! {
	set_resource {
		let caller: T::AccountId = whitelisted_caller();
		let bridge_index = 0_u32;
		let resource_id : ResourceId = [0u8;32].into();
		let metadata : ResourceInfo<T::MaxAdditionalFields> = Default::default();
		T::Currency::make_free_balance_be(&caller.clone(), 200_000_000u32.into());
	}: _(RawOrigin::Signed(caller.clone()), resource_id, Box::new(metadata))
	verify {
		assert_last_event::<T>(Event::ResourceSet{ who : caller}.into())
	}

	clear_resource {
		let caller: T::AccountId = whitelisted_caller();
		let bridge_index = 0_u32;
		let resource_id : ResourceId = [0u8;32].into();
		let metadata : ResourceInfo<T::MaxAdditionalFields> = Default::default();
		T::Currency::make_free_balance_be(&caller.clone(), 200_000_000u32.into());
		Pallet::<T>::set_resource(RawOrigin::Signed(caller.clone()).into(), resource_id, Box::new(metadata)).unwrap();
	}: _(RawOrigin::Signed(caller.clone()), resource_id)
	verify {
		assert_last_event::<T>(Event::ResourceCleared{ who : caller, deposit : T::BasicDeposit::get() }.into())
	}
}

impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Test);
