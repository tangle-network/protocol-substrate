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

use frame_benchmarking::{
	account, benchmarks_instance_pallet, impl_benchmark_test_suite, whitelist_account,
	whitelisted_caller,
};
use frame_system::RawOrigin;
use orml_traits::MultiCurrency;

use ark_ff::{BigInteger, PrimeField};
use pallet_linkable_tree::LinkableTreeConfigration;
use webb_primitives::{
	traits::{merkle_tree::TreeInspector, vanchor::VAnchorInterface},
	utils::compute_chain_id_type,
	AccountId, Amount, AssetId, Balance, Element, ElementTrait,
};

use crate::{benchmarking_utils::*, Pallet as VAnchor};
use arkworks_setups::{common::setup_params, Curve};
use frame_support::{
	assert_ok, storage,
	traits::{Currency, Get, OnInitialize, PalletInfo},
};
use sp_io::hashing::keccak_256;

fn assert_last_event<T: Config<I>, I: 'static>(generic_event: <T as Config<I>>::RuntimeEvent) {
	frame_system::Pallet::<T>::assert_last_event(generic_event.into());
}

const CHAIN_IDENTIFIER: u32 = 1080;

fn setup_env<T: Config<I>, I: 'static>() -> Vec<u8>
where
	T: pallet_vanchor_verifier::Config<I>,
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

	let pk_2_2_bytes = include_bytes!(
		"../../../substrate-fixtures/vanchor/bn254/x5/2-2-2/proving_key_uncompressed.bin"
	)
	.to_vec();
	let vk_2_2_bytes =
		include_bytes!("../../../substrate-fixtures/vanchor/bn254/x5/2-2-2/verifying_key.bin")
			.to_vec();

	let pk_2_16_bytes = include_bytes!(
		"../../../substrate-fixtures/vanchor/bn254/x5/2-16-2/proving_key_uncompressed.bin"
	)
	.to_vec();
	let vk_2_16_bytes =
		include_bytes!("../../../substrate-fixtures/vanchor/bn254/x5/2-16-2/verifying_key.bin")
			.to_vec();

	assert_ok!(<pallet_vanchor_verifier::Pallet<T, I>>::force_set_parameters(
		RawOrigin::Root.into(),
		(2, 2),
		vk_2_2_bytes.clone().try_into().unwrap()
	));
	assert_ok!(<pallet_vanchor_verifier::Pallet<T, I>>::force_set_parameters(
		RawOrigin::Root.into(),
		(2, 16),
		vk_2_16_bytes.clone().try_into().unwrap()
	));

	pk_2_2_bytes
}

pub fn hasher_params() -> Vec<u8> {
	let curve = Curve::Bn254;
	let params = setup_params::<ark_bn254::Fr>(curve, 5, 3);
	params.to_bytes()
}

const SEED: u32 = 0;
const MAX_EDGES: u32 = 256;

