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
#![allow(unused_imports)]

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
	// traits::merkle_tree::TreeInspector,
	// utils::compute_chain_id_type,
	AccountId, Amount, AssetId, Element,
};

use crate::{benchmarking_utils::*, Pallet as AnonimityMiningClaims};
use ark_circom::{read_zkey, WitnessCalculator};
use circom_proving::circom_from_folder;
use arkworks_setups::{common::setup_params, Curve};
use frame_support::{
	assert_ok,
	traits::OnInitialize
};
use std::{
	fs::File,
	sync::Mutex,
};
use ark_groth16::ProvingKey;

// use sp_std::convert::TryInto;
use ark_serialize::CanonicalSerialize;
// use sp_io::hashing::keccak_256;
use ark_std::vec::Vec;
// use frame_benchmarking::vec;
use ark_bn254::{Bn254, Fr};
use ark_relations::r1cs::ConstraintMatrices;
// use ark_ff::{BigInteger, PrimeField};
// use ark_ff::fields::PrimeField;

#[allow(dead_code)]
fn assert_last_event<T: Config<I>, I: 'static>(generic_event: <T as Config<I>>::RuntimeEvent) {
	frame_system::Pallet::<T>::assert_last_event(generic_event.into());
}

// const CHAIN_IDENTIFIER: u32 = 1080;

#[allow(dead_code)]
fn setup_env<T: Config<I>, I: 'static>() -> ((ProvingKey<Bn254>, ConstraintMatrices<Fr>), &'static Mutex<WitnessCalculator>)
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

	let path_2_2 = "../../solidity-fixtures/solidity-fixtures/reward_2/30/circuit_final.zkey";
	let mut file_2_2 = File::open(path_2_2).unwrap();
	let params_2_2 = read_zkey(&mut file_2_2).unwrap();

	let wasm_2_2_path = "../../solidity-fixtures/solidity-fixtures/reward_2/30/reward_30_2.wasm";

	let wc_2_2 = circom_from_folder(wasm_2_2_path);

	println!("Setting up the verifier pallet");
	let mut vk_2_2_bytes = Vec::new();
	params_2_2.0.vk.serialize(&mut vk_2_2_bytes).unwrap();

	// assert_ok!(ClaimsVerifier::force_set_parameters(
	// 	RuntimeOrigin::root(),
	// 	2,
	// 	vk_2_2_bytes.try_into().unwrap(),
	// ));


	assert_ok!(<pallet_claims_verifier::Pallet<T, I>>::force_set_parameters(
		RawOrigin::Root.into(),
		2,
		vk_2_2_bytes.clone().try_into().unwrap()
	));

	(params_2_2, wc_2_2)
	// pk_2_2_bytes
}

pub fn hasher_params() -> Vec<u8> {
	let curve = Curve::Bn254;
	let params = setup_params::<ark_bn254::Fr>(curve, 5, 3);
	params.to_bytes()
}

// const SEED: u32 = 0;
// const MAX_EDGES: u32 = 2;

