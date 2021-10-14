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

//! Anchor pallet benchmarking.

#![cfg(feature = "runtime-benchmarks")]

use super::*;

use darkwebb_primitives::anchor::AnchorInterface;
use frame_benchmarking::{account, benchmarks, impl_benchmark_test_suite, whitelist_account, whitelisted_caller};
use orml_traits::MultiCurrency;
use frame_system::RawOrigin;

fn assert_last_event<T: Config>(generic_event: <T as Config>::Event) {
	frame_system::Pallet::<T>::assert_last_event(generic_event.into());
}

const SEED: u32 = 0;
const MAX_EDGES: u32 = 256;

benchmarks! {

    create {
      let i in 1..MAX_EDGES;
      let d in 1..<crate::mock::Test as pallet_mt::Config>::MaxTreeDepth::get() as u32;

      let deposit_size = BalanceOf<T>::max_value();
    }: _(RawOrigin::Root, deposit_size, i, d as u8, 1)

    deposit {
      let caller = whitelisted_caller();
      let deposit_size: u32 = 1_000_000;
      let tree_id = <Pallet as AnchorInterface<crate::AnchorConfigration<T>>>::create(caller.clone(), deposit_size.into(), 32, 256,  1.into());


    }: _(RawOrigin::Signed(caller.clone(), tree_id, <crate::mock::Test as pallet_mt::Config>::DefaultZeroElement::get())) 
    verify {
      assert_eq!(<<T as pallet_mixer::Config>::Currency as MultiCurrency<T::AccountId>>::total_balance(1.into(), T::Mixer::account_id()), deposit_size.into())
    }

    set_maintainer {
		let caller: T::AccountId = whitelisted_caller();
		let new_maintainer: T::AccountId = account("maintainer", 0, SEED);
		Maintainer::<T>::put::<T::AccountId>(caller.clone());
	}: _(RawOrigin::Signed(caller.clone()), new_maintainer.clone())
	verify {
		assert_last_event::<T>(Event::MaintainerSet(caller, new_maintainer.into()).into());
	}

    force_set_maintainer {
		let new_maintainer: T::AccountId = account("maintainer", 0, SEED);
	}: _(RawOrigin::Root, new_maintainer.clone())
	verify {
		assert_last_event::<T>(Event::MaintainerSet(Default::default(), new_maintainer.into()).into());
	}
	
    withdraw {

    }: _()
    verify {
        
    }
	
}

impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Test);


