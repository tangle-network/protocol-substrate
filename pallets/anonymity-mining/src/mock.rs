use super::*;
use crate::{self as pallet_anonymity_mining};

use frame_support::{parameter_types, traits::Nothing};
use frame_system as system;
use orml_currencies::{BasicCurrencyAdapter, NativeCurrencyOf};
use sp_core::H256;
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup},
};
use sp_std::convert::{TryFrom, TryInto};
pub use webb_primitives::hasher::{HasherModule, InstanceHasher};
use webb_primitives::{AccountId, BlockNumber};

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
	pub enum Test where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
		Balances: pallet_balances::{Pallet, Call, Storage, Event<T>},
		Currencies: orml_currencies::{Pallet, Call},
		Tokens: orml_tokens::{Pallet, Storage, Call, Event<T>},
		AssetRegistry: pallet_asset_registry::{Pallet, Call, Storage, Event<T>},
		AnonymityMining: pallet_anonymity_mining::{Pallet, Call, Storage, Event<T>, Config<T>},
	}
);

parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub const SS58Prefix: u8 = 42;
}

impl system::Config for Test {
	type AccountData = pallet_balances::AccountData<Balance>;
	type AccountId = AccountId;
	type BaseCallFilter = frame_support::traits::Everything;
	type BlockHashCount = BlockHashCount;
	type BlockLength = ();
	type BlockNumber = BlockNumber;
	type BlockWeights = ();
	type RuntimeCall = RuntimeCall;
	type DbWeight = ();
	type RuntimeEvent = RuntimeEvent;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type Header = Header;
	type Index = u64;
	type Lookup = IdentityLookup<Self::AccountId>;
	type MaxConsumers = frame_support::traits::ConstU32<16>;
	type OnKilledAccount = ();
	type OnNewAccount = ();
	type OnSetCode = ();
	type RuntimeOrigin = RuntimeOrigin;
	type PalletInfo = PalletInfo;
	type SS58Prefix = SS58Prefix;
	type SystemWeightInfo = ();
	type Version = ();
}

parameter_types! {
	pub const ExistentialDeposit: u64 = 1;
}

impl pallet_balances::Config for Test {
	type AccountStore = System;
	type Balance = Balance;
	type DustRemoval = ();
	type RuntimeEvent = RuntimeEvent;
	type ExistentialDeposit = ExistentialDeposit;
	type MaxLocks = ();
	type MaxReserves = ();
	type ReserveIdentifier = [u8; 8];
	type WeightInfo = ();
}

/// Tokens Configurations
impl orml_tokens::Config for Test {
	type Amount = Amount;
	type Balance = Balance;
	type CurrencyId = webb_primitives::AssetId;
	type DustRemovalWhitelist = Nothing;
	type RuntimeEvent = RuntimeEvent;
	type ExistentialDeposits = AssetRegistry;
	type OnDust = ();
	type WeightInfo = ();
	type MaxLocks = ();
	type MaxReserves = ();
	type OnSlash = ();
	type OnDeposit = ();
	type OnTransfer = ();
	type OnNewTokenAccount = ();
	type OnKilledTokenAccount = ();
	type ReserveIdentifier = [u8; 8];
}

pub type NativeCurrency = NativeCurrencyOf<Test>;
pub type AdaptedBasicCurrency = BasicCurrencyAdapter<Test, Balances, Amount, BlockNumber>;
impl orml_currencies::Config for Test {
	type MultiCurrency = Tokens;
	type NativeCurrency = AdaptedBasicCurrency;
	type GetNativeCurrencyId = NativeCurrencyId;
	type WeightInfo = ();
}

parameter_types! {
	pub const NativeCurrencyId: AssetId = 0;
	pub const RegistryStringLimit: u32 = 10;
}

/// Type for storing the id of an asset.
pub type AssetId = u32;
/// Signed version of Balance
pub type Amount = i128;
pub type Balance = u128;

impl pallet_asset_registry::Config for Test {
	type AssetId = webb_primitives::AssetId;
	type AssetNativeLocation = ();
	type Balance = Balance;
	type RuntimeEvent = RuntimeEvent;
	type NativeAssetId = NativeCurrencyId;
	type RegistryOrigin = frame_system::EnsureRoot<AccountId>;
	type StringLimit = RegistryStringLimit;
	type WeightInfo = ();
}

parameter_types! {
	pub const ParameterDeposit: u64 = 1;
	pub const StringLimit: u32 = 50;
	pub const MetadataDepositBase: u64 = 1;
	pub const MetadataDepositPerByte: u64 = 1;
	pub const PotId: PalletId = PalletId(*b"py/anmin");
}

impl pallet_anonymity_mining::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type PotId = PotId;
	type Currency = Currencies;
	type NativeCurrencyId = NativeCurrencyId;
	type ForceOrigin = frame_system::EnsureRoot<AccountId>;
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
	system::GenesisConfig::default().build_storage::<Test>().unwrap().into()
}
