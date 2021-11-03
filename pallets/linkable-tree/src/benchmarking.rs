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

use darkwebb_primitives::{anchor::AnchorInterface, traits::merkle_tree::TreeInspector};
use frame_benchmarking::{account, benchmarks, impl_benchmark_test_suite, whitelist_account, whitelisted_caller};
use frame_system::RawOrigin;
use orml_traits::MultiCurrency;
// Run the zk-setup binary before compiling the with runtime-benchmarks to
// generate the zk_config.rs file if it doesn't exist The accounts used in
// generating the proofs have to be the same accounts used in the withdraw
// benchmark
use zk_config::*;

use crate::Pallet as Anchor;
use frame_support::{
	storage,
	traits::{Currency, Get, OnInitialize, PalletInfo},
};

fn assert_last_event<T: Config>(generic_event: <T as Config>::Event) {
	frame_system::Pallet::<T>::assert_last_event(generic_event.into());
}

const SEED: u32 = 0;
const MAX_EDGES: u32 = 256;

benchmarks! {

	create {
	  let i in 1..MAX_EDGES;
	  let d in 1..<T as pallet_mt::Config>::MaxTreeDepth::get() as u32;

	  let deposit_size: u32 = 1_000_000_000;
	  let asset_id = <<T as pallet_mixer::Config>::NativeCurrencyId as Get<pallet_mixer::CurrencyIdOf<T, _>>>::get();
	}: _(RawOrigin::Root, deposit_size.into(), i, d as u8, asset_id)

	set_maintainer {
		let caller: T::AccountId = whitelisted_caller();
		let new_maintainer: T::AccountId = account("maintainer", 0, SEED);
		Maintainer::<T>::put::<T::AccountId>(caller.clone());
	}: _(RawOrigin::Signed(caller.clone()), new_maintainer.clone())
	verify {
		assert_last_event::<T>(Event::MaintainerSet{old_maintainer: caller, new_maintainer: new_maintainer.into()}.into());
	}

	force_set_maintainer {
		let new_maintainer: T::AccountId = account("maintainer", 0, SEED);
	}: _(RawOrigin::Root, new_maintainer.clone())
	verify {
		assert_last_event::<T>(Event::MaintainerSet{old_maintainer: Default::default(), new_maintainer: new_maintainer.into()}.into());
	}

}

impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Test);
