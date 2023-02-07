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

use arkworks_setups::{common::setup_params, Curve};
use frame_benchmarking::{
	benchmarks_instance_pallet, impl_benchmark_test_suite, whitelisted_caller,
};
use frame_support::traits::Currency;
use frame_system::RawOrigin;
use sp_runtime::traits::Bounded;
use sp_std::vec;
use webb_primitives::traits::merkle_tree::TreeInterface;

type BalanceOf<T, I> =
	<<T as Config<I>>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

fn assert_last_event<T: Config<I>, I: 'static>(generic_event: <T as Config<I>>::RuntimeEvent) {
	frame_system::Pallet::<T>::assert_last_event(generic_event.into());
}

pub fn hasher_params() -> Vec<u8> {
	let curve = Curve::Bn254;
	let params = setup_params::<ark_bn254::Fr>(curve, 5, 3);
	params.to_bytes()
}

benchmarks_instance_pallet! {
	where_clause {  where T: pallet_hasher::Config<I> }

	create {
		let d in 1..<T as pallet::Config<I>>::MaxTreeDepth::get() as u32;
		pallet_hasher::Pallet::<T, I>::force_set_parameters(RawOrigin::Root.into(), hasher_params().try_into().unwrap()).unwrap();
		let caller: T::AccountId = whitelisted_caller();
		<<T as pallet::Config<I>>::Currency as Currency<T::AccountId>>::make_free_balance_be(&caller, BalanceOf::<T, I>::max_value());
		let tree_id = Pallet::<T, I>::next_tree_id();

	}:_(RawOrigin::Signed(caller.clone()), d as u8)
	verify {
		assert_last_event::<T, I>(Event::TreeCreation{tree_id: tree_id, who: caller}.into())
	}

	insert {
		let caller: T::AccountId = whitelisted_caller();
		pallet_hasher::Pallet::<T, I>::force_set_parameters(RawOrigin::Root.into(), hasher_params().try_into().unwrap()).unwrap();
		let tree_id: T::TreeId = <Pallet<T, I> as TreeInterface<_,_,_>>::create(Some(caller.clone()), T::MaxTreeDepth::get()).unwrap();
		let leaf_index = Pallet::<T, I>::next_leaf_index(tree_id);
		let element: T::Element = T::DefaultZeroElement::get();

	}:_(RawOrigin::Signed(caller.clone()), tree_id, element)
	verify {
		assert_last_event::<T, I>(Event::LeafInsertion{tree_id, leaf_index, leaf: element}.into())
	}

	force_set_default_hashes {
		let p in 1..<T as pallet::Config<I>>::MaxTreeDepth::get() as u32;

		let default_hashes = vec![<T as pallet::Config<I>>::DefaultZeroElement::get();p as usize];

	}:_(RawOrigin::Root, default_hashes)
	verify {
		assert_eq!(DefaultHashes::<T, I>::get().len(), p as usize)
	}

}

impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Test);
