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

use darkwebb_primitives::types::DepositDetails;
use frame_benchmarking::{account, benchmarks_instance_pallet, impl_benchmark_test_suite, whitelist_account, whitelisted_caller};
use frame_support::traits::Currency;
use frame_system::RawOrigin;
use sp_runtime::traits::Bounded;
type BalanceOf<T, I> =
	<<T as Config<I>>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

fn assert_last_event<T: Config<I>, I: 'static>(generic_event: <T as Config<I>>::Event) {
	frame_system::Pallet::<T>::assert_last_event(generic_event.into());
}

const SEED: u32 = 0;
// Based on verifier bytes generated from the zero knowledge setup for anchor pallet and mixer pallet,
// Max verifier bytes length generated ranged between 456 - 552
const MAX_VERIFIER_LENGTH: u32 = 1024;

benchmarks_instance_pallet! {

	set_parameters {
		let c in 0..MAX_VERIFIER_LENGTH;
		let caller: T::AccountId = whitelisted_caller();
		let depositor: T::AccountId = account("depositor", 0, SEED);
		whitelist_account!(depositor);
		let parameters = vec![0u8;c as usize];
		Maintainer::<T, I>::put::<T::AccountId>(caller.clone());

		<<T as Config<I>>::Currency as Currency<T::AccountId>>::make_free_balance_be(&caller, BalanceOf::<T, I>::max_value());

		Deposit::<T, I>::put::<Option<DepositDetails<T::AccountId, DepositBalanceOf<T, I>>>>(Some(DepositDetails{
			depositor,
			deposit:1_000u32.into()
		}));

	}: _(RawOrigin::Signed(caller.clone()), parameters.clone())
	verify {
		assert_last_event::<T, I>(Event::ParametersSet(caller, parameters).into());
	}



	set_maintainer {
		let caller: T::AccountId = whitelisted_caller();
		let new_maintainer: T::AccountId = account("maintainer", 0, SEED);
		Maintainer::<T, I>::put::<T::AccountId>(caller.clone());
	}: _(RawOrigin::Signed(caller.clone()), new_maintainer.clone())
	verify {
		assert_last_event::<T, I>(Event::MaintainerSet(caller, new_maintainer.into()).into());
	}


	force_set_parameters {
		let c in 0..MAX_VERIFIER_LENGTH;
		let depositor: T::AccountId = account("depositor", 0, SEED);
		let parameters = vec![0u8;c as usize];

		Deposit::<T, I>::put::<Option<DepositDetails<T::AccountId, DepositBalanceOf<T, I>>>>(Some(DepositDetails{
			depositor,
			deposit:1_000u32.into()
		}));


	}: _(RawOrigin::Root, parameters.clone())
	verify {
		assert_last_event::<T, I>(Event::ParametersSet(Default::default(), parameters).into());
	}


	force_set_maintainer {
		let new_maintainer: T::AccountId = account("maintainer", 0, SEED);
	}: _(RawOrigin::Root, new_maintainer.clone())
	verify {
		assert_last_event::<T, I>(Event::MaintainerSet(Default::default(), new_maintainer.into()).into());
	}

	
}

impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Test);


