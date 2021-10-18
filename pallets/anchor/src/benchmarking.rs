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


use darkwebb_primitives::{anchor::AnchorInterface, traits::merkle_tree::{TreeInspector}};
use frame_benchmarking::{account, benchmarks, impl_benchmark_test_suite, whitelist_account, whitelisted_caller};
use frame_system::RawOrigin;
use orml_traits::MultiCurrency;

use ark_ff::{BigInteger, PrimeField};
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use arkworks_gadgets::{
	poseidon::PoseidonParameters,
	prelude::ark_groth16::ProvingKey,
	setup::{
		bridge::{
			prove_groth16_circuit_circomx5, setup_arbitrary_data, setup_groth16_random_circuit_circomx5,
			setup_leaf_circomx5, setup_set, Circuit_Circomx5,
		},
		common::{setup_circom_params_x5_3, setup_circom_params_x5_5, setup_tree_and_create_path_tree_circomx5, Curve},
	},
	utils::{get_mds_poseidon_circom_bn254_x5_3, get_rounds_poseidon_circom_bn254_x5_3},
};

use frame_support::traits::{Currency, Get, OnInitialize};
use crate::Pallet as Anchor;

fn assert_last_event<T: Config>(generic_event: <T as Config>::Event) {
	frame_system::Pallet::<T>::assert_last_event(generic_event.into());
}

pub const TREE_DEPTH: usize = 30;
pub const M: usize = 2;
pub const DEPOSIT_SIZE: u128 = 10_000;


const SEED: u32 = 0;
const MAX_EDGES: u32 = 246;
type Bn254Fr = ark_bn254::Fr;

