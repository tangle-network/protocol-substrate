// This file is part of Webb.

// Copyright (C) 2022 Webb Technologies Inc.
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

//! Signature pallet benchmarking.
use super::*;
use frame_benchmarking::{
	benchmarks_instance_pallet, impl_benchmark_test_suite, whitelisted_caller,
};
use frame_system::RawOrigin;
use sp_io::{
	crypto::{ecdsa_generate, ecdsa_sign_prehashed},
	hashing::keccak_256,
};
use sp_runtime::key_types::DUMMY;
use webb_primitives::{
	utils::{compute_chain_id_type, derive_resource_id},
	webb_proposals::SubstrateTargetSystem,
};
use frame_support::BoundedVec;

/// Helper function to test last event
fn assert_last_event<T: Config<I>, I: 'static>(generic_event: <T as Config<I>>::RuntimeEvent) {
	frame_system::Pallet::<T>::assert_last_event(generic_event.into());
}

/// Helper function to generate old/new maintainer signatures
/// The function will add add a maintainer to storage, create a new message with new maintainer and
/// sign Returns (old_maintainer_key, message, signature)
pub fn generate_maintainer_signatures<T: Config<I>, I: 'static>() -> (Vec<u8>, Vec<u8>, Vec<u8>) {
	let new_maintainer = ecdsa_generate(DUMMY, None);
	let old_maintainer = ecdsa_generate(DUMMY, None);
	let old_maintainer_key = set_maintainer_on_chain::<T, I>(old_maintainer);
	let mut message = vec![];
	let nonce = 1u32.encode();
	message.extend_from_slice(&nonce);
	message.extend_from_slice(&new_maintainer.encode());
	let hash = keccak_256(&message);
	let signature = ecdsa_sign_prehashed(DUMMY, &old_maintainer, &hash).unwrap();
	(old_maintainer_key, message, signature.encode())
}

/// Helper function to generate proposal data
fn make_proposal_data(encoded_r_id: Vec<u8>, nonce: [u8; 4], encoded_call: Vec<u8>) -> Vec<u8> {
	let mut prop_data = encoded_r_id;
	prop_data.extend_from_slice(&[0u8; 4]);
	prop_data.extend_from_slice(&nonce);
	prop_data.extend_from_slice(&encoded_call[..]);
	prop_data
}

/// Helper function to set maintainer on chain
fn set_maintainer_on_chain<T: Config<I>, I: 'static>(pub_key: sp_core::ecdsa::Public) -> Vec<u8> {
	let maintainer_key : BoundedVec<u8, T::MaxStringLength> =
		libsecp256k1::PublicKey::parse_compressed(&pub_key.0).unwrap().serialize()[1..].to_vec().try_into().unwrap();
	Maintainer::<T, _>::put(maintainer_key.clone());
	maintainer_key.into_inner()
}

/// Return the src_chain_id in correct format
fn get_chain_id() -> u64 {
	let chain_type = [2, 0];
	compute_chain_id_type(1u32, chain_type)
}

benchmarks_instance_pallet! {

	where_clause {  where T::Proposal : From<frame_system::Call<T>> }

	set_maintainer {
		let caller: T::AccountId = whitelisted_caller();
		let (old_maintainer, message, signature) = generate_maintainer_signatures::<T, I>();
	}: _(RawOrigin::Signed(caller), message.clone().try_into().unwrap(), signature.try_into().unwrap())
	verify {
		assert_last_event::<T, I>(Event::MaintainerSet{old_maintainer: old_maintainer.try_into().unwrap(), new_maintainer: message.try_into().unwrap()}.into());
	}

	force_set_maintainer {
		let caller: T::AccountId = whitelisted_caller();
		let new_maintainer = ecdsa_generate(DUMMY, None);
	}: _(RawOrigin::Root, new_maintainer.encode().try_into().unwrap())
	verify {
		assert_last_event::<T, I>(Event::MaintainerSet{old_maintainer: Default::default(), new_maintainer: new_maintainer.encode().try_into().unwrap()}.into());
	}

	set_resource {
		let id: ResourceId = ResourceId([1; 32]);
	}: _(RawOrigin::Root, id)
	verify {
	   assert_eq!(Resources::<T, I>::get(id), Some(()));
	}

	remove_resource {
		let id: ResourceId = ResourceId([1; 32]);
		let _ = crate::Pallet::<T,I>::set_resource(RawOrigin::Root.into(), id);
	}: _(RawOrigin::Root, id)
	verify {
	   assert_eq!(Resources::<T, I>::get(id), None);
	}

	whitelist_chain {
	}: _(RawOrigin::Root, 0_u32.into())
	verify {
		assert_last_event::<T, I>(Event::ChainWhitelisted{chain_id : 0_u32.into()}.into());
	}

	set_resource_with_signature {
		let caller: T::AccountId = whitelisted_caller();

		// set a new maintainer
		let maintainer = ecdsa_generate(DUMMY, None);
		set_maintainer_on_chain::<T, I>(maintainer);

		// whitelist chain
		let src_id = get_chain_id();
		let _ = crate::Pallet::<T,I>::whitelist_chain(RawOrigin::Root.into(), src_id.into());
		let r_id : ResourceId = derive_resource_id(1080u32, SubstrateTargetSystem { pallet_index: 2, tree_id: 1 }).into();

		// prepare proposal
		let call : <T as pallet::Config<I>>::Proposal = frame_system::Call::<T>::remark { remark: vec![10] }.into();
		let call_encoded = call.encode();
		let nonce = [0u8, 0u8, 0u8, 1u8];
		let prop_data = make_proposal_data(r_id.encode(), nonce, call_encoded);
		let msg = keccak_256(&prop_data);
		let signature = ecdsa_sign_prehashed(DUMMY, &maintainer, &msg).unwrap();
	}: _(RawOrigin::Signed(caller), src_id.into(), prop_data.try_into().unwrap(), signature.encode().try_into().unwrap())
	verify {
		assert_last_event::<T, I>(Event::ProposalSucceeded{chain_id : src_id.into(), proposal_nonce : 1_u32.into()}.into());
	}

	execute_proposal {
		let caller: T::AccountId = whitelisted_caller();

		// set a new maintainer
		let maintainer = ecdsa_generate(DUMMY, None);
		set_maintainer_on_chain::<T, I>(maintainer);

		// whitelist chain
		let src_id = get_chain_id();
		let _ = crate::Pallet::<T,I>::whitelist_chain(RawOrigin::Root.into(), src_id.into());

		// set resource
		let r_id : ResourceId = derive_resource_id(1080u32, SubstrateTargetSystem { pallet_index: 2, tree_id: 1 }).into();
		let _ = crate::Pallet::<T,I>::set_resource(RawOrigin::Root.into(), r_id.into());

		// prepare proposal
		let call : <T as pallet::Config<I>>::Proposal = frame_system::Call::<T>::remark { remark: vec![10] }.into();
		let call_encoded = call.encode();
		let nonce = [0u8, 0u8, 0u8, 1u8];
		let prop_data = make_proposal_data(r_id.encode(), nonce, call_encoded);
		let msg = keccak_256(&prop_data);
		let signature = ecdsa_sign_prehashed(DUMMY, &maintainer, &msg).unwrap();
	}: _(RawOrigin::Signed(caller), src_id.into(), prop_data, signature.encode())
	verify {
		assert_last_event::<T, I>(Event::ProposalSucceeded{chain_id : src_id.into(), proposal_nonce : 1_u32.into()}.into());
	}
}

impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Test);
