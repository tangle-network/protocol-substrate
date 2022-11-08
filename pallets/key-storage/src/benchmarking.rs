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

#![cfg(feature = "runtime-benchmarks")]
use super::*;
use crate::Pallet;
use frame_benchmarking::{
	benchmarks_instance_pallet, impl_benchmark_test_suite, whitelisted_caller,
};
use frame_system::RawOrigin;
use sp_std::vec;
fn assert_last_event<T: Config<I>, I: 'static>(generic_event: <T as Config<I>>::Event) {
	frame_system::Pallet::<T>::assert_last_event(generic_event.into());
}

const MAX_PARAMETER_LENGTH: u32 = 20000;

benchmarks_instance_pallet! {
	register {
		let c in 0..MAX_PARAMETER_LENGTH;
		let owner: T::AccountId = whitelisted_caller();
		let public_key = vec![0u8;c as usize];
	}: _(RawOrigin::Signed(owner.clone()), owner.clone(), public_key.clone())
	verify {
		assert_last_event::<T, I>(Event::PublicKeyRegistration{owner, public_key}.into());
	}
}

impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Test);
