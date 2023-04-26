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

//! Mixer pallet benchmarking.

#![cfg(feature = "runtime-benchmarks")]

use super::*;

use frame_benchmarking::{
	account, benchmarks_instance_pallet, whitelist_account, whitelisted_caller,
};
use frame_system::RawOrigin;
use orml_traits::MultiCurrency;
use webb_primitives::{mixer::MixerInterface, traits::merkle_tree::TreeInspector, ElementTrait};
// Run the zk-setup binary before compiling the with runtime-benchmarks to
// generate the zk_config.rs file if it doesn't exist The accounts used in
// generating the proofs have to be the same accounts used in the withdraw
// benchmark
use zk_config::*;

use crate::Pallet as Mixer;
use frame_support::{
	storage,
	traits::{Currency, Get, OnInitialize, PalletInfo},
};

fn assert_last_event<T: Config>(generic_event: <T as Config>::RuntimeEvent) {
	frame_system::Pallet::<T>::assert_last_event(generic_event.into());
}

const SEED: u32 = 0;

benchmarks_instance_pallet! {

	create {
	  let d in 1..<T as pallet_mt::Config<I>>::MaxTreeDepth::get() as u32;

	  let deposit_size: u32 = 1_000_000_000;
	  let asset_id = <<T as Config<I>>::NativeCurrencyId as Get<CurrencyIdOf<T, _>>>::get();
	}: _(RawOrigin::Root, deposit_size.into(), d as u8, asset_id)

	deposit {
	  let caller: T::AccountId = whitelisted_caller();
	  let deposit_size: u32 = 50_000_000;
	  let asset_id = <<T as Config<I>>::NativeCurrencyId as Get<CurrencyIdOf<T, _>>>::get();
	  let depth = <T as pallet_mt::Config<I>>::MaxTreeDepth::get();

	  let tree_id = <Mixer<T, I> as MixerInterface<_,_,_,_,_>>::create(None, deposit_size.into(), depth, asset_id)?;
	  let leaf = <T as pallet_mt::Config<I>>::Element::from_bytes(&[1u8; 32]);
	  <<T as pallet_mt::Config<I>>::Currency as Currency<T::AccountId>>::make_free_balance_be(&caller.clone(), 900_000_000u32.into());
	}: _(RawOrigin::Signed(caller.clone()), tree_id, leaf)
	verify {
	  assert_eq!(<<T as Config<I>>::Currency as MultiCurrency<T::AccountId>>::total_balance(asset_id, &Pallet::<T, I>::account_id()), deposit_size.into())
	}

	withdraw {

		let hasher_pallet_name = <T as frame_system::Config>::PalletInfo::name::<<T as pallet_mt::Config<I>>::Hasher>().unwrap();
		let verifier_pallet_name = <T as frame_system::Config>::PalletInfo::name::<<T as Config<I>>::Verifier>().unwrap();

		// 1. Setup The Hasher Pallet.
		storage::unhashed::put(&storage::storage_prefix(hasher_pallet_name.as_bytes(), "Parameters".as_bytes()),&HASH_PARAMS[..]);

		// 2. Initialize MerkleTree pallet
		<pallet_mt::Pallet<T, I> as OnInitialize<_>>::on_initialize(Default::default());


		storage::unhashed::put(&storage::storage_prefix(verifier_pallet_name.as_bytes(), "Parameters".as_bytes()),&VK_BYTES[..]);

		// inputs
		let caller: T::AccountId = whitelisted_caller();
		<<T as pallet_mt::Config<I>>::Currency as Currency<T::AccountId>>::make_free_balance_be(&caller.clone(), 200_000_000u32.into());
		let src_chain_id: u32 = 1;
		let recipient_account_id: T::AccountId = account("recipient", 0, SEED);
		let relayer_account_id: T::AccountId = account("relayer", 1, SEED);
		whitelist_account!(recipient_account_id);
		whitelist_account!(relayer_account_id);
		<<T as pallet_mt::Config<I>>::Currency as Currency<T::AccountId>>::make_free_balance_be(&recipient_account_id.clone(), 100_000_000u32.into());
		let fee_value: u32 = 0;
		let refund_value: u32 = 0;

		let deposit_size: u32 = 50_000_000;
		let depth = <T as pallet_mt::Config<I>>::MaxTreeDepth::get();
		let asset_id = <<T as Config<I>>::NativeCurrencyId as Get<CurrencyIdOf<T, _>>>::get();

		let tree_id = <Mixer<T, I> as MixerInterface<_,_,_,_,_>>::create(None, deposit_size.into(), depth, asset_id)?;

		<Mixer<T, I> as MixerInterface<_,_,_,_,_>>::deposit(
			caller.clone(),
			tree_id,
			<T as pallet_mt::Config<I>>::Element::from_bytes(&LEAF[..]),
		)?;

		let tree_root = <pallet_mt::Pallet<T, I> as TreeInspector<T::AccountId, <T as pallet_mt::Config<I>>::TreeId, <T as pallet_mt::Config<I>>::Element>>::get_root(tree_id).unwrap();
		// sanity check.

		assert_eq!(<T as pallet_mt::Config<I>>::Element::from_bytes(&ROOT_ELEMENT_BYTES[0]), tree_root);

		let roots_element: Vec<<T as pallet_mt::Config<I>>::Element> = ROOT_ELEMENT_BYTES
			.iter()
			.map(|v| <T as pallet_mt::Config<I>>::Element::from_bytes(&v[..]))
			.collect();
		let root = roots_element[0];


		let nullifier_hash_element = <T as pallet_mt::Config<I>>::Element::from_bytes(&NULLIFIER_HASH_ELEMENTS_BYTES[..]);
	}: _(
		RawOrigin::Signed(caller),
		tree_id,
		PROOF_BYTES.to_vec(),
		root,
		nullifier_hash_element,
		recipient_account_id.clone(),
		relayer_account_id,
		fee_value.into(),
		refund_value.into()
	)
	verify {
		assert_eq!(<<T as Config<I>>::Currency as MultiCurrency<T::AccountId>>::total_balance(asset_id, &recipient_account_id), (100_000_000u32 + deposit_size).into())
	}

	impl_benchmark_test_suite!(Mixer, crate::mock::new_bench_ext(), crate::mock::Test);

}
