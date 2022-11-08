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

//! Token wrapper pallet benchmarking.

#![cfg(feature = "runtime-benchmarks")]

use super::*;

use frame_benchmarking::{benchmarks, impl_benchmark_test_suite, whitelisted_caller};

use crate::traits::TokenWrapperInterface;
use asset_registry::{Registry, ShareTokenRegistry};
use frame_support::dispatch::DispatchError;
use frame_system::RawOrigin;
use orml_traits::MultiCurrency;
use sp_std::vec;

fn assert_last_event<T: Config>(generic_event: <T as Config>::RuntimeEvent) {
	frame_system::Pallet::<T>::assert_last_event(generic_event.into());
}
use crate::pallet::Pallet as TokenWrapper;

benchmarks! {
	wrap {
		let existential_balance: u32 = 1000;
		let balance: u32 = 10_000;
		let recipient: T::AccountId = whitelisted_caller();
		let first_token_id = <<T as Config>::AssetRegistry as Registry<<T as asset_registry::Config>::AssetId, Vec<u8>, <T as asset_registry::Config>::Balance, BoundedVec<u8, T::StringLimit>, DispatchError>>::create_asset(
			&b"shib".to_vec(),
			existential_balance.into(),
		)
		.unwrap();
		let second_token_id = <<T as Config>::AssetRegistry as Registry<<T as asset_registry::Config>::AssetId, Vec<u8>, <T as asset_registry::Config>::Balance, BoundedVec<u8, T::StringLimit>, DispatchError>>::create_asset(
			&b"doge".to_vec(),
			existential_balance.into(),
		)
		.unwrap();

		let pool_share_id = <<T as Config>::AssetRegistry as ShareTokenRegistry<<T as asset_registry::Config>::AssetId, Vec<u8>, T::Balance, BoundedVec<u8, T::StringLimit>, DispatchError>>::create_shared_asset(
			&b"meme".to_vec(),
			&vec![second_token_id, first_token_id],
			existential_balance.into(),
		)
		.unwrap();


		<<T as Config>::Currency as MultiCurrency<T::AccountId>>::deposit(
			TokenWrapper::<T>::to_currency_id(first_token_id).unwrap(),
			&recipient,
			balance.into()
		);

		let fee: <T::Currency as MultiCurrency<<T as frame_system::Config>::AccountId>>::Balance = 5u32.into();

		WrappingFeePercent::<T>::insert(pool_share_id, fee);

	}: _(RawOrigin::Signed(recipient.clone()), first_token_id, pool_share_id, 5_000u32.into(), recipient.clone())
	verify {
		assert_last_event::<T>(
			Event::WrappedToken {
				pool_share_asset: pool_share_id,
				asset_id: first_token_id,
				amount: 5_000u32.into(),
				recipient: recipient,
			}.into()
		)
	}

	unwrap {
		let existential_balance: u32 = 1000;
		let balance: u32 = 10_000;
		let recipient: T::AccountId = whitelisted_caller();
		let first_token_id = <<T as Config>::AssetRegistry as Registry<<T as asset_registry::Config>::AssetId, Vec<u8>, <T as asset_registry::Config>::Balance, BoundedVec<u8, T::StringLimit>, DispatchError>>::create_asset(
			&b"shib".to_vec(),
			existential_balance.into(),
		)
		.unwrap();
		let second_token_id = <<T as Config>::AssetRegistry as Registry<<T as asset_registry::Config>::AssetId, Vec<u8>, <T as asset_registry::Config>::Balance, BoundedVec<u8, T::StringLimit>, DispatchError>>::create_asset(
			&b"doge".to_vec(),
			existential_balance.into(),
		)
		.unwrap();

		let pool_share_id = <<T as Config>::AssetRegistry as ShareTokenRegistry<<T as asset_registry::Config>::AssetId, Vec<u8>, T::Balance, BoundedVec<u8, T::StringLimit>, DispatchError>>::create_shared_asset(
			&b"meme".to_vec(),
			&vec![second_token_id, first_token_id],
			existential_balance.into(),
		)
		.unwrap();

		<<T as Config>::Currency as MultiCurrency<T::AccountId>>::deposit(
			TokenWrapper::<T>::to_currency_id(first_token_id).unwrap(),
			&recipient,
			balance.into()
		);

		<TokenWrapper<T> as TokenWrapperInterface<T::AccountId, <T as asset_registry::Config>::AssetId, BalanceOf<T>, T::ProposalNonce>>::wrap(recipient.clone(), first_token_id, pool_share_id, 5_000u32.into(), recipient.clone());

	}:_(RawOrigin::Signed(recipient.clone()), pool_share_id, first_token_id, 5_000u32.into(), recipient.clone())
	verify {
		assert_last_event::<T>(
			Event::UnwrappedToken {
				pool_share_asset: pool_share_id,
				asset_id: first_token_id,
				amount: 5_000u32.into(),
				recipient: recipient,
			}.into()
		)
	}

	set_wrapping_fee {
		let existential_balance: u32 = 1000;
		let balance: u32 = 10_000;
		let recipient: T::AccountId = whitelisted_caller();
		let first_token_id = <<T as Config>::AssetRegistry as Registry<<T as asset_registry::Config>::AssetId, Vec<u8>, <T as asset_registry::Config>::Balance, BoundedVec<u8, T::StringLimit>, DispatchError>>::create_asset(
			&b"shib".to_vec(),
			existential_balance.into(),
		)
		.unwrap();
		let second_token_id = <<T as Config>::AssetRegistry as Registry<<T as asset_registry::Config>::AssetId, Vec<u8>, <T as asset_registry::Config>::Balance, BoundedVec<u8, T::StringLimit>, DispatchError>>::create_asset(
			&b"doge".to_vec(),
			existential_balance.into(),
		)
		.unwrap();

		let pool_share_id = <<T as Config>::AssetRegistry as ShareTokenRegistry<<T as asset_registry::Config>::AssetId, Vec<u8>, T::Balance, BoundedVec<u8, T::StringLimit>, DispatchError>>::create_shared_asset(
			&b"meme".to_vec(),
			&vec![second_token_id, first_token_id],
			existential_balance.into(),
		)
		.unwrap();

		let nonce = 1048u32;
	}:_(RawOrigin::Root, 5u32.into(), pool_share_id, nonce.into())
	verify {
		assert_last_event::<T>(
			Event::UpdatedWrappingFeePercent {
				into_pool_share_id: pool_share_id,
				wrapping_fee_percent: 5u32.into(),
			}.into()
		)
	}

	set_fee_recipient {
		let fee_recipient: T::AccountId = whitelisted_caller();
		let nonce = 1048u32;
	}:_(RawOrigin::Root,fee_recipient.clone(), nonce.into())
	verify {
		assert_last_event::<T>(
			Event::UpdatedFeeRecipient {
				fee_recipient
			}.into()
		)
	}
}

impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Test);
