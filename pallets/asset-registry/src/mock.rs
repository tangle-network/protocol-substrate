// This file is part of Basilisk-node.

// Copyright (C) 2020-2021  Intergalactic, Limited (GIB).
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

#![cfg(test)]
use webb_primitives::{AssetId, Balance};
use frame_support::{
	parameter_types,
	traits::{Everything, GenesisBuild},
};
use frame_system as system;
use polkadot_xcm::v0::MultiLocation;
use scale_info::TypeInfo;
use sp_core::H256;
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup},
};

// use polkadot_xcm::v0::MultiLocation;

use crate::{self as asset_registry, Config};

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

frame_support::construct_runtime!(
	pub enum Test where
	 Block = Block,
	 NodeBlock = Block,
	 UncheckedExtrinsic = UncheckedExtrinsic,
	 {
		 System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
		 Registry: asset_registry::{Pallet, Call, Storage, Event<T>},
	 }

);

parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub const SS58Prefix: u8 = 63;
	pub const NativeAssetId: AssetId = 0;
	pub const RegistryStringLimit: u32 = 10;
}

impl system::Config for Test {
	type AccountData = ();
	type AccountId = u64;
	type BaseCallFilter = Everything;
	type BlockHashCount = BlockHashCount;
	type BlockLength = ();
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
	type OnKilledAccount = ();
	type OnNewAccount = ();
	type OnSetCode = ();
	type Origin = Origin;
	type PalletInfo = PalletInfo;
	type SS58Prefix = SS58Prefix;
	type SystemWeightInfo = ();
	type Version = ();
	type MaxConsumers = frame_support::traits::ConstU32<16>;
}

use codec::{Decode, Encode};

#[derive(Debug, Encode, Decode, Clone, PartialEq, Eq, TypeInfo)]
pub struct AssetLocation(pub MultiLocation);

impl Default for AssetLocation {
	fn default() -> Self {
		AssetLocation(MultiLocation::Null)
	}
}

impl Config for Test {
	type AssetId = u32;
	type AssetNativeLocation = AssetLocation;
	type Balance = Balance;
	type Event = Event;
	type NativeAssetId = NativeAssetId;
	type RegistryOrigin = frame_system::EnsureRoot<u64>;
	type StringLimit = RegistryStringLimit;
	type WeightInfo = ();
}
pub type AssetRegistryPallet = crate::Pallet<Test>;

#[derive(Default)]
pub struct ExtBuilder {
	assets: Vec<(Vec<u8>, Balance)>,
	native_asset_name: Option<Vec<u8>>,
}

impl ExtBuilder {
	pub fn with_assets(mut self, assets: Vec<(Vec<u8>, Balance)>) -> Self {
		self.assets = assets;
		self
	}

	pub fn with_native_asset_name(mut self, name: Vec<u8>) -> Self {
		self.native_asset_name = Some(name);
		self
	}

	pub fn build(self) -> sp_io::TestExternalities {
		let mut t = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();

		if let Some(name) = self.native_asset_name {
			crate::GenesisConfig::<Test> {
				asset_names: self.assets,
				native_asset_name: name,
				native_existential_deposit: 1_000_000u128,
			}
		} else {
			crate::GenesisConfig::<Test> {
				asset_names: self.assets,
				..Default::default()
			}
		}
		.assimilate_storage(&mut t)
		.unwrap();
		t.into()
	}
}

pub fn new_test_ext() -> sp_io::TestExternalities {
	let mut ext = ExtBuilder::default().build();
	ext.execute_with(|| System::set_block_number(1));
	ext
}
