#![allow(clippy::zero_prefixed_literal)]

use super::*;
use crate as pallet_token_wrapper;

use frame_support::{pallet_prelude::GenesisBuild, parameter_types, traits::Nothing, PalletId};
use frame_system as system;
use orml_currencies::{BasicCurrencyAdapter, NativeCurrencyOf};
use sp_core::H256;
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup},
	Permill,
};
use sp_std::convert::{TryFrom, TryInto};

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;
type BlockNumber = u64;

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
	pub enum Test where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
		Balances: pallet_balances::{Pallet, Call, Storage, Event<T>},
		AssetRegistry: asset_registry::{Pallet, Call, Storage, Event<T>},
		Currencies: orml_currencies::{Pallet, Call},
		Tokens: orml_tokens::{Pallet, Storage, Call, Event<T>},
		Treasury: pallet_treasury::{Pallet, Call, Storage, Config, Event<T>},
		TokenWrapper: pallet_token_wrapper::{Pallet, Call, Storage, Event<T>}
	}
);

parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub const SS58Prefix: u8 = 42;
}

impl system::Config for Test {
	type AccountData = pallet_balances::AccountData<u128>;
	type AccountId = u64;
	type BaseCallFilter = frame_support::traits::Everything;
	type BlockHashCount = BlockHashCount;
	type BlockLength = ();
	type BlockNumber = BlockNumber;
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
	type SS58Prefix = SS58Prefix;
	type SystemWeightInfo = ();
	type Version = ();
}

parameter_types! {
	pub const NativeAssetId: AssetId = 0;
	pub const RegistryStringLimit: u32 = 10;
}

/// Type for storing the id of an asset.
pub type AssetId = u32;
/// Signed version of Balance
pub type Amount = i128;
/// Unsigned version of Balance
pub type Balance = u128;

impl asset_registry::Config for Test {
	type AssetId = webb_primitives::AssetId;
	type AssetNativeLocation = ();
	type Balance = u128;
	type Event = Event;
	type NativeAssetId = NativeAssetId;
	type RegistryOrigin = frame_system::EnsureRoot<u64>;
	type StringLimit = RegistryStringLimit;
	type WeightInfo = ();
}

parameter_types! {
	pub const ExistentialDeposit: u64 = 1;
}

impl pallet_balances::Config for Test {
	type AccountStore = System;
	type Balance = u128;
	type DustRemoval = ();
	type Event = Event;
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
	type Event = Event;
	type ExistentialDeposits = AssetRegistry;
	type OnDust = ();
	type WeightInfo = ();
	type MaxLocks = ();
	type OnNewTokenAccount = ();
	type OnKilledTokenAccount = ();
	type MaxReserves = ();
	type ReserveIdentifier = [u8; 8];
}

parameter_types! {
	pub const NativeCurrencyId: AssetId = 0;
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
	type ApproveOrigin = frame_system::EnsureRoot<u64>;
	type Burn = Burn;
	type BurnDestination = ();
	type Currency = pallet_balances::Pallet<Test>;
	type Event = Event;
	type MaxApprovals = MaxApprovals;
	type OnSlash = ();
	type PalletId = TreasuryPalletId;
	type ProposalBond = ProposalBond;
	type ProposalBondMinimum = ProposalBondMinimum;
	type RejectOrigin = frame_system::EnsureRoot<u64>;
	type SpendFunds = ();
	type SpendPeriod = SpendPeriod;
	type SpendOrigin = frame_support::traits::NeverEnsureOrigin<u128>;
	// Just gets burned.
	type WeightInfo = ();
	type ProposalBondMaximum = ();
}

parameter_types! {
	pub const TokenWrapperPalletId: PalletId = PalletId(*b"py/tkwrp");
	pub const WrappingFeeDivider: u128 = 100;
}

impl Config for Test {
	type AssetRegistry = AssetRegistry;
	type Currency = Currencies;
	type Event = Event;
	type PalletId = TokenWrapperPalletId;
	type TreasuryId = TreasuryPalletId;
	type WeightInfo = ();
	type WrappingFeeDivider = WrappingFeeDivider;
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
	let mut storage = system::GenesisConfig::default().build_storage::<Test>().unwrap();
	let _ = pallet_balances::GenesisConfig::<Test> {
		balances: vec![(1, 10u128.pow(18)), (2, 20u128.pow(18)), (3, 30u128.pow(18))],
	}
	.assimilate_storage(&mut storage);

	GenesisBuild::<Test>::assimilate_storage(&pallet_treasury::GenesisConfig, &mut storage)
		.unwrap();
	storage.into()
}
