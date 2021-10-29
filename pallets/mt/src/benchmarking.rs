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

//! Merkle Tree pallet benchmarking.

#![cfg(feature = "runtime-benchmarks")]

use super::*;

use darkwebb_primitives::traits::merkle_tree::TreeInterface;
use frame_benchmarking::{account, benchmarks, impl_benchmark_test_suite, whitelisted_caller};
use frame_support::traits::Currency;
use frame_system::RawOrigin;
use sp_runtime::traits::Bounded;
use sp_std::vec;
type BalanceOf<T> = <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

fn assert_last_event<T: Config>(generic_event: <T as Config>::Event) {
	frame_system::Pallet::<T>::assert_last_event(generic_event.into());
}

const SEED: u32 = 0;

benchmarks! {

	create {
		let d in 1..T::MaxTreeDepth::get() as u32;

		let caller: T::AccountId = whitelisted_caller();

		<<T as Config>::Currency as Currency<T::AccountId>>::make_free_balance_be(&caller, BalanceOf::<T>::max_value());
		let tree_id = Pallet::<T>::next_tree_id();

	}:_(RawOrigin::Signed(caller.clone()), d as u8)
	verify {
		assert_last_event::<T>(Event::TreeCreation{tree_id: tree_id, who: caller}.into())
	}

	insert {
		let caller: T::AccountId = whitelisted_caller();
		let tree_id: T::TreeId = <Pallet<T> as TreeInterface<_,_,_>>::create(caller.clone(), T::MaxTreeDepth::get()).unwrap();

		let leaf_index = Pallet::<T>::next_leaf_index(tree_id);

		let element: T::Element = T::DefaultZeroElement::get();

	}:_(RawOrigin::Signed(caller.clone()), tree_id, element)
	verify {
		assert_last_event::<T>(Event::LeafInsertion{tree_id, leaf_index, leaf: element}.into())
	}

	set_maintainer {
		let caller: T::AccountId = whitelisted_caller();
		let new_maintainer: T::AccountId = account("maintainer", 0, SEED);
		Maintainer::<T>::put::<T::AccountId>(caller.clone());
	}:_(RawOrigin::Signed(caller.clone()), new_maintainer.clone())
	verify {
		assert_last_event::<T>(Event::MaintainerSet{old_maintainer: caller, new_maintainer: new_maintainer.into()}.into());
	}

	force_set_maintainer {
		let new_maintainer: T::AccountId = account("maintainer", 0, SEED);
	}:_(RawOrigin::Root, new_maintainer.clone())
	verify {
		assert_last_event::<T>(Event::MaintainerSet{old_maintainer: Default::default(), new_maintainer: new_maintainer.into()}.into());
	}

	force_set_default_hashes {
		let p in 1..T::MaxTreeDepth::get() as u32;

		let default_hashes = vec![T::DefaultZeroElement::get();p as usize];

	}:_(RawOrigin::Root, default_hashes)
	verify {
		assert_eq!(DefaultHashes::<T>::get().len(), p as usize)
	}

}

impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Test);
