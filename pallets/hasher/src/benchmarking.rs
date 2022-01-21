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

use frame_benchmarking::{
	account, benchmarks_instance_pallet, impl_benchmark_test_suite, whitelist_account,
	whitelisted_caller,
};
use frame_support::traits::Currency;
use frame_system::RawOrigin;
use sp_runtime::traits::Bounded;
use webb_primitives::types::DepositDetails;
type BalanceOf<T, I> =
	<<T as Config<I>>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

fn assert_last_event<T: Config<I>, I: 'static>(generic_event: <T as Config<I>>::Event) {
	frame_system::Pallet::<T>::assert_last_event(generic_event.into());
}

const SEED: u32 = 0;
// Based on parameters generated from these functions below  using the
// arkworks_gadgets package, 16k was the max parameter length, so it's safe to
// benchmark with 20k
// poseidon_bls381_x5_5 poseidon_bn254_x3_5
// poseidon_circom_bn254_x5_5
// poseidon_circom_bn254_x5_3
// poseidon_bls381_x3_5
// poseidon_circom_bn254_x5_5
// poseidon_circom_bn254_x5_3
const MAX_PARAMETER_LENGTH: u32 = 20000;

benchmarks_instance_pallet! {
	force_set_parameters {
		let c in 0..MAX_PARAMETER_LENGTH;
		let depositor: T::AccountId = account("depositor", 0, SEED);
		let parameters = vec![0u8;c as usize];

		Deposit::<T, I>::put::<Option<DepositDetails<T::AccountId, DepositBalanceOf<T, I>>>>(Some(DepositDetails{
			depositor,
			deposit:1_000u32.into()
		}));


	}: _(RawOrigin::Root, parameters.clone())
	verify {
		assert_last_event::<T, I>(Event::ParametersSet{who: Default::default(), parameters}.into());
	}
}

impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Test);
