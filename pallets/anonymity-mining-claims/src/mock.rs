use super::*;
// use crate::{self as pallet_anonymity_mining_claims};
use crate as pallet_anonymity_mining_claims;
use crate::Instance1;
use codec::{Decode, Encode, MaxEncodedLen};
use frame_support::{
	parameter_types,
	traits::{Contains, Nothing},
	PalletId,
};
use frame_system as system;
use orml_currencies::{BasicCurrencyAdapter, NativeCurrencyOf};
pub use pallet::*;
use scale_info::TypeInfo;
use serde::{Deserialize, Serialize};
use sp_core::H256;
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, ConstU32, IdentityLookup},
};
use sp_std::convert::{TryFrom, TryInto};
use webb_primitives::{field_ops::ArkworksIntoFieldBn254, verifying::CircomVerifierBn254};
pub use webb_primitives::{
	hasher::{HasherModule, InstanceHasher},
	hashing::ethereum::Keccak256HasherBn254,
	types::runtime::Moment,
	AccountId, ElementTrait,
};

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

pub type Balance = u128;
pub type BlockNumber = u64;
pub type CurrencyId = u32;
pub type ChainId = u64;
pub type AssetId = u32;
pub type Amount = i128;

pub const MILLISECS_PER_BLOCK: u64 = 12000;

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
	pub enum Test where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
		Balances: pallet_balances::{Pallet, Call, Storage, Event<T>},
		// HasherPallet: pallet_hasher::{Pallet, Call, Storage, Event<T>},
		HasherPallet: pallet_hasher::<Instance1>::{Pallet, Call, Storage, Event<T>},
		VAnchorVerifier: pallet_vanchor_verifier::<Instance1>::{Pallet, Call, Storage, Event<T>},
		ClaimsVerifier: pallet_claims_verifier::<Instance1>::{Pallet, Call, Storage, Event<T>},
		LinkableTree: pallet_linkable_tree::<Instance1>::{Pallet, Call, Storage, Event<T>},
		MerkleTree: pallet_mt::<Instance1>::{Pallet, Call, Storage, Event<T>},
		Currencies: orml_currencies::{Pallet, Call},
		Tokens: orml_tokens::{Pallet, Storage, Call, Event<T>},
		AssetRegistry: pallet_asset_registry::{Pallet, Call, Storage, Event<T>},
		VAnchor: pallet_vanchor::<Instance1>::{Pallet, Call, Storage, Event<T>},
		TokenWrapper: pallet_token_wrapper::{Pallet, Call, Storage, Event<T>},
		KeyStorage: pallet_key_storage::{Pallet, Call, Storage, Event<T>, Config<T>},
		AnonymityMiningClaims: pallet_anonymity_mining_claims::<Instance1>::{Pallet, Call, Storage, Event<T>, Config<T>}
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
	type WeightInfo = ();
	type MaxLocks = ();
	type MaxReserves = ();
	type CurrencyHooks = ();
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
	pub const TreeDeposit: u64 = 1;
	pub const LeafDepositBase: u64 = 1;
	pub const LeafDepositPerByte: u64 = 1;
	pub const Two: u64 = 2;
	pub const MaxTreeDepth: u8 = 32;
	pub const RootHistorySize: u32 = 100;
	pub const UnspentRootHistorySize: u32 = 30;
	pub const SpentRootHistorySize: u32 = 30;
	// 21663839004416932945382355908790599225266501822907911457504978515578255421292
	pub const DefaultZeroElement: Element = Element([
		47, 229, 76, 96, 211, 172, 171, 243, 52, 58, 53, 182, 235, 161, 93, 180, 130, 27, 52,
		15, 118, 231, 65, 226, 36, 150, 133, 237, 72, 153, 175, 108,
	]);
}

#[derive(
	Debug,
	Encode,
	Decode,
	Default,
	Copy,
	Clone,
	PartialEq,
	Eq,
	scale_info::TypeInfo,
	Serialize,
	Deserialize,
	MaxEncodedLen,
)]
pub struct Element([u8; 32]);

impl ElementTrait for Element {
	fn to_bytes(&self) -> &[u8] {
		&self.0
	}

	fn from_bytes(input: &[u8]) -> Self {
		let mut buf = [0u8; 32];
		buf.iter_mut().zip(input).for_each(|(a, b)| *a = *b);
		Self(buf)
	}
}

parameter_types! {
	#[derive(Debug, TypeInfo)]
	pub const MaxEdges: u32 = 1000;
	#[derive(Debug, TypeInfo)]
	pub const MaxDefaultHashes: u32 = 1000;
}

type MerkleInstance1 = pallet_mt::Instance1;
impl pallet_mt::Config<MerkleInstance1> for Test {
	type Currency = Balances;
	type DataDepositBase = LeafDepositBase;
	type DataDepositPerByte = LeafDepositPerByte;
	type DefaultZeroElement = DefaultZeroElement;
	type Element = Element;
	type MaxEdges = MaxEdges;
	type MaxDefaultHashes = MaxDefaultHashes;
	type RuntimeEvent = RuntimeEvent;
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
	pub const ParameterDeposit: u64 = 1;
	pub const StringLimit: u32 = 50;
	pub const MetadataDepositBase: u64 = 1;
	pub const MetadataDepositPerByte: u64 = 1;
	pub const PotId: PalletId = PalletId(*b"py/anmin");
	pub const APVanchorTreeId: u32 = 99;
}

parameter_types! {
	pub const NativeCurrencyId: AssetId = 0;
	pub const AnonymityPointsAssetId: AssetId = 1;
	pub const RewardAssetId: AssetId = 2;
	pub const RegistryStringLimit: u32 = 10;
}

