#![allow(clippy::zero_prefixed_literal)]

use crate as pallet_vanchor_handler;
use codec::{Decode, Encode};
use frame_support::{
	assert_ok, ord_parameter_types, parameter_types,
	traits::{Contains, Nothing},
	PalletId,
};
use frame_system as system;
use orml_currencies::{BasicCurrencyAdapter, NativeCurrencyOf};
pub use pallet_balances;
use serde::{Deserialize, Serialize};
use sp_core::H256;
use sp_runtime::{
	testing::Header,
	traits::{AccountIdConversion, BlakeTwo256, IdentityLookup},
};
use sp_std::convert::{TryFrom, TryInto};
use webb_primitives::{
	field_ops::ArkworksIntoFieldBn254, verifying::ArkworksVerifierBn254, webb_proposals::ResourceId,
};
pub use webb_primitives::{hashing::ethereum::Keccak256HasherBn254, ElementTrait, InstanceHasher};
type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

pub type AccountId = u64;
pub type Balance = u128;
pub type BlockNumber = u64;
pub type CurrencyId = u32;
pub type ChainId = u64;
/// Type for storing the id of an asset.
pub type AssetId = u32;
/// Signed version of Balance
pub type Amount = i128;

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
		LinkableTree: pallet_linkable_tree::{Pallet, Call, Storage, Event<T>},
		MerkleTree: pallet_mt::{Pallet, Call, Storage, Event<T>},
		Currencies: orml_currencies::{Pallet, Call},
		Tokens: orml_tokens::{Pallet, Storage, Call, Event<T>},
		AssetRegistry: pallet_asset_registry::{Pallet, Call, Storage, Event<T>},
		VAnchor: pallet_vanchor::{Pallet, Call, Storage, Event<T>},
		VAnchorHandler: pallet_vanchor_handler::{Pallet, Call, Storage, Event<T>},
		SignatureBridge: pallet_signature_bridge::<Instance1>::{Pallet, Call, Storage, Event<T>},
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

ord_parameter_types! {
	pub const One: u64 = 1;
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

parameter_types! {
	pub const ParameterDeposit: u64 = 1;
	pub const StringLimit: u32 = 50;
	pub const MetadataDepositBase: u64 = 1;
	pub const MetadataDepositPerByte: u64 = 1;
}

impl pallet_vanchor_verifier::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type ForceOrigin = frame_system::EnsureRoot<AccountId>;
	type Verifier = ArkworksVerifierBn254;
	type WeightInfo = ();
}

impl pallet_hasher::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type ForceOrigin = frame_system::EnsureRoot<AccountId>;
	type Hasher = webb_primitives::hashing::ArkworksPoseidonHasherBn254;
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