benchmarks! {

	create {
	  let i in 1..MAX_EDGES;
	  let d in 1..<T as pallet_mt::Config>::MaxTreeDepth::get() as u32;

	  let deposit_size: u32 = 1_000_000;
	  let asset_id = <<T as pallet_mixer::Config>::NativeCurrencyId as Get<pallet_mixer::CurrencyIdOf<T, _>>>::get();
	}: _(RawOrigin::Root, deposit_size.into(), i, d as u8, asset_id)

	deposit {
	  let caller: T::AccountId = whitelisted_caller();
	  let creator: T::AccountId = account("creator", 0, SEED);
	  whitelist_account!(creator);
	  let deposit_size: u32 = 1_000_000;
	  let asset_id = <<T as pallet_mixer::Config>::NativeCurrencyId as Get<pallet_mixer::CurrencyIdOf<T, _>>>::get();
	  let depth = <T as pallet_mt::Config>::MaxTreeDepth::get();
	  let tree_id = <Anchor<T> as AnchorInterface<AnchorConfigration<T, _>>>::create(creator, deposit_size.into(), depth, MAX_EDGES as u32, asset_id)?;
	  let leaf = <T as pallet_mt::Config>::Element::from_bytes(&[1u8; 32]);
	  <<T as pallet_mt::Config>::Currency as Currency<T::AccountId>>::make_free_balance_be(&caller.clone(), 100_000_000u32.into());

	}: _(RawOrigin::Signed(caller.clone()), tree_id, leaf)
	verify {
	  assert_eq!(<<T as pallet_mixer::Config>::Currency as MultiCurrency<T::AccountId>>::total_balance(asset_id, &pallet_mixer::Pallet::<T>::account_id()), deposit_size.into())
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
		
		let curve = Curve::Bn254;
		
		let pk_bytes = {
			let rng = &mut ark_std::test_rng();
			let params = match curve {
				Curve::Bn254 => {
					let rounds = get_rounds_poseidon_circom_bn254_x5_3::<ark_bn254::Fr>();
					let mds = get_mds_poseidon_circom_bn254_x5_3::<ark_bn254::Fr>();
					PoseidonParameters::new(rounds, mds)
				}
				Curve::Bls381 => todo!("Setup environment for bls381"),
			};
			// 1. Setup The Hasher Pallet.
			
			//Todo
			
			//<T as pallet_mt::Config>::Hasher::force_set_parameters{parameters: params.to_bytes()};

	
			// 2. Initialize MerkleTree pallet.

			// Todo
			//<<T as pallet_mixer::Config>::Tree as OnInitialize<u64>>::on_initialize(1);

			// 3. Setup the VerifierPallet
			//    but to do so, we need to have a VerifyingKey
			let mut verifier_key_bytes = Vec::new();
			let mut proving_key_bytes = Vec::new();
		
			match curve {
				Curve::Bn254 => {
					let (pk, vk) = setup_groth16_random_circuit_circomx5::<_, ark_bn254::Bn254, TREE_DEPTH, M>(rng, curve);
					vk.serialize(&mut verifier_key_bytes).unwrap();
					pk.serialize(&mut proving_key_bytes).unwrap();
				}
				Curve::Bls381 => {
					let (pk, vk) =
						setup_groth16_random_circuit_circomx5::<_, ark_bls12_381::Bls12_381, TREE_DEPTH, M>(rng, curve);
					vk.serialize(&mut verifier_key_bytes).unwrap();
					pk.serialize(&mut proving_key_bytes).unwrap();
				}
			};

			// Todo
			//<T as pallet_mixer::Config>::Verifier::force_set_parameters(RawOrigin::Root, verifier_key_bytes);
			
			proving_key_bytes
		};

		let rng = &mut ark_std::test_rng();

		// inputs
		let caller: T::AccountId = whitelisted_caller();
		<<T as pallet_mt::Config>::Currency as Currency<T::AccountId>>::make_free_balance_be(&caller.clone(), 100_000_000u32.into());
		let src_chain_id: u32 = 1;
		let recipient_account_id: T::AccountId = account("recipient", 0, SEED);
		let relayer_account_id: T::AccountId = account("recipient", 1, SEED);
		let creator: T::AccountId = account("creator", 0, SEED);
		whitelist_account!(recipient_account_id);
		whitelist_account!(relayer_account_id);
		whitelist_account!(creator);
		let fee_value: u32 = 0;
		let refund_value: u32 = 0;
		// fit inputs to the curve.
		let chain_id = Bn254Fr::from(src_chain_id.into());
		let recipient = Bn254Fr::from(recipient_account_id.into());
		let relayer = Bn254Fr::from(relayer_account_id.into());
		let fee = Bn254Fr::from(fee_value);
		let refund = Bn254Fr::from(refund_value);

		let params5 = setup_circom_params_x5_5::<Bn254Fr>(curve);
		let (leaf_private, leaf_public, leaf, nullifier_hash) = setup_leaf_circomx5(chain_id, &params5, rng);

		let deposit_size: u32 = 1_000_000;
		let depth = <T as pallet_mt::Config>::MaxTreeDepth::get();
		let asset_id = <<T as pallet_mixer::Config>::NativeCurrencyId as Get<pallet_mixer::CurrencyIdOf<T, _>>>::get();
		
		let tree_id = <Anchor<T> as AnchorInterface<AnchorConfigration<T, _>>>::create(creator, deposit_size.into(), depth, 2, asset_id)?;

		<Anchor<T> as AnchorInterface<AnchorConfigration<T, _>>>::deposit(
			caller.clone(),
			tree_id,
			<T as pallet_mt::Config>::Element::from_bytes(&leaf.into_repr().to_bytes_le()),
		);

		// the withdraw process..
		// we setup the inputs to our proof generator.
		let params3 = setup_circom_params_x5_3::<Bn254Fr>(curve);
		let (mt, path) = setup_tree_and_create_path_tree_circomx5::<_, TREE_DEPTH>(&[leaf], 0, &params3);
		let root = mt.root().inner();
		let tree_root = <pallet_mt::Pallet<T> as TreeInspector<T::AccountId, <T as pallet_mt::Config>::TreeId, <T as pallet_mt::Config>::Element>>::get_root(tree_id).unwrap();

		let mut roots = [Bn254Fr::default(); M];
		roots[0] = root; // local root.

		let set_private_inputs = setup_set(&root, &roots);
		let arbitrary_input = setup_arbitrary_data(recipient, relayer, fee, refund);
		// setup the circuit.
		let circuit = Circuit_Circomx5::new(
			arbitrary_input,
			leaf_private,
			leaf_public,
			set_private_inputs,
			roots,
			params5,
			path,
			root,
			nullifier_hash,
		);
		let pk = ProvingKey::<ark_bn254::Bn254>::deserialize(&*pk_bytes).unwrap();
		// generate the proof.
		let proof = prove_groth16_circuit_circomx5(&pk, circuit, rng);

		// format the input for the pallet.
		let mut proof_bytes = Vec::new();
		proof.serialize(&mut proof_bytes).unwrap();
		let roots_element = roots
			.iter()
			.map(|v| <T as pallet_mt::Config>::Element::from_bytes(&v.into_repr().to_bytes_le()))
			.collect();

		let nullifier_hash_element = <T as pallet_mt::Config>::Element::from_bytes(&nullifier_hash.into_repr().to_bytes_le());

	}: _(
		RawOrigin::Signed(caller), 
		tree_id,proof_bytes,
		src_chain_id.into(),
		roots_element,
		nullifier_hash_element,
		recipient_account_id.clone(),
		relayer_account_id.into(),
		fee_value.into(),
		refund_value.into()
	)
	verify {
		assert_eq!(<<T as pallet_mixer::Config>::Currency as MultiCurrency<T::AccountId>>::total_balance(asset_id, &recipient_account_id), deposit_size.into())
	}

}

impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Test);