parameter_types! {
	#[derive(Debug, TypeInfo, PartialEq, Clone, Eq)]
	pub const MaxAssetIdInPool: u32 = 1000;
}

impl pallet_asset_registry::Config for Test {
	type AssetId = webb_primitives::AssetId;
	type AssetNativeLocation = ();
	type Balance = Balance;
	type RuntimeEvent = RuntimeEvent;
	type NativeAssetId = NativeCurrencyId;
	type MaxAssetIdInPool = MaxAssetIdInPool;
	type RegistryOrigin = frame_system::EnsureRoot<AccountId>;
	type StringLimit = RegistryStringLimit;
	type WeightInfo = ();
}

type VAnchorVerifierInstance1 = pallet_vanchor_verifier::Instance1;
impl pallet_vanchor_verifier::Config<VAnchorVerifierInstance1> for Test {
	type RuntimeEvent = RuntimeEvent;
	type ForceOrigin = frame_system::EnsureRoot<AccountId>;
	type MaxParameterLength = ConstU32<1000>;
	type Verifier = CircomVerifierBn254;
	type WeightInfo = ();
}

type ClaimsVerifierInstance1 = pallet_claims_verifier::Instance1;
impl pallet_claims_verifier::Config<ClaimsVerifierInstance1> for Test {
	type RuntimeEvent = RuntimeEvent;
	type ForceOrigin = frame_system::EnsureRoot<AccountId>;
	type MaxParameterLength = ConstU32<1000>;
	type Verifier = CircomVerifierBn254;
	type WeightInfo = ();
}

type HasherInstance1 = pallet_hasher::Instance1;
impl pallet_hasher::Config<HasherInstance1> for Test {
	type RuntimeEvent = RuntimeEvent;
	type ForceOrigin = frame_system::EnsureRoot<AccountId>;
	type MaxParameterLength = ConstU32<10000>;
	type Hasher = webb_primitives::hashing::ArkworksPoseidonHasherBn254;
	type WeightInfo = ();
}

parameter_types! {
	pub const TokenWrapperPalletId: PalletId = PalletId(*b"py/tkwrp");
	pub const WrappingFeeDivider: u128 = 100;
}

impl pallet_token_wrapper::Config for Test {
	type AssetRegistry = AssetRegistry;
	type Currency = Currencies;
	type RuntimeEvent = RuntimeEvent;
	type PalletId = TokenWrapperPalletId;
	type TreasuryId = TokenWrapperPalletId;
	type WeightInfo = ();
	type ProposalNonce = u32;
	type WrappingFeeDivider = WrappingFeeDivider;
}

parameter_types! {
	pub const HistoryLength: u32 = 30;
	// Substrate standalone chain ID type
	pub const ChainType: [u8; 2] = [2, 0];
	pub const ChainIdentifier: u32 = 5;
}

type LinkableTreeInstance1 = pallet_linkable_tree::Instance1;
impl pallet_linkable_tree::Config<LinkableTreeInstance1> for Test {
	type ChainId = ChainId;
	type ChainType = ChainType;
	type ChainIdentifier = ChainIdentifier;
	type RuntimeEvent = RuntimeEvent;
	type HistoryLength = HistoryLength;
	type Tree = MerkleTree;
	type WeightInfo = ();
}

parameter_types! {
	pub const ProposalLifetime: u64 = 50;
	pub const BridgeAccountId: PalletId = PalletId(*b"dw/bridg");
}

pub type ProposalNonce = u32;
pub type MaintainerNonce = u32;

parameter_types! {
	pub const VAnchorPalletId: PalletId = PalletId(*b"py/vanch");
	pub const MaxFee: Balance = 5;
	pub const MaxExtAmount: Balance = 21;
	pub const MaxCurrencyId: AssetId = AssetId::MAX - 1;
}

type VAnchorInstance1 = pallet_vanchor::Instance1;
impl pallet_vanchor::Config<VAnchorInstance1> for Test {
	type Currency = Currencies;
	type EthereumHasher = Keccak256HasherBn254;
	type RuntimeEvent = RuntimeEvent;
	type IntoField = ArkworksIntoFieldBn254;
	type LinkableTree = LinkableTree;
	type NativeCurrencyId = NativeCurrencyId;
	type PalletId = VAnchorPalletId;
	type MaxFee = MaxFee;
	type MaxExtAmount = MaxExtAmount;
	type MaxCurrencyId = MaxCurrencyId;
	type TokenWrapper = TokenWrapper;
	type PostDepositHook = ();
	type ProposalNonce = u32;
	type VAnchorVerifier = VAnchorVerifier;
	type KeyStorage = KeyStorage;
	type WeightInfo = ();
}

impl pallet_key_storage::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type MaxPubkeyLength = ConstU32<1000>;
	type MaxPubKeyOwners = ConstU32<1000>;
	type WeightInfo = ();
}

type AnonymityMiningClaimsInstance1 = pallet_anonymity_mining_claims::Instance1;
impl pallet_anonymity_mining_claims::Config<AnonymityMiningClaimsInstance1> for Test {
	type RuntimeEvent = RuntimeEvent;
	type PotId = PotId;
	type APVanchorTreeId = APVanchorTreeId;
	type Currency = Currencies;
	type VAnchor = VAnchor;
	type NativeCurrencyId = NativeCurrencyId;
	type AnonymityPointsAssetId = AnonymityPointsAssetId;
	type RewardAssetId = RewardAssetId;
	type UnspentRootHistorySize = UnspentRootHistorySize;
	type SpentRootHistorySize = SpentRootHistorySize;
	type ForceOrigin = frame_system::EnsureRoot<AccountId>;
	type ClaimsVerifier = ClaimsVerifier;
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
	system::GenesisConfig::default().build_storage::<Test>().unwrap().into()
}
