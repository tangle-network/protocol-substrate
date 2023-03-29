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

//! VAnchor pallet benchmarking.

#![cfg(feature = "runtime-benchmarks")]

use super::*;

use frame_benchmarking::{account, benchmarks_instance_pallet, impl_benchmark_test_suite};
use frame_system::RawOrigin;
use orml_traits::MultiCurrency;

// use ark_ff::{BigInteger, PrimeField};
use codec::Decode;
use pallet_linkable_tree::LinkableTreeConfigration;
use webb_primitives::{
	webb_proposals::{ResourceId, SubstrateTargetSystem, TargetSystem, TypedChainId},
	AccountId, Amount, AssetId, Element,
};

// use crate::{benchmarking_utils::*, Pallet as AnonimityMiningClaims};
// use crate::Pallet as AnonimityMiningClaims;
use arkworks_setups::{common::setup_params, Curve};
use frame_support::{
	assert_ok,
	traits::{Currency, Get, OnInitialize},
};
// use sp_io::hashing::keccak_256;

fn assert_last_event<T: Config<I>, I: 'static>(generic_event: <T as Config<I>>::RuntimeEvent) {
	frame_system::Pallet::<T>::assert_last_event(generic_event.into());
}

// const CHAIN_IDENTIFIER: u32 = 1080;

fn setup_env<T: Config<I>, I: 'static>() -> RewardProofData<T::Element>
where
	T: pallet_claims_verifier::Config<I>,
	T: pallet_hasher::Config<I>,
{
	// Initialize hasher pallet
	pallet_hasher::Pallet::<T, I>::force_set_parameters(
		RawOrigin::Root.into(),
		hasher_params().try_into().unwrap(),
	)
	.unwrap();

	// 2. Initialize MerkleTree pallet.
	<pallet_mt::Pallet<T, I> as OnInitialize<_>>::on_initialize(Default::default());
	// 3. Setup the VerifierPallet
	//    but to do so, we need to have a VerifyingKey

	let vk_2_2_bytes = include_bytes!("../circom_vk_2_2_bytes").to_vec();

	assert_ok!(<pallet_claims_verifier::Pallet<T, I>>::force_set_parameters(
		RawOrigin::Root.into(),
		2,
		vk_2_2_bytes.clone().try_into().unwrap()
	));

	let proof_data_raw = include_bytes!("../proof_data");
	let proof_data = RewardProofData::decode(&mut proof_data_raw.as_slice()).unwrap();
	// proof_data_raw.decode();
	// let proof_data: RewardProofData<T::Element> = proof_data_raw.decode();
	proof_data
}

pub fn hasher_params() -> Vec<u8> {
	let curve = Curve::Bn254;
	let params = setup_params::<ark_bn254::Fr>(curve, 5, 3);
	params.to_bytes()
}

const SEED: u32 = 0;
// const MAX_EDGES: u32 = 256;

