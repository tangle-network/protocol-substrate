#![allow(clippy::zero_prefixed_literal)]

use super::*;
use crate as pallet_vanchor;
use sp_core::H256;

use frame_support::{parameter_types, traits::Nothing, PalletId};
use frame_system as system;
use orml_currencies::{BasicCurrencyAdapter, NativeCurrencyOf};
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup},
	Permill,
};
use sp_std::convert::{TryFrom, TryInto};
pub use webb_primitives::{
	field_ops::arkworks::ArkworksIntoFieldBn254,
	hasher::{HasherModule, InstanceHasher},
	hashing::ethereum::Keccak256HasherBn254,
	types::{ElementTrait, IntoAbiToken},
	AccountId, Element,
};
use webb_primitives::{hashing::ArkworksPoseidonHasherBn254, verifying::ArkworksVerifierBn254};

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

pub type Balance = u128;

pub type BlockNumber = u64;
/// Type for storing the id of an asset.
pub type AssetId = u32;
/// Signed version of Balance
pub type Amount = i128;
pub type CurrencyId = u32;
pub type ChainId = u64;

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
		VAnchorVerifier: pallet_vanchor_verifier::{Pallet, Call, Storage, Event<T>},
		MerkleTree: pallet_mt::{Pallet, Call, Storage, Event<T>},
		LinkableTree: pallet_linkable_tree::{Pallet, Call, Storage, Event<T>},
		Currencies: orml_currencies::{Pallet, Call},
		Tokens: orml_tokens::{Pallet, Storage, Call, Event<T>},
		AssetRegistry: pallet_asset_registry::{Pallet, Call, Storage, Event<T>},
		VAnchor: pallet_vanchor::{Pallet, Call, Storage, Event<T>},
		Treasury: pallet_treasury::{Pallet, Call, Storage, Config, Event<T>},
		TokenWrapper: pallet_token_wrapper::{Pallet, Call, Storage, Event<T>},
		KeyStorage: pallet_key_storage::{Pallet, Call, Storage, Event<T>, Config<T>}
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
	pub const ExistentialDeposit: u64 = 1;
}

impl pallet_balances::Config for Test {
	type AccountStore = System;
	type Balance = Balance;
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

impl pallet_vanchor_verifier::Config for Test {
	type Event = Event;
	type ForceOrigin = frame_system::EnsureRoot<AccountId>;
	type Verifier = ArkworksVerifierBn254;
	type WeightInfo = ();
}

impl pallet_hasher::Config for Test {
	type Event = Event;
	type ForceOrigin = frame_system::EnsureRoot<AccountId>;
	type Hasher = ArkworksPoseidonHasherBn254;
	type WeightInfo = ();
}

parameter_types! {
	pub const TreeDeposit: u64 = 1;
	pub const LeafDepositBase: u64 = 1;
	pub const LeafDepositPerByte: u64 = 1;
	pub const Two: u64 = 2;
	pub const MaxTreeDepth: u8 = 32;
	pub const RootHistorySize: u32 = 100;
	// 21663839004416932945382355908790599225266501822907911457504978515578255421292
	pub const DefaultZeroElement: Element = Element([
		47, 229, 76, 96, 211, 172, 171, 243, 52, 58, 53, 182, 235, 161, 93, 180, 130, 27, 52,
		15, 118, 231, 65, 226, 36, 150, 133, 237, 72, 153, 175, 108,
	]);
}

impl pallet_mt::Config for Test {
	type Currency = Balances;
	type DataDepositBase = LeafDepositBase;
	type DataDepositPerByte = LeafDepositPerByte;
	type DefaultZeroElement = DefaultZeroElement;
	type Element = Element;
	type Event = Event;
	type ForceOrigin = frame_system::EnsureRoot<AccountId>;
	type Hasher = HasherPallet;
	type LeafIndex = u32;
	type MaxTreeDepth = MaxTreeDepth;
	type RootHistorySize = RootHistorySize;
	type RootIndex = u32;
	type StringLimit = StringLimit;
	type TreeDeposit = TreeDeposit;
	type TreeId = u32;
	type Two = Two;
	type WeightInfo = ();
}

parameter_types! {
	pub const HistoryLength: u32 = 30;
	// Substrate standalone chain ID type
	pub const ChainType: [u8; 2] = [2, 0];
	pub const ChainIdentifier: ChainId = 1;
}

impl pallet_linkable_tree::Config for Test {
	type ChainId = ChainId;
	type ChainType = ChainType;
	type ChainIdentifier = ChainIdentifier;
	type Event = Event;
	type HistoryLength = HistoryLength;
	type Tree = MerkleTree;
	type WeightInfo = ();
}

parameter_types! {
	pub const NativeCurrencyId: AssetId = 0;
	pub const RegistryStringLimit: u32 = 10;
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
	type MaxReserves = ();
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

impl pallet_asset_registry::Config for Test {
	type AssetId = AssetId;
	type AssetNativeLocation = ();
	type Balance = u128;
	type Event = Event;
	type NativeAssetId = NativeCurrencyId;
	type RegistryOrigin = frame_system::EnsureRoot<AccountId>;
	type StringLimit = RegistryStringLimit;
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
	type ApproveOrigin = frame_system::EnsureRoot<AccountId>;
	type Burn = Burn;
	type SpendOrigin = frame_support::traits::NeverEnsureOrigin<u128>;
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
	type ProposalBondMaximum = ();
}

parameter_types! {
	pub const TokenWrapperPalletId: PalletId = PalletId(*b"py/tkwrp");
	pub const WrappingFeeDivider: u128 = 100;
}

impl pallet_token_wrapper::Config for Test {
	type AssetRegistry = AssetRegistry;
	type Currency = Currencies;
	type Event = Event;
	type PalletId = TokenWrapperPalletId;
	type TreasuryId = TreasuryPalletId;
	type WeightInfo = ();
	type ProposalNonce = u32;
	type WrappingFeeDivider = WrappingFeeDivider;
}

parameter_types! {
	pub const VAnchorPalletId: PalletId = PalletId(*b"py/vanch");
	pub const MaxFee: Balance = 5;
	pub const MaxExtAmount: Balance = 21;
	pub const MaxCurrencyId: AssetId = AssetId::MAX - 1;
}

impl pallet_vanchor::Config for Test {
	type Currency = Currencies;
	type EthereumHasher = Keccak256HasherBn254;
	type Event = Event;
	type IntoField = ArkworksIntoFieldBn254;
	type LinkableTree = LinkableTree;
	type NativeCurrencyId = NativeCurrencyId;
	type PalletId = VAnchorPalletId;
	type MaxFee = MaxFee;
	type MaxExtAmount = MaxExtAmount;
	type MaxCurrencyId = MaxCurrencyId;
	type ProposalNonce = u32;
	type TokenWrapper = TokenWrapper;
	type PostDepositHook = ();
	type VAnchorVerifier = VAnchorVerifier;
	type KeyStorage = KeyStorage;
	type WeightInfo = ();
}

impl pallet_key_storage::Config for Test {
	type Event = Event;
	type WeightInfo = ();
}

pub fn assert_last_event<T: pallet_vanchor::Config>(
	generic_event: <T as pallet_vanchor::Config>::Event,
) {
	frame_system::Pallet::<T>::assert_last_event(generic_event.into());
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
	let t = system::GenesisConfig::default().build_storage::<Test>().unwrap().into();

	let mut ext = sp_io::TestExternalities::new(t);
	ext.execute_with(|| System::set_block_number(1));
	ext
}
