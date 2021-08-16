use super::*;
use crate as pallet_hasher;

pub use darkwebb_primitives::hasher::{HasherModule, InstanceHasher};
use frame_support::parameter_types;
use frame_system as system;
use sp_core::H256;
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup},
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
		BN254Poseidon3x5Hasher: pallet_hasher::<Instance1>::{Pallet, Call, Storage, Event<T>},
		BN254Poseidon5x5Hasher: pallet_hasher::<Instance2>::{Pallet, Call, Storage, Event<T>},
		BN254CircomPoseidon3x5Hasher: pallet_hasher::<Instance3>::{Pallet, Call, Storage, Event<T>},
		Balances: pallet_balances::{Pallet, Call, Storage, Event<T>},
	}
);

parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub const SS58Prefix: u8 = 42;
}

impl system::Config for Test {
	type AccountData = pallet_balances::AccountData<u64>;
	type AccountId = u64;
	type BaseCallFilter = ();
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
}

parameter_types! {
	pub const ExistentialDeposit: u64 = 1;
}

impl pallet_balances::Config for Test {
	type AccountStore = System;
	type Balance = u64;
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

impl pallet_hasher::Config<Instance1> for Test {
	type Currency = Balances;
	type Event = Event;
	type ForceOrigin = frame_system::EnsureRoot<u64>;
	type Hasher = darkwebb_primitives::hashing::BN254Poseidon3x5Hasher;
	type MetadataDepositBase = MetadataDepositBase;
	type MetadataDepositPerByte = MetadataDepositPerByte;
	type ParameterDeposit = ParameterDeposit;
	type StringLimit = StringLimit;
}

impl pallet_hasher::Config<Instance2> for Test {
	type Currency = Balances;
	type Event = Event;
	type ForceOrigin = frame_system::EnsureRoot<u64>;
	type Hasher = darkwebb_primitives::hashing::BN254Poseidon5x5Hasher;
	type MetadataDepositBase = MetadataDepositBase;
	type MetadataDepositPerByte = MetadataDepositPerByte;
	type ParameterDeposit = ParameterDeposit;
	type StringLimit = StringLimit;
}

impl pallet_hasher::Config<Instance3> for Test {
	type Currency = Balances;
	type Event = Event;
	type ForceOrigin = frame_system::EnsureRoot<u64>;
	type Hasher = darkwebb_primitives::hashing::BN254CircomPoseidon3x5Hasher;
	type MetadataDepositBase = MetadataDepositBase;
	type MetadataDepositPerByte = MetadataDepositPerByte;
	type ParameterDeposit = ParameterDeposit;
	type StringLimit = StringLimit;
}

pub type BN254Poseidon3x5HasherCall = pallet_hasher::Call<Test, Instance1>;
pub type BN254Poseidon5x5HasherCall = pallet_hasher::Call<Test, Instance2>;
pub type BN254CircomPoseidon3x5HasherCall = pallet_hasher::Call<Test, Instance3>;
// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
	system::GenesisConfig::default().build_storage::<Test>().unwrap().into()
}