benchmarks_instance_pallet! {

	where_clause {  where T: pallet_hasher::Config<I>,
		T: pallet_linkable_tree::Config<I>,
		T: pallet_claims_verifier::Config<I>,
		T: pallet_mt::Config<I, Element = Element>,
		T: orml_tokens::Config<Amount = Amount>,
		<T as frame_system::Config>::AccountId: From<AccountId>,
		<<T as pallet::Config<I>>::Currency as MultiCurrency<<T as frame_system::Config>::AccountId>>::CurrencyId: From<AssetId>,
		pallet_linkable_tree::Pallet<T, I>: webb_primitives::linkable_tree::LinkableTreeInspector<LinkableTreeConfigration<T, I>>,
		<T as frame_system::Config>::AccountId: From<AccountId>,
	}

	create {
		let i: u8 = 2;
		let d: u32 = 30;
		let transactor : T::AccountId = account("", 0, SEED);
		<<T as pallet_mt::Config<I>>::Currency as Currency<T::AccountId>>::make_free_balance_be(&transactor.clone(), 100_000_000u32.into());

		pallet_hasher::Pallet::<T, I>::force_set_parameters(RawOrigin::Root.into(), hasher_params().try_into().unwrap()).unwrap();

		let asset_id = <<T as crate::Config<I>>::NativeCurrencyId as Get<crate::CurrencyIdOf<T, I>>>::get();
	}: _(RawOrigin::Root, i, d as u8, asset_id)
	verify {
		assert_last_event::<T, I>(Event::APVanchorCreated{ tree_id : 0_u32.into()}.into())
	}
	claim {
		let transactor : T::AccountId = account("", 0, SEED);
		let reward_proof_data: RewardProofData<Element> =  setup_env::<T,I>();

		let src_id = TypedChainId::Substrate(1);
		let target_id = TypedChainId::Substrate(5);
		let target_system =
			TargetSystem::Substrate(SubstrateTargetSystem { pallet_index: 11, tree_id: 0 });

		let src_target_system = target_system;
		let src_resource_id = ResourceId::new(src_target_system, src_id);

		let dest_target_system = target_system;
		let dest_resource_id = ResourceId::new(dest_target_system, target_id);

		<<T as pallet_mt::Config<I>>::Currency as Currency<T::AccountId>>::make_free_balance_be(&transactor.clone(), 100_000_000u32.into());

		let asset_id = <<T as crate::Config<I>>::NativeCurrencyId as Get<crate::CurrencyIdOf<T, I>>>::get();
		let call = AnonymityMiningClaims::create(RuntimeOrigin::root(), depth, max_edges, asset_id)?;

		let init_call_0 = AnonymityMiningClaims::_init_resource_id_history(
			src_resource_id,
			Element::from_bytes(&circuit_inputs.unspent_roots[0].to_bytes_be().1),
			Element::from_bytes(&circuit_inputs.spent_roots[0].to_bytes_be().1),
		)?;
		assert_ok!(init_call_0);
		let init_call_1 = AnonymityMiningClaims::_init_resource_id_history(
			dest_resource_id,
			Element::from_bytes(&circuit_inputs.unspent_roots[1].to_bytes_be().1),
			Element::from_bytes(&circuit_inputs.spent_roots[1].to_bytes_be().1),
		);
		assert_ok!(init_call_1);

	}: _(RawOrigin::Signed(transactor.clone()), reward_proof_data)
	verify {
		assert_last_event::<T, I>(Event::RewardClaimed{ reward_proof_data,  })
	}
	init_resource_id_history {
		let chain_id = TypedChainId::Substrate(1);
		let target_system =
			TargetSystem::Substrate(SubstrateTargetSystem { pallet_index: 11, tree_id: 0 });

		let resource_id = ResourceId::new(target_system, chain_id);

		let transactor : T::AccountId = account("", 0, SEED);
		let reward_proof_data: RewardProofData<Element> =  setup_env::<T,I>();

		<<T as pallet_mt::Config<I>>::Currency as Currency<T::AccountId>>::make_free_balance_be(&transactor.clone(), 100_000_000u32.into());

	}: _(RawOrigin::Signed(transactor.clone()), resource_id, reward_proof_data.unspent_roots[0], reward_proof_data.spent_roots[0])
	// verify {
	// 	assert_last_event::<T, I>(Event::VAnchorCreation{ tree_id : 0_u32.into() }.into())
	// }
	// update_unspent_root {
	// 	let transactor : T::AccountId = account("", 0, SEED);
	// 	let reward_proof_data: RewardProofData<Element> =  setup_env::<T,I>();
	//
	// 	<<T as pallet_mt::Config<I>>::Currency as Currency<T::AccountId>>::make_free_balance_be(&transactor.clone(), 100_000_000u32.into());
	//
	// }: _(RawOrigin::Signed(transactor.clone()), reward_proof_data)
}

impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Test);
