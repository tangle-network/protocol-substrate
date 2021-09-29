#![allow(clippy::zero_prefixed_literal)]

use super::*;
use crate as pallet_mixer;
use codec::Decode;
use pallet_balances::Pallet;
use sp_core::H256;

pub use darkwebb_primitives::hasher::{HasherModule, InstanceHasher};
use darkwebb_primitives::types::ElementTrait;
use frame_support::{parameter_types, traits::Nothing};
use frame_system as system;
use orml_currencies::BasicCurrencyAdapter;
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup},
};

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
		HasherPallet: pallet_hasher::{Pallet, Call, Storage, Event<T>},
		VerifierPallet: pallet_verifier::{Pallet, Call, Storage, Event<T>},
		MerkleTree: pallet_mt::{Pallet, Call, Storage, Event<T>},
		Mixer: pallet_mixer::{Pallet, Call, Storage, Event<T>},
		AssetRegistry: pallet_asset_registry::{Pallet, Call, Storage, Event<T>},
		Currencies: orml_currencies::{Pallet, Call, Event<T>},
		Tokens: orml_tokens::{Pallet, Storage, Call, Event<T>},
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

parameter_types! {
	pub const ParameterDeposit: u64 = 1;
	pub const StringLimit: u32 = 50;
	pub const MetadataDepositBase: u64 = 1;
	pub const MetadataDepositPerByte: u64 = 1;
}

impl pallet_hasher::Config for Test {
	type Currency = Balances;
	type Event = Event;
	type ForceOrigin = frame_system::EnsureRoot<u64>;
	type Hasher = darkwebb_primitives::hashing::BN254CircomPoseidon3x5Hasher;
	type MetadataDepositBase = MetadataDepositBase;
	type MetadataDepositPerByte = MetadataDepositPerByte;
	type ParameterDeposit = ParameterDeposit;
	type StringLimit = StringLimit;
}

parameter_types! {
	pub const TreeDeposit: u64 = 1;
	pub const LeafDepositBase: u64 = 1;
	pub const LeafDepositPerByte: u64 = 1;
	pub const Two: u64 = 2;
	pub const MaxTreeDepth: u8 = 30;
	pub const RootHistorySize: u32 = 1096;
	// 21663839004416932945382355908790599225266501822907911457504978515578255421292
	pub const DefaultZeroElement: Element = Element([
		108, 175, 153, 072, 237, 133, 150, 036,
		226, 065, 231, 118, 015, 052, 027, 130,
		180, 093, 161, 235, 182, 053, 058, 052,
		243, 171, 172, 211, 096, 076, 229, 047,
	]);
	pub const NewDefaultZeroElement: Element = Element([0u8; 32]);
}

#[derive(Debug, Encode, Decode, Default, Copy, Clone, PartialEq, Eq, scale_info::TypeInfo)]
pub struct Element([u8; 32]);

impl ElementTrait for Element {
	fn to_bytes(&self) -> &[u8] {
		&self.0
	}

	fn from_bytes(input: &[u8]) -> Self {
		let mut buf = [0u8; 32];
		buf.copy_from_slice(input);
		Self(buf)
	}
}

impl pallet_mt::Config for Test {
	type Currency = Balances;
	type DataDepositBase = LeafDepositBase;
	type DataDepositPerByte = LeafDepositPerByte;
	type DefaultZeroElement = NewDefaultZeroElement;
	type Element = Element;
	type Event = Event;
	type ForceOrigin = frame_system::EnsureRoot<u64>;
	type Hasher = HasherPallet;
	type LeafIndex = u32;
	type MaxTreeDepth = MaxTreeDepth;
	type RootHistorySize = RootHistorySize;
	type RootIndex = u32;
	type StringLimit = StringLimit;
	type TreeDeposit = TreeDeposit;
	type TreeId = u32;
	type Two = Two;
}

impl pallet_verifier::Config for Test {
	type Currency = Balances;
	type Event = Event;
	type ForceOrigin = frame_system::EnsureRoot<u64>;
	type MetadataDepositBase = MetadataDepositBase;
	type MetadataDepositPerByte = MetadataDepositPerByte;
	type ParameterDeposit = ParameterDeposit;
	type StringLimit = StringLimit;
	type Verifier = darkwebb_primitives::verifying::ArkworksBn254Verifier;
}

parameter_types! {
	pub const NativeAssetId: AssetId = 0;
	pub const RegistryStringLimit: u32 = 10;
}

/// Type for storing the id of an asset.
pub type AssetId = u32;
/// Signed version of Balance
pub type Amount = i128;

impl pallet_asset_registry::Config for Test {
	type AssetId = AssetId;
	type AssetNativeLocation = ();
	type Balance = u128;
	type Event = Event;
	type NativeAssetId = NativeAssetId;
	type RegistryOrigin = frame_system::EnsureRoot<u64>;
	type StringLimit = RegistryStringLimit;
	type WeightInfo = ();
}

/// Tokens Configurations
impl orml_tokens::Config for Test {
	type Amount = Amount;
	type Balance = u128;
	type CurrencyId = AssetId;
	type DustRemovalWhitelist = Nothing;
	type Event = Event;
	type ExistentialDeposits = AssetRegistry;
	type MaxLocks = ();
	type OnDust = ();
	type WeightInfo = ();
}

impl orml_currencies::Config for Test {
	type Event = Event;
	type GetNativeCurrencyId = NativeAssetId;
	type MultiCurrency = Tokens;
	type NativeCurrency = BasicCurrencyAdapter<Test, Balances, Amount, BlockNumber>;
	type WeightInfo = ();
}

parameter_types! {
	pub const MixerPalletId: PalletId = PalletId(*b"py/mixer");
	pub const NativeCurrencyId: AssetId = 0;
}

impl Config for Test {
	type Currency = Currencies;
	type Event = Event;
	type NativeCurrencyId = NativeCurrencyId;
	type PalletId = MixerPalletId;
	type Tree = MerkleTree;
	type Verifier = VerifierPallet;
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
	let mut storage = system::GenesisConfig::default().build_storage::<Test>().unwrap();
	let _ = pallet_balances::GenesisConfig::<Test> {
		balances: vec![(1, 10u128.pow(18)), (2, 20u128.pow(18)), (3, 30u128.pow(18))],
	}
	.assimilate_storage(&mut storage);
	storage.into()
}