benchmarks_instance_pallet! {

	where_clause {  where T: pallet_hasher::Config<I>,
		T: pallet_linkable_tree::Config<I>,
		T: pallet_vanchor_verifier::Config<I>,
		T: pallet_mt::Config<I, Element = Element>,
		T: orml_tokens::Config<Amount = Amount>,
		<T as frame_system::Config>::AccountId: From<AccountId>,
		<<T as pallet::Config<I>>::Currency as MultiCurrency<<T as frame_system::Config>::AccountId>>::CurrencyId: From<AssetId>,
		pallet_linkable_tree::Pallet<T, I>: webb_primitives::linkable_tree::LinkableTreeInspector<LinkableTreeConfigration<T, I>> }

	create {
	  let i in 1..MAX_EDGES;
	  let d in 1..<T as pallet_mt::Config<I>>::MaxTreeDepth::get() as u32;

	  pallet_hasher::Pallet::<T, I>::force_set_parameters(RawOrigin::Root.into(), hasher_params().try_into().unwrap()).unwrap();

	  let asset_id = <<T as crate::Config<I>>::NativeCurrencyId as Get<crate::CurrencyIdOf<T, I>>>::get();
	}: _(RawOrigin::Root, i, d as u8, asset_id)
	verify {
		assert_last_event::<T, I>(Event::VAnchorCreation{ tree_id : 0_u32.into() }.into())
	}

	transact {

		let pk_2_2_bytes =  setup_env::<T,I>();

		let deposit_size: u32 = 50_000_000;
		let asset_id = <<T as crate::Config<I>>::NativeCurrencyId as Get<crate::CurrencyIdOf<T, I>>>::get();
		  let depth = <T as pallet_mt::Config<I>>::MaxTreeDepth::get();

		let tree_id = <VAnchor<T, I> as VAnchorInterface<VAnchorConfigration<T, I>>>::create(None, depth, 1u32, asset_id, 1u32.into())?;

		<VAnchor<T, I> as VAnchorInterface<VAnchorConfigration<T, I>>>::set_max_deposit_amount(100u32.into(), 2u32.into())?;

		let transactor : T::AccountId = account("", 0, SEED);
		let recipient : T::AccountId = account("", 1, SEED);
		let relayer: T::AccountId = account("", 4, SEED);
		let ext_amount: u32 = 10;
		let fee: u32 = 0;

		<<T as pallet_mt::Config<I>>::Currency as Currency<T::AccountId>>::make_free_balance_be(&transactor.clone(), 100_000_000u32.into());

		let public_amount : i128 = 10;

		let chain_type = [2, 0];
		let chain_id = compute_chain_id_type(CHAIN_IDENTIFIER, chain_type);
		let in_chain_ids = [chain_id; 2];
		let in_amounts = [0, 0];
		let in_indices = [0, 1];
		let out_chain_ids = [chain_id; 2];
		let out_amounts = [10, 0];

		let in_utxos = setup_utxos(in_chain_ids, in_amounts, Some(in_indices));
		// We are adding indicies to out utxos, since they will be used as an input utxos in next
		// transaction
		let out_utxos = setup_utxos(out_chain_ids, out_amounts, Some(in_indices));

		let output1 = out_utxos[0].commitment.into_repr().to_bytes_be();
		let output2 = out_utxos[1].commitment.into_repr().to_bytes_be();
		let ext_data = ExtData::<T::AccountId, AmountOf<T, I>, BalanceOf<T, I>, CurrencyIdOf<T, I>>::new(
			recipient.into(),
			relayer.into(),
			ext_amount.into(),
			fee.into(),
			0u32.into(),
			(AssetId::MAX - 1).into(),
			output1.to_vec(), // Mock encryption value, not meant to be used in production
			output2.to_vec(), // Mock encryption value, not meant to be used in production
		);

		let ext_data_hash = keccak_256(&ext_data.encode_abi());

		let custom_root = <pallet_mt::Pallet<T, I>>::get_default_root(tree_id).unwrap();
		let neighbor_roots: [Element; 1] = <pallet_linkable_tree::Pallet<T, I> as LinkableTreeInspector<
			LinkableTreeConfigration<T, I>,
		>>::get_neighbor_roots(tree_id).unwrap().try_into().unwrap();

		let (proof, public_inputs) = setup_zk_circuit(
			public_amount,
			chain_id,
			ext_data_hash.to_vec(),
			in_utxos,
			out_utxos.clone(),
			pk_2_2_bytes,
			neighbor_roots,
			custom_root,
		);

		// Deconstructing public inputs
		let (_chain_id, public_amount, root_set, nullifiers, commitments, ext_data_hash) =
			deconstruct_public_inputs_el(&public_inputs);

		// Constructing proof data
		let proof_data =
			ProofData::new(proof, public_amount, root_set, nullifiers, commitments, ext_data_hash);

	  }: _(RawOrigin::Signed(transactor.clone()), tree_id, proof_data.clone(), ext_data)
	  verify {
		  assert_last_event::<T, I>(
			Event::Transaction {
			transactor,
			tree_id,
			leafs : proof_data.output_commitments,
			encrypted_output1: output1.to_vec(),
			encrypted_output2: output2.to_vec(),
			amount : ext_amount.into() }.into()
		)
	}

	register_and_transact {

		let pk_2_2_bytes =  setup_env::<T,I>();


		let deposit_size: u32 = 50_000_000;
		let asset_id = <<T as crate::Config<I>>::NativeCurrencyId as Get<crate::CurrencyIdOf<T, I>>>::get();
		  let depth = <T as pallet_mt::Config<I>>::MaxTreeDepth::get();

		let tree_id = <VAnchor<T, I> as VAnchorInterface<VAnchorConfigration<T, I>>>::create(None, depth, 1u32, asset_id, 1u32.into())?;

		<VAnchor<T, I> as VAnchorInterface<VAnchorConfigration<T, I>>>::set_max_deposit_amount(100u32.into(), 2u32.into())?;

		let transactor : T::AccountId = account("", 0, SEED);
		let recipient : T::AccountId = account("", 1, SEED);
		let relayer: T::AccountId = account("", 4, SEED);
		let ext_amount: u32 = 10;
		let fee: u32 = 0;

		<<T as pallet_mt::Config<I>>::Currency as Currency<T::AccountId>>::make_free_balance_be(&transactor.clone(), 100_000_000u32.into());

		let public_amount : i128 = 10;

		let chain_type = [2, 0];
		let chain_id = compute_chain_id_type(CHAIN_IDENTIFIER, chain_type);
		let in_chain_ids = [chain_id; 2];
		let in_amounts = [0, 0];
		let in_indices = [0, 1];
		let out_chain_ids = [chain_id; 2];
		let out_amounts = [10, 0];

		let in_utxos = setup_utxos(in_chain_ids, in_amounts, Some(in_indices));
		// We are adding indicies to out utxos, since they will be used as an input utxos in next
		// transaction
		let out_utxos = setup_utxos(out_chain_ids, out_amounts, Some(in_indices));

		let output1 = out_utxos[0].commitment.into_repr().to_bytes_be();
		let output2 = out_utxos[1].commitment.into_repr().to_bytes_be();
		let ext_data = ExtData::<T::AccountId, AmountOf<T, I>, BalanceOf<T, I>, CurrencyIdOf<T, I>>::new(
			recipient.into(),
			relayer.into(),
			ext_amount.into(),
			fee.into(),
			0u32.into(),
			(AssetId::MAX - 1).into(),
			output1.to_vec(), // Mock encryption value, not meant to be used in production
			output2.to_vec(), // Mock encryption value, not meant to be used in production
		);

		let ext_data_hash = keccak_256(&ext_data.encode_abi());

		let custom_root = <pallet_mt::Pallet<T, I>>::get_default_root(tree_id).unwrap();
		let neighbor_roots: [Element; 1] = <pallet_linkable_tree::Pallet<T, I> as LinkableTreeInspector<
			LinkableTreeConfigration<T, I>,
		>>::get_neighbor_roots(tree_id).unwrap().try_into().unwrap();

		let (proof, public_inputs) = setup_zk_circuit(
			public_amount,
			chain_id,
			ext_data_hash.to_vec(),
			in_utxos,
			out_utxos.clone(),
			pk_2_2_bytes,
			neighbor_roots,
			custom_root,
		);

		// Deconstructing public inputs
		let (_chain_id, public_amount, root_set, nullifiers, commitments, ext_data_hash) =
			deconstruct_public_inputs_el(&public_inputs);

		// Constructing proof data
		let proof_data =
			ProofData::new(proof, public_amount, root_set, nullifiers, commitments, ext_data_hash);

	  }: _(RawOrigin::Signed(transactor.clone()), transactor.clone(),[0u8; 32].to_vec(),tree_id, proof_data.clone(), ext_data)
	  verify {
		assert_last_event::<T, I>(
		  Event::Transaction {
		  transactor,
		  tree_id,
		  leafs : proof_data.output_commitments,
		  encrypted_output1: output1.to_vec(),
		  encrypted_output2: output2.to_vec(),
		  amount : ext_amount.into() }.into()
	  )
  }

	set_max_deposit_amount {
	  }: _(RawOrigin::Root, 100u32.into(), 101u32.into())
	  verify {
		  assert_last_event::<T, I>(Event::MaxDepositAmountChanged{ max_deposit_amount : 100_u32.into() }.into())
	}

	set_min_withdraw_amount {
	}: _(RawOrigin::Root, 1u32.into(), 101u32.into())
	verify {
		assert_last_event::<T, I>(Event::MinWithdrawAmountChanged{ min_withdraw_amount : 1_u32.into() }.into())
  }


}

impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Test);
