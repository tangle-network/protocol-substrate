use super::*;
use crate as pallet_smt;
use sp_core::H256;

use frame_support::parameter_types;
use sp_runtime::{
	traits::{BlakeTwo256, IdentityLookup}, testing::Header,
};
use frame_system as system;
pub use darkwebb_primitives::hasher::{
	InstanceHasher, HasherModule
};

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
		HasherPallet: pallet_hasher::{Pallet, Call, Storage, Event<T>},
		Balances: pallet_balances::{Pallet, Call, Storage, Event<T>},
		SMT: pallet_smt::{Pallet, Call, Storage, Event<T>},
	}
);

parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub const SS58Prefix: u8 = 42;
}

impl system::Config for Test {
	type BaseCallFilter = ();
	type BlockWeights = ();
	type BlockLength = ();
	type DbWeight = ();
	type Origin = Origin;
	type Call = Call;
	type Index = u64;
	type BlockNumber = u64;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = u64;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type Event = Event;
	type BlockHashCount = BlockHashCount;
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = pallet_balances::AccountData<u64>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type SS58Prefix = SS58Prefix;
	type OnSetCode = ();
}

parameter_types! {
	pub const ExistentialDeposit: u64 = 1;
}

impl pallet_balances::Config for Test {
	type Balance = u64;
	type DustRemoval = ();
	type Event = Event;
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = System;
	type WeightInfo = ();
	type MaxLocks = ();
	type MaxReserves = ();
	type ReserveIdentifier = [u8; 8];
}

pub struct TestHasher;
impl InstanceHasher for TestHasher {
	fn hash(data: &[u8], _params: &[u8]) -> Result<Vec<u8>, ark_crypto_primitives::Error> {
		return Ok(data.to_vec());
	}
}

parameter_types! {
	pub const ParameterDeposit: u64 = 1;
	pub const StringLimit: u32 = 50;
	pub const MetadataDepositBase: u64 = 1;
	pub const MetadataDepositPerByte: u64 = 1;
}

impl pallet_hasher::Config for Test {
	type Event = Event;
	type Hasher = TestHasher;
	type Currency = Balances;
	type ForceOrigin = frame_system::EnsureRoot<u64>;
	type ParameterDeposit = ParameterDeposit;
	type MetadataDepositBase = MetadataDepositBase;
	type MetadataDepositPerByte = MetadataDepositPerByte;
	type StringLimit = StringLimit;
}

parameter_types! {
	pub const TreeDeposit: u64 = 1;
	pub const LeafDepositBase: u64 = 1;
	pub const LeafDepositPerByte: u64 = 1;
	pub const Two: u64 = 2;
	pub const MaxTreeDepth: u8 = 255;
	pub const RootHistorySize: u32 = 1096;
}

#[derive(Debug, Encode, Decode, Default, Copy, Clone, PartialEq, Eq)]
pub struct Element([u8; 32]);
impl ElementTrait for Element {
	fn to_bytes(&self) -> &[u8] {
		&self.0
	}

	fn from_bytes(mut input: &[u8]) -> Self {
		let mut buf = [0u8; 32];
		let _ = input.read(&mut buf);
		Self(buf)
	}
}

impl Config for Test {
	type Event = Event;
	type TreeId = u32;
	type LeafIndex = u32;
	type RootIndex = u32;
	type Element = Element;
	type Hasher = HasherPallet;
	type Currency = Balances;
	type ForceOrigin = frame_system::EnsureRoot<u64>;
	type TreeDeposit = TreeDeposit;
	type DataDepositBase = LeafDepositBase;
	type DataDepositPerByte = LeafDepositPerByte;
	type Two = Two;
	type RootHistorySize = RootHistorySize;
	type MaxTreeDepth = MaxTreeDepth;
	type StringLimit = StringLimit;
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
	system::GenesisConfig::default().build_storage::<Test>().unwrap().into()
}
