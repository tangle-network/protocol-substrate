// Copyright (C) 2021 Parity Technologies (UK) Ltd.
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

//! Auxillary struct/enums for parachain runtimes.
//! Taken from polkadot/runtime/common (at a21cd64) and adapted for parachains.

use frame_support::traits::{Currency, Imbalance, OnUnbalanced};

pub type NegativeImbalance<T> =
	<pallet_balances::Pallet<T> as Currency<<T as frame_system::Config>::AccountId>>::NegativeImbalance;

/// Logic for the author to get a portion of fees.
pub struct ToTreasury<R>(sp_std::marker::PhantomData<R>);
impl<R> OnUnbalanced<NegativeImbalance<R>> for ToTreasury<R>
where
	R: pallet_balances::Config + pallet_treasury::Config,
	<R as frame_system::Config>::AccountId: From<polkadot_primitives::v1::AccountId>,
	<R as frame_system::Config>::AccountId: Into<polkadot_primitives::v1::AccountId>,
	<R as frame_system::Config>::Event: From<pallet_balances::Event<R>>,
{
	fn on_nonzero_unbalanced(amount: NegativeImbalance<R>) {
		let numeric_amount = amount.peek();
		let treasury = <pallet_treasury::Pallet<R>>::account_id();
		<pallet_balances::Pallet<R>>::resolve_creating(&treasury, amount);
		<frame_system::Pallet<R>>::deposit_event(pallet_balances::Event::Deposit {
			who: treasury,
			amount: numeric_amount,
		});
	}
}

pub struct DealWithFees<R>(sp_std::marker::PhantomData<R>);
impl<R> OnUnbalanced<NegativeImbalance<R>> for DealWithFees<R>
where
	R: pallet_balances::Config + pallet_treasury::Config,
	<R as frame_system::Config>::AccountId: From<polkadot_primitives::v1::AccountId>,
	<R as frame_system::Config>::AccountId: Into<polkadot_primitives::v1::AccountId>,
	<R as frame_system::Config>::Event: From<pallet_balances::Event<R>>,
{
	fn on_unbalanceds<B>(mut fees_then_tips: impl Iterator<Item = NegativeImbalance<R>>) {
		if let Some(mut fees) = fees_then_tips.next() {
			if let Some(tips) = fees_then_tips.next() {
				tips.merge_into(&mut fees);
			}
			<ToTreasury<R> as OnUnbalanced<_>>::on_unbalanced(fees);
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use frame_support::{
		parameter_types,
		traits::{FindAuthor, ValidatorRegistration},
		PalletId,
	};
	use frame_system::limits;
	use polkadot_primitives::v1::AccountId;
	use sp_core::H256;
	use sp_runtime::{
		testing::Header,
		traits::{BlakeTwo256, IdentityLookup},
		Perbill, Permill,
	};

	type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
	type Block = frame_system::mocking::MockBlock<Test>;

	frame_support::construct_runtime!(
		pub enum Test where
			Block = Block,
			NodeBlock = Block,
			UncheckedExtrinsic = UncheckedExtrinsic,
		{
			System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
			Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
			Treasury: pallet_treasury::{Pallet, Call, Storage, Event<T>, Config},
		}
	);

	parameter_types! {
		pub const BlockHashCount: u64 = 250;
		pub BlockLength: limits::BlockLength = limits::BlockLength::max(2 * 1024);
		pub const AvailableBlockRatio: Perbill = Perbill::one();
		pub const MaxReserves: u32 = 50;
	}

	impl frame_system::Config for Test {
		type AccountData = pallet_balances::AccountData<u64>;
		type AccountId = AccountId;
		type BaseCallFilter = frame_support::traits::Everything;
		type BlockHashCount = BlockHashCount;
		type BlockLength = BlockLength;
		type BlockNumber = u64;
		type BlockWeights = ();
		type Call = Call;
		type DbWeight = ();
		type Event = Event;
		type Hash = H256;
		type Hashing = BlakeTwo256;
		type Header = Header;
		type Index = u64;
		type Lookup = IdentityLookup<Self::AccountId>;
		type MaxConsumers = frame_support::traits::ConstU32<16>;
		type OnKilledAccount = ();
		type OnNewAccount = ();
		type OnSetCode = ();
		type Origin = Origin;
		type PalletInfo = PalletInfo;
		type SS58Prefix = ();
		type SystemWeightInfo = ();
		type Version = ();
	}

	impl pallet_balances::Config for Test {
		type AccountStore = System;
		type Balance = u64;
		type DustRemoval = ();
		type Event = Event;
		type ExistentialDeposit = ();
		type MaxLocks = ();
		type MaxReserves = MaxReserves;
		type ReserveIdentifier = [u8; 8];
		type WeightInfo = ();
	}

	pub struct OneAuthor;
	impl FindAuthor<AccountId> for OneAuthor {
		fn find_author<'a, I>(_: I) -> Option<AccountId>
		where
			I: 'a,
		{
			Some(Default::default())
		}
	}

	pub struct IsRegistered;
	impl ValidatorRegistration<AccountId> for IsRegistered {
		fn is_registered(_id: &AccountId) -> bool {
			true
		}
	}

	impl pallet_authorship::Config for Test {
		type EventHandler = ();
		type FilterUncle = ();
		type FindAuthor = OneAuthor;
		type UncleGenerations = ();
	}

	parameter_types! {
		pub const ProposalBond: Permill = Permill::from_percent(5);
		pub const ProposalBondMinimum: u64 = 1;
		pub const SpendPeriod: u64 = 2;
		pub const Burn: Permill = Permill::from_percent(50);
		pub const TreasuryPalletId: PalletId = PalletId(*b"py/trsry");
		pub const BountyUpdatePeriod: u32 = 20;
		pub const BountyCuratorDeposit: Permill = Permill::from_percent(50);
		pub const BountyValueMinimum: u64 = 1;
		pub const MaxApprovals: u32 = 100;
	}

	impl pallet_treasury::Config for Test {
		type ApproveOrigin = frame_system::EnsureRoot<AccountId>;
		type Burn = Burn;
		type BurnDestination = ();
		type Currency = pallet_balances::Pallet<Test>;
		type Event = Event;
		type MaxApprovals = MaxApprovals;
		type OnSlash = ();
		type PalletId = TreasuryPalletId;
		type ProposalBond = ProposalBond;
		type ProposalBondMinimum = ProposalBondMinimum;
		type RejectOrigin = frame_system::EnsureRoot<AccountId>;
		type SpendFunds = ();
		type SpendPeriod = SpendPeriod;
		// Just gets burned.
		type WeightInfo = ();
	}

	pub fn new_test_ext() -> sp_io::TestExternalities {
		let mut t = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();
		// We use default for brevity, but you can configure as desired if needed.
		pallet_balances::GenesisConfig::<Test>::default()
			.assimilate_storage(&mut t)
			.unwrap();
		t.into()
	}

	#[test]
	fn test_fees_and_tip_split() {
		new_test_ext().execute_with(|| {
			let fee = Balances::issue(10);
			let tip = Balances::issue(20);

			assert_eq!(Balances::free_balance(AccountId::default()), 0);

			DealWithFees::on_unbalanceds(vec![fee, tip].into_iter());

			// Author gets 100% of tip and 100% of fee = 30
			assert_eq!(Balances::free_balance(Treasury::account_id()), 30);
		});
	}
}
