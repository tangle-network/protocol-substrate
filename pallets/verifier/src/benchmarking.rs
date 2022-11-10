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
use crate::Pallet;
use frame_benchmarking::{
	account, benchmarks_instance_pallet, impl_benchmark_test_suite, whitelist_account,
	whitelisted_caller,
};
use frame_support::traits::Currency;
use frame_system::RawOrigin;
use sp_runtime::traits::Bounded;
use webb_primitives::types::DepositDetails;

fn assert_last_event<T: Config<I>, I: 'static>(generic_event: <T as Config<I>>::RuntimeEvent) {
	frame_system::Pallet::<T>::assert_last_event(generic_event.into());
}

const SEED: u32 = 0;
// Based on verifier bytes generated from the zero knowledge setup for anchor
// pallet and mixer pallet, Max verifier bytes length generated ranged between
// 456 - 552
const MAX_VERIFIER_LENGTH: u32 = 1024;

benchmarks_instance_pallet! {
	force_set_parameters {
		let c in 0..MAX_VERIFIER_LENGTH;
		let depositor: T::AccountId = account("depositor", 0, SEED);
		let parameters = vec![0u8;c as usize];
	}: _(RawOrigin::Root, parameters.clone())
	verify {
		assert_eq!(Pallet::<T, I>::parameters(), parameters);
	}
}

impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Test);
