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

//! Hasher pallet benchmarking.

#![cfg(feature = "runtime-benchmarks")]

use super::*;

use frame_benchmarking::{account, benchmarks, impl_benchmark_test_suite, whitelist_account, whitelisted_caller};
use frame_support::traits::{Currency, EnsureOrigin};
use frame_system::RawOrigin;

fn assert_last_event<T: Config>(generic_event: <T as Config>::Event) {
	frame_system::Pallet::<T>::assert_last_event(generic_event.into());
}

use crate::Pallet as HasherModule;

const SEED: u32 = 0;

benchmarks! {
	// Benchmark `set_parameters` extrinsic with the worst case scenario:
	set_parameters {
		let caller: T::AccountId = whitelisted_caller();
		let depositor: T::AccountId = account("depositor", 0, SEED);
		whitelist_account!(depositor);
		let parameters = vec![0u8;std::u32::MAX as usize];
		Maintainer::<T>::put::<T::AccountId>(caller.clone());

		<<T as Config>::Currency as Currency<T::AccountId>>::make_free_balance_be(&caller, std::u32::MAX.into());

		Deposit::<T>::put::<Option<DepositDetails<T::AccountId, DepositBalanceOf<T>>>>(Some(DepositDetails{
			depositor,
			deposit:1_000_000u32.into()
		}));

	}: _(RawOrigin::Signed(caller.clone()), parameters.clone())
	verify {
		assert_last_event::<T>(Event::ParametersSet(caller.into(), parameters).into());
	}



	set_maintainer {
		let caller: T::AccountId = whitelisted_caller();
		let new_maintainer: T::AccountId = account("maintainer", 0, SEED);
		Maintainer::<T>::put::<T::AccountId>(caller.clone());
	}: _(RawOrigin::Signed(caller.clone()), new_maintainer.clone())
	verify {
		assert_last_event::<T>(Event::MaintainerSet(caller.into(), new_maintainer.into()).into());
	}


	force_set_parameters {
		let caller = <T::ForceOrigin as EnsureOrigin<T::Origin>>::successful_origin();
		let depositor: T::AccountId = account("depositor", 0, SEED);
		let parameters = vec![0u8;std::u32::MAX as usize];

		Deposit::<T>::put::<Option<DepositDetails<T::AccountId, DepositBalanceOf<T>>>>(Some(DepositDetails{
			depositor,
			deposit:1_000_000u32.into()
		}));


	}: _(RawOrigin::Signed(caller.clone()), parameters.clone())
	verify {
		assert_last_event::<T>(Event::ParametersSet(Default::default(), parameters).into());
	}


	force_set_maintainer {
		let caller = <T::ForceOrigin as EnsureOrigin<T::Origin>>::successful_origin();
		let new_maintainer: T::AccountId = account("maintainer", 0, SEED);
	}: _(RawOrigin::Signed(caller.clone()), new_maintainer.clone())
	verify {
		assert_last_event::<T>(Event::MaintainerSet(Default::default(), new_maintainer.into()).into());
	}
}

impl_benchmark_test_suite!(HasherModule, crate::mock::new_test_ext(), crate::mock::Test);