benchmarks_instance_pallet! {

	where_clause {  where T: pallet_hasher::Config<I>,
		T: pallet_linkable_tree::Config<I>,
		T: pallet_claims_verifier::Config<I>,
		T: pallet_mt::Config<I, Element = Element>,
		T: orml_tokens::Config<Amount = Amount>,
		<T as frame_system::Config>::AccountId: From<AccountId>,
		<<T as pallet::Config<I>>::Currency as MultiCurrency<<T as frame_system::Config>::AccountId>>::CurrencyId: From<AssetId>,
		pallet_linkable_tree::Pallet<T, I>: webb_primitives::linkable_tree::LinkableTreeInspector<LinkableTreeConfigration<T, I>> }

	create {
		let number_of_anchors = 2u8;

	    pallet_hasher::Pallet::<T, I>::force_set_parameters(RawOrigin::Root.into(), hasher_params().try_into().unwrap()).unwrap();

		let asset_id = <<T as crate::Config<I>>::NativeCurrencyId as Get<crate::CurrencyIdOf<T, I>>>::get();
	}: create(RawOrigin::Root, number_of_anchors, asset_id)
	// verify {
	// 	assert_last_event::<T, I>(Event::VAnchorCreation{ tree_id : 0_u32.into() }.into())
	// }

	// transact {
	// 	let pk_2_2_bytes =  setup_env::<T,I>();
	//
	// 	let deposit_size: u32 = 50_000_000;
	// 	let asset_id = <<T as crate::Config<I>>::NativeCurrencyId as Get<crate::CurrencyIdOf<T, I>>>::get();
	// 	let depth = <T as pallet_mt::Config<I>>::MaxTreeDepth::get();
	//
	// 	let tree_id = <VAnchor<T, I> as VAnchorInterface<VAnchorConfigration<T, I>>>::create(None, depth, 1u32, asset_id, 1u32.into())?;
	//
	// 	<VAnchor<T, I> as VAnchorInterface<VAnchorConfigration<T, I>>>::set_max_deposit_amount(100u32.into(), 2u32.into())?;
	//
	// 	let transactor : T::AccountId = account("", 0, SEED);
	// 	let recipient : T::AccountId = account("", 1, SEED);
	// 	let relayer: T::AccountId = account("", 4, SEED);
	// 	let ext_amount: u32 = 10;
	// 	let fee: u32 = 0;
	//
	// 	<<T as pallet_mt::Config<I>>::Currency as Currency<T::AccountId>>::make_free_balance_be(&transactor.clone(), 100_000_000u32.into());
	//
	// 	let public_amount : i128 = 10;
	//
	// 	let chain_type = [2, 0];
	// 	let chain_id = compute_chain_id_type(CHAIN_IDENTIFIER, chain_type);
	// 	let in_chain_ids = [chain_id; 2];
	// 	let in_amounts = [0, 0];
	// 	let in_indices = [0, 1];
	// 	let out_chain_ids = [chain_id; 2];
	// 	let out_amounts = [10, 0];
	//
	// 	let in_utxos = setup_utxos(in_chain_ids, in_amounts, Some(in_indices));
	// 	// We are adding indicies to out utxos, since they will be used as an input utxos in next
	// 	// transaction
	// 	let out_utxos = setup_utxos(out_chain_ids, out_amounts, Some(in_indices));
	//
	// 	let output1 = out_utxos[0].commitment.into_repr().to_bytes_be();
	// 	let output2 = out_utxos[1].commitment.into_repr().to_bytes_be();
	// 	let ext_data = ExtData::<T::AccountId, AmountOf<T, I>, BalanceOf<T, I>, CurrencyIdOf<T, I>>::new(
	// 		recipient.into(),
	// 		relayer.into(),
	// 		ext_amount.into(),
	// 		fee.into(),
	// 		0u32.into(),
	// 		(AssetId::MAX - 1).into(),
	// 		output1.to_vec(), // Mock encryption value, not meant to be used in production
	// 		output2.to_vec(), // Mock encryption value, not meant to be used in production
	// 	);
	//
	// 	let ext_data_hash = keccak_256(&ext_data.encode_abi());
	//
	// 	let custom_root = <pallet_mt::Pallet<T, I>>::get_default_root(tree_id).unwrap();
	// 	let neighbor_roots: [Element; 1] = <pallet_linkable_tree::Pallet<T, I> as LinkableTreeInspector<
	// 		LinkableTreeConfigration<T, I>,
	// 	>>::get_neighbor_roots(tree_id).unwrap().try_into().unwrap();
	//
	// 	let (proof, public_inputs) = setup_zk_circuit(
	// 		public_amount,
	// 		chain_id,
	// 		ext_data_hash.to_vec(),
	// 		in_utxos,
	// 		out_utxos.clone(),
	// 		pk_2_2_bytes,
	// 		neighbor_roots,
	// 		custom_root,
	// 	);
	//
	// 	// Deconstructing public inputs
	// 	let (_chain_id, public_amount, root_set, nullifiers, commitments, ext_data_hash) =
	// 		deconstruct_public_inputs_el(&public_inputs);
	//
	// 	// Constructing proof data
	// 	let proof_data =
	// 		ProofData::new(proof, public_amount, root_set, nullifiers, commitments, ext_data_hash);
	// }: _(RawOrigin::Signed(transactor.clone()), tree_id, proof_data.clone(), ext_data)
	// verify {
	// 	assert_last_event::<T, I>(
	// 		Event::Transaction {
	// 		transactor,
	// 		tree_id,
	// 		leafs : proof_data.output_commitments,
	// 		encrypted_output1: output1.to_vec(),
	// 		encrypted_output2: output2.to_vec(),
	// 		amount : ext_amount.into() }.into()
	// 	)
	// }

	// set_max_deposit_amount {
	// }: _(RawOrigin::Root, 100u32.into(), 101u32.into())
	// verify {
	// 	assert_last_event::<T, I>(Event::MaxDepositAmountChanged{ max_deposit_amount : 100_u32.into() }.into())
	// }

}

impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Test);