impl pallet_mt::Config for Test {
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
	pub const NativeCurrencyId: AssetId = 0;
	pub const RegistryStringLimit: u32 = 10;
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
	type ReserveIdentifier = [u8; 8];
	type CurrencyHooks = ();
	type MaxReserves = ();
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
	type AssetId = webb_primitives::AssetId;
	type AssetNativeLocation = ();
	type Balance = u128;
	type RuntimeEvent = RuntimeEvent;
	type NativeAssetId = NativeCurrencyId;
	type RegistryOrigin = frame_system::EnsureRoot<u64>;
	type StringLimit = RegistryStringLimit;
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

impl pallet_linkable_tree::Config for Test {
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

pub struct SetResourceProposalFilter;
impl Contains<RuntimeCall> for SetResourceProposalFilter {
	fn contains(c: &RuntimeCall) -> bool {
		match c {
			RuntimeCall::VAnchorHandler(method) => match method {
				pallet_vanchor_handler::Call::execute_set_resource_proposal { .. } => true,
				_ => false,
			},
			_ => false,
		}
	}
}

pub struct ExecuteProposalFilter;
impl Contains<RuntimeCall> for ExecuteProposalFilter {
	fn contains(c: &RuntimeCall) -> bool {
		match c {
			RuntimeCall::VAnchorHandler(method) => match method {
				pallet_vanchor_handler::Call::execute_vanchor_create_proposal { .. } => true,
				pallet_vanchor_handler::Call::execute_vanchor_update_proposal { .. } => true,
				_ => false,
			},
			_ => false,
		}
	}
}

pub type ProposalNonce = u32;
pub type MaintainerNonce = u32;

type BridgeInstance = pallet_signature_bridge::Instance1;
impl pallet_signature_bridge::Config<BridgeInstance> for Test {
	type AdminOrigin = frame_system::EnsureRoot<Self::AccountId>;
	type BridgeAccountId = BridgeAccountId;
	type ChainId = ChainId;
	type ChainIdentifier = ChainIdentifier;
	type ChainType = ChainType;
	type RuntimeEvent = RuntimeEvent;
	type ProposalLifetime = ProposalLifetime;
	type ProposalNonce = ProposalNonce;
	type SetResourceProposalFilter = SetResourceProposalFilter;
	type ExecuteProposalFilter = ExecuteProposalFilter;
	type MaintainerNonce = MaintainerNonce;
	type SignatureVerifier = webb_primitives::signing::SignatureVerifier;
	type WeightInfo = ();
	type Proposal = RuntimeCall;
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

impl pallet_vanchor_handler::Config for Test {
	type VAnchor = VAnchor;
	type BridgeOrigin = pallet_signature_bridge::EnsureBridge<Test, BridgeInstance>;
	type RuntimeEvent = RuntimeEvent;
}

impl pallet_key_storage::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = ();
}

pub const RELAYER_A: u64 = 0x2;
pub const RELAYER_B: u64 = 0x3;
pub const RELAYER_C: u64 = 0x4;
pub const ENDOWED_BALANCE: u128 = 100_000_000;

pub fn new_test_ext() -> sp_io::TestExternalities {
	let bridge_id = PalletId(*b"dw/bridg").into_account_truncating();
	let mut t = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();
	pallet_balances::GenesisConfig::<Test> {
		balances: vec![(bridge_id, ENDOWED_BALANCE), (RELAYER_A, ENDOWED_BALANCE)],
	}
	.assimilate_storage(&mut t)
	.unwrap();
	let mut ext = sp_io::TestExternalities::new(t);
	ext.execute_with(|| System::set_block_number(1));
	ext
}

fn last_event() -> RuntimeEvent {
	system::Pallet::<Test>::events()
		.pop()
		.map(|e| e.event)
		.expect("RuntimeEvent expected")
}

pub fn expect_event<E: Into<RuntimeEvent>>(e: E) {
	assert_eq!(last_event(), e.into());
}

// Asserts that the RuntimeEvent was emitted at some point.
pub fn event_exists<E: Into<RuntimeEvent>>(e: E) {
	let actual: Vec<RuntimeEvent> =
		system::Pallet::<Test>::events().iter().map(|e| e.event.clone()).collect();
	let e: RuntimeEvent = e.into();
	let mut exists = false;
	for evt in actual {
		if evt == e {
			exists = true;
			break
		}
	}
	assert!(exists);
}

// Checks events against the latest. A contiguous set of events must be
// provided. They must include the most recent RuntimeEvent, but do not have to include
// every past RuntimeEvent.
pub fn assert_events(mut expected: Vec<RuntimeEvent>) {
	let mut actual: Vec<RuntimeEvent> =
		system::Pallet::<Test>::events().iter().map(|e| e.event.clone()).collect();

	expected.reverse();

	for evt in expected {
		let next = actual.pop().expect("RuntimeEvent expected");
		assert_eq!(next, evt, "Events don't match");
	}
}

pub fn new_test_ext_initialized(
	src_id: <Test as pallet_signature_bridge::Config<BridgeInstance>>::ChainId,
	r_id: ResourceId,
	_resource: Vec<u8>,
) -> sp_io::TestExternalities {
	let mut t = new_test_ext();
	t.execute_with(|| {
		// Whitelist chain
		assert_ok!(SignatureBridge::whitelist_chain(RuntimeOrigin::root(), src_id));
		// Set and check resource ID mapped to some junk data
		assert_ok!(SignatureBridge::set_resource(RuntimeOrigin::root(), r_id));
		assert!(SignatureBridge::resource_exists(r_id));
	});
	t
}

pub fn new_test_ext_for_set_resource_proposal_initialized(
	src_id: <Test as pallet_signature_bridge::Config<BridgeInstance>>::ChainId,
) -> sp_io::TestExternalities {
	let mut t = new_test_ext();
	t.execute_with(|| {
		// Whitelist chain
		assert_ok!(SignatureBridge::whitelist_chain(RuntimeOrigin::root(), src_id));
	});
	t
}
