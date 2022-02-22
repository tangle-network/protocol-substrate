#![allow(clippy::from_over_into, non_snake_case)]
#![cfg_attr(not(feature = "std"), no_std)]
#![recursion_limit = "256"]

// Make the WASM binary available.
#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

pub mod constants;
pub use constants::{currency::*, fee::WeightToFee, time::*};
mod weights;

use codec::{Decode, Encode, MaxEncodedLen};
use common::{
	impls::DealWithFees, opaque, AccountId, AuraId, Balance, BlockNumber, Hash, Header, Index,
	Signature, AVERAGE_ON_INITIALIZE_RATIO, MAXIMUM_BLOCK_WEIGHT, NORMAL_DISPATCH_RATIO,
};
use frame_support::{
	construct_runtime, match_type, parameter_types,
	traits::{Contains, EnsureOneOf, Everything, InstanceFilter},
	weights::{
		constants::{BlockExecutionWeight, ExtrinsicBaseWeight},
		DispatchClass, IdentityFee, Weight,
	},
	PalletId, RuntimeDebug,
};
use frame_system::{
	limits::{BlockLength, BlockWeights},
	EnsureRoot,
};
use sp_api::impl_runtime_apis;
use sp_core::{
	crypto::KeyTypeId,
	u32_trait::{_1, _2, _3, _5},
	OpaqueMetadata,
};
use sp_runtime::{
	create_runtime_str, generic, impl_opaque_keys,
	traits::{AccountIdLookup, BlakeTwo256, Block as BlockT},
	transaction_validity::{TransactionSource, TransactionValidity},
	ApplyExtrinsicResult, Perbill, Percent, Permill,
};
use sp_std::prelude::*;
#[cfg(feature = "std")]
use sp_version::NativeVersion;
use sp_version::RuntimeVersion;

#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

#[cfg(any(feature = "std", test))]
pub use sp_runtime::BuildStorage;

use frame_support::traits::Nothing;
use orml_currencies::BasicCurrencyAdapter;
use pallet_xcm::{EnsureXcm, IsMajorityOfBody, XcmPassthrough};
use polkadot_parachain::primitives::Sibling;
use polkadot_runtime_common::{BlockHashCount, RocksDbWeight, SlowAdjustingFeeUpdate};
use xcm::latest::prelude::*;
use xcm_builder::{
	AccountId32Aliases, AllowTopLevelPaidExecutionFrom, AllowUnpaidExecutionFrom, CurrencyAdapter,
	EnsureXcmOrigin, FixedWeightBounds, IsConcrete, LocationInverter, NativeAsset,
	ParentAsSuperuser, ParentIsDefault, RelayChainAsNative, SiblingParachainAsNative,
	SiblingParachainConvertsVia, SignedAccountId32AsNative, SignedToAccountId32,
	SovereignSignedViaLocation, TakeWeightCredit, UsingComponents,
};
use xcm_executor::{Config, XcmExecutor};

use frame_support::traits::ConstU128;
use webb_primitives::{
	hashing::{ArkworksPoseidonHasherBls381, ArkworksPoseidonHasherBn254},
	types::ElementTrait,
	verifying::{ArkworksVerifierBls381, ArkworksVerifierBn254},
	Amount, ChainId,
};

impl_opaque_keys! {
	pub struct SessionKeys {
		pub aura: Aura,
	}
}

/// Wasm binary unwrapped. If built with `SKIP_WASM_BUILD`, the function panics.
#[cfg(feature = "std")]
pub fn wasm_binary_unwrap() -> &'static [u8] {
	WASM_BINARY.expect(
		"Development wasm binary is not available. This means the client is built with \
		 `SKIP_WASM_BUILD` flag and it is only usable for production chains. Please rebuild with \
		 the flag disabled.",
	)
}

#[sp_version::runtime_version]
pub const VERSION: RuntimeVersion = RuntimeVersion {
	spec_name: create_runtime_str!("webb"),
	impl_name: create_runtime_str!("webb"),
	authoring_version: 1,
	spec_version: 3,
	impl_version: 0,
	apis: RUNTIME_API_VERSIONS,
	transaction_version: 1,
	state_version: 1,
};

/// The version information used to identify this runtime when compiled
/// natively.
#[cfg(feature = "std")]
pub fn native_version() -> NativeVersion {
	NativeVersion { runtime_version: VERSION, can_author_with: Default::default() }
}

parameter_types! {
	pub const Version: RuntimeVersion = VERSION;
	pub RuntimeBlockLength: BlockLength =
		BlockLength::max_with_normal_ratio(5 * 1024 * 1024, NORMAL_DISPATCH_RATIO);
	pub RuntimeBlockWeights: BlockWeights = BlockWeights::builder()
		.base_block(BlockExecutionWeight::get())
		.for_class(DispatchClass::all(), |weights| {
			weights.base_extrinsic = ExtrinsicBaseWeight::get();
		})
		.for_class(DispatchClass::Normal, |weights| {
			weights.max_total = Some(NORMAL_DISPATCH_RATIO * MAXIMUM_BLOCK_WEIGHT);
		})
		.for_class(DispatchClass::Operational, |weights| {
			weights.max_total = Some(MAXIMUM_BLOCK_WEIGHT);
			// Operational transactions have some extra reserved space, so that they
			// are included even if block reached `MAXIMUM_BLOCK_WEIGHT`.
			weights.reserved = Some(
				MAXIMUM_BLOCK_WEIGHT - NORMAL_DISPATCH_RATIO * MAXIMUM_BLOCK_WEIGHT
			);
		})
		.avg_block_initialization(AVERAGE_ON_INITIALIZE_RATIO)
		.build_or_panic();
	pub const SS58Prefix: u8 = 2;
}

pub struct BaseFilter;
impl Contains<Call> for BaseFilter {
	fn contains(_c: &Call) -> bool {
		true
	}
}

// Configure FRAME pallets to include in runtime.
impl frame_system::Config for Runtime {
	type AccountData = pallet_balances::AccountData<Balance>;
	type AccountId = AccountId;
	type BaseCallFilter = BaseFilter;
	type BlockHashCount = BlockHashCount;
	type BlockLength = RuntimeBlockLength;
	type BlockNumber = BlockNumber;
	type BlockWeights = RuntimeBlockWeights;
	type Call = Call;
	type DbWeight = RocksDbWeight;
	type Event = Event;
	type Hash = Hash;
	type Hashing = BlakeTwo256;
	type Header = Header;
	type Index = Index;
	type Lookup = AccountIdLookup<AccountId, ()>;
	type MaxConsumers = frame_support::traits::ConstU32<16>;
	type OnKilledAccount = ();
	type OnNewAccount = ();
	type OnSetCode = cumulus_pallet_parachain_system::ParachainSetCode<Self>;
	type Origin = Origin;
	type PalletInfo = PalletInfo;
	type SS58Prefix = SS58Prefix;
	type SystemWeightInfo = ();
	type Version = Version;
}

parameter_types! {
	pub const MinimumPeriod: u64 = SLOT_DURATION / 2;
}

impl pallet_timestamp::Config for Runtime {
	type MinimumPeriod = MinimumPeriod;
	/// A timestamp: milliseconds since the unix epoch.
	type Moment = u64;
	type OnTimestampSet = ();
	type WeightInfo = weights::pallet_timestamp::WeightInfo<Runtime>;
}

parameter_types! {
	pub const MaxAuthorities: u32 = 1_000;
}

impl pallet_aura::Config for Runtime {
	type AuthorityId = AuraId;
	type DisabledValidators = ();
	type MaxAuthorities = MaxAuthorities;
}

impl pallet_session::Config for Runtime {
	type Event = Event;
	type Keys = SessionKeys;
	type NextSessionRotation = ParachainStaking;
	// Essentially just Aura, but lets be pedantic.
	type SessionHandler = <SessionKeys as sp_runtime::traits::OpaqueKeys>::KeyTypeIdProviders;
	type SessionManager = ParachainStaking;
	type ShouldEndSession = ParachainStaking;
	type ValidatorId = <Self as frame_system::Config>::AccountId;
	// we don't have stash and controller, thus we don't need the convert as well.
	type ValidatorIdOf = pallet_parachain_staking::IdentityCollator;
	type WeightInfo = weights::pallet_session::WeightInfo<Runtime>;
}

parameter_types! {
	pub const  BlocksPerRound: u32 = 6 * HOURS;
	pub const  MinBlocksPerRound: u32 = 10;
	/// Collator candidate exits are delayed by 2 rounds
	pub const LeaveCandidatesDelay: u32 = 2;
	/// Nominator exits are delayed by 2 rounds
	pub const LeaveNominatorsDelay: u32 = 2;
	/// Nomination revocations are delayed by 2 rounds
	pub const RevokeNominationDelay: u32 = 2;
	/// Reward payments are delayed by 2 rounds
	pub const RewardPaymentDelay: u32 = 2;
	/// Minimum 8 collators selected per round, default at genesis and minimum forever after
	pub const MinSelectedCandidates: u32 = 2;
	/// Maximum 100 nominators per collator
	pub const MaxNominatorsPerCollator: u32 = 100;
	/// Maximum 100 collators per nominator
	pub const MaxCollatorsPerNominator: u32 = 100;
	/// Default fixed percent a collator takes off the top of due rewards is 20%
	pub const DefaultCollatorCommission: Perbill = Perbill::from_percent(20);
	/// Default percent of inflation set aside for parachain bond every round
	pub const DefaultParachainBondReservePercent: Percent = Percent::from_percent(30);
	/// Minimum stake required to become a collator
	pub const MinCollatorStk: u128 = 2 * CENTS;
	/// Minimum stake required to be reserved to be a candidate
	pub const MinCollatorCandidateStk: u128 = CENTS / 10;
	/// Minimum stake required to be reserved to be a nominator is 5
	pub const MinNominatorStk: u128 = MILLICENTS;
	pub const ParachainStakingPalletId: PalletId = PalletId(*b"dw/pcstk");
}

impl pallet_parachain_staking::Config for Runtime {
	type BlocksPerRound = BlocksPerRound;
	type Currency = Balances;
	type DefaultCollatorCommission = DefaultCollatorCommission;
	type DefaultParachainBondReservePercent = DefaultParachainBondReservePercent;
	type Event = Event;
	type LeaveCandidatesDelay = LeaveCandidatesDelay;
	type LeaveNominatorsDelay = LeaveNominatorsDelay;
	type MaxCollatorsPerNominator = MaxCollatorsPerNominator;
	type MaxNominatorsPerCollator = MaxNominatorsPerCollator;
	type MinBlocksPerRound = MinBlocksPerRound;
	type MinCollatorCandidateStk = MinCollatorCandidateStk;
	type MinCollatorStk = MinCollatorStk;
	type MinNomination = MinNominatorStk;
	type MinNominatorStk = MinNominatorStk;
	type MinSelectedCandidates = MinSelectedCandidates;
	type MonetaryGovernanceOrigin = EnsureRoot<AccountId>;
	type PalletId = ParachainStakingPalletId;
	type RevokeNominationDelay = RevokeNominationDelay;
	type RewardPaymentDelay = RewardPaymentDelay;
	type WeightInfo = pallet_parachain_staking::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
	pub const UncleGenerations: u32 = 0;
}

impl pallet_authorship::Config for Runtime {
	type EventHandler = (ParachainStaking,);
	type FilterUncle = ();
	type FindAuthor = pallet_session::FindAccountFromAuthorIndex<Self, Aura>;
	type UncleGenerations = UncleGenerations;
}

parameter_types! {
	pub const ExistentialDeposit: Balance = EXISTENTIAL_DEPOSIT;
	pub const MaxLocks: u32 = 50;
	pub const MaxReserves: u32 = 50;
}

impl pallet_balances::Config for Runtime {
	type AccountStore = System;
	/// The type for recording an account's balance.
	type Balance = Balance;
	type DustRemoval = ();
	/// The ubiquitous event type.
	type Event = Event;
	type ExistentialDeposit = ExistentialDeposit;
	type MaxLocks = MaxLocks;
	type MaxReserves = MaxReserves;
	type ReserveIdentifier = [u8; 8];
	type WeightInfo = weights::pallet_balances::WeightInfo<Runtime>;
}

parameter_types! {
	pub const TransactionByteFee: Balance = 1;
	pub const OperationalFeeMultiplier: u8 = 5;
}

impl pallet_transaction_payment::Config for Runtime {
	type FeeMultiplierUpdate = SlowAdjustingFeeUpdate<Self>;
	type OnChargeTransaction =
		pallet_transaction_payment::CurrencyAdapter<Balances, DealWithFees<Runtime>>;
	type OperationalFeeMultiplier = OperationalFeeMultiplier;
	type TransactionByteFee = TransactionByteFee;
	type WeightToFee = WeightToFee;
}

parameter_types! {
	// One storage item; key size is 32; value is size 4+4+16+32 bytes = 56 bytes.
	pub const DepositBase: Balance = deposit(1, 88);
	// Additional storage item size of 32 bytes.
	pub const DepositFactor: Balance = deposit(0, 32);
	pub const MaxSignatories: u16 = 100;
}

impl pallet_multisig::Config for Runtime {
	type Call = Call;
	type Currency = Balances;
	type DepositBase = DepositBase;
	type DepositFactor = DepositFactor;
	type Event = Event;
	type MaxSignatories = MaxSignatories;
	type WeightInfo = weights::pallet_multisig::WeightInfo<Runtime>;
}

impl pallet_utility::Config for Runtime {
	type Call = Call;
	type Event = Event;
	type PalletsOrigin = OriginCaller;
	type WeightInfo = weights::pallet_utility::WeightInfo<Runtime>;
}

parameter_types! {
	// One storage item; key size 32, value size 8; .
	pub const ProxyDepositBase: Balance = deposit(1, 40);
	// Additional storage item size of 33 bytes.
	pub const ProxyDepositFactor: Balance = deposit(0, 33);
	pub const MaxProxies: u16 = 32;
	// One storage item; key size 32, value size 16
	pub const AnnouncementDepositBase: Balance = deposit(1, 48);
	pub const AnnouncementDepositFactor: Balance = deposit(0, 66);
	pub const MaxPending: u16 = 32;
}

/// The type used to represent the kinds of proxying allowed.
#[derive(
	Copy,
	Clone,
	Eq,
	PartialEq,
	Ord,
	PartialOrd,
	Encode,
	Decode,
	RuntimeDebug,
	MaxEncodedLen,
	scale_info::TypeInfo,
)]
pub enum ProxyType {
	/// Fully permissioned proxy. Can execute any call on behalf of _proxied_.
	Any,
	/// Can execute any call that does not transfer funds or assets.
	NonTransfer,
	/// Proxy with the ability to reject time-delay proxy announcements.
	CancelProxy,
	/// Assets proxy. Can execute any call from `assets`, **including asset
	/// transfers**.
	Assets,
	/// Owner proxy. Can execute calls related to asset ownership.
	AssetOwner,
	/// Asset manager. Can execute calls related to asset management.
	AssetManager,
	// Collator selection proxy. Can execute calls related to collator
	// selection mechanism.
	Collator,
	/// Can execute calls related related to staking.
	Staking,
}
impl Default for ProxyType {
	fn default() -> Self {
		Self::Any
	}
}
impl InstanceFilter<Call> for ProxyType {
	fn filter(&self, c: &Call) -> bool {
		match self {
			ProxyType::Any => true,
			ProxyType::NonTransfer => !matches!(
				c,
				Call::Balances { .. } | Call::Assets { .. } | Call::ParachainStaking { .. }
			),
			ProxyType::CancelProxy => matches!(
				c,
				Call::Proxy(pallet_proxy::Call::reject_announcement { .. }) |
					Call::Utility { .. } | Call::Multisig { .. }
			),
			ProxyType::Staking => matches!(c, Call::ParachainStaking { .. }),
			ProxyType::Assets => {
				matches!(c, Call::Assets { .. } | Call::Utility { .. } | Call::Multisig { .. })
			},
			ProxyType::AssetOwner => matches!(
				c,
				Call::Assets(pallet_assets::Call::create { .. }) |
					Call::Assets(pallet_assets::Call::destroy { .. }) |
					Call::Assets(pallet_assets::Call::transfer_ownership { .. }) |
					Call::Assets(pallet_assets::Call::set_team { .. }) |
					Call::Assets(pallet_assets::Call::set_metadata { .. }) |
					Call::Assets(pallet_assets::Call::clear_metadata { .. }) |
					Call::Utility { .. } | Call::Multisig { .. }
			),
			ProxyType::AssetManager => matches!(
				c,
				Call::Assets(pallet_assets::Call::mint { .. }) |
					Call::Assets(pallet_assets::Call::burn { .. }) |
					Call::Assets(pallet_assets::Call::freeze { .. }) |
					Call::Assets(pallet_assets::Call::thaw { .. }) |
					Call::Assets(pallet_assets::Call::freeze_asset { .. }) |
					Call::Assets(pallet_assets::Call::thaw_asset { .. }) |
					Call::Utility { .. } | Call::Multisig { .. }
			),
			ProxyType::Collator => matches!(
				c,
				Call::ParachainStaking { .. } | Call::Utility { .. } | Call::Multisig { .. }
			),
		}
	}

	fn is_superset(&self, o: &Self) -> bool {
		match (self, o) {
			(x, y) if x == y => true,
			(ProxyType::Any, _) => true,
			(_, ProxyType::Any) => false,
			(ProxyType::Assets, ProxyType::AssetOwner) => true,
			(ProxyType::Assets, ProxyType::AssetManager) => true,
			_ => false,
		}
	}
}

impl pallet_proxy::Config for Runtime {
	type AnnouncementDepositBase = AnnouncementDepositBase;
	type AnnouncementDepositFactor = AnnouncementDepositFactor;
	type Call = Call;
	type CallHasher = BlakeTwo256;
	type Currency = Balances;
	type Event = Event;
	type MaxPending = MaxPending;
	type MaxProxies = MaxProxies;
	type ProxyDepositBase = ProxyDepositBase;
	type ProxyDepositFactor = ProxyDepositFactor;
	type ProxyType = ProxyType;
	type WeightInfo = weights::pallet_proxy::WeightInfo<Runtime>;
}

parameter_types! {
	pub const AssetDeposit: Balance = UNITS; // 1 UNIT deposit to create asset
	pub const ApprovalDeposit: Balance = EXISTENTIAL_DEPOSIT;
	pub const AssetsStringLimit: u32 = 50;
	/// Key = 32 bytes, Value = 36 bytes (32+1+1+1+1)
	// https://github.com/paritytech/substrate/blob/069917b/frame/assets/src/lib.rs#L257L271
	pub const MetadataDepositBase: Balance = deposit(1, 68);
	pub const MetadataDepositPerByte: Balance = deposit(0, 1);
	pub const ExecutiveBody: BodyId = BodyId::Executive;
}

/// We allow root and the Relay Chain council to execute privileged asset
/// operations.
pub type AssetsForceOrigin =
	EnsureOneOf<EnsureRoot<AccountId>, EnsureXcm<IsMajorityOfBody<KsmLocation, ExecutiveBody>>>;

impl pallet_assets::Config for Runtime {
	type ApprovalDeposit = ApprovalDeposit;
	type AssetAccountDeposit = ConstU128<DOLLARS>;
	type AssetDeposit = AssetDeposit;
	type AssetId = u32;
	type Balance = u128;
	type Currency = Balances;
	type Event = Event;
	type Extra = ();
	type ForceOrigin = EnsureRoot<AccountId>;
	type Freezer = ();
	type MetadataDepositBase = MetadataDepositBase;
	type MetadataDepositPerByte = MetadataDepositPerByte;
	type StringLimit = StringLimit;
	type WeightInfo = pallet_assets::weights::SubstrateWeight<Runtime>;
}

impl pallet_sudo::Config for Runtime {
	type Call = Call;
	type Event = Event;
}

parameter_types! {
	pub const ReservedXcmpWeight: Weight = MAXIMUM_BLOCK_WEIGHT / 4;
	pub const ReservedDmpWeight: Weight = MAXIMUM_BLOCK_WEIGHT / 4;
}

impl cumulus_pallet_parachain_system::Config for Runtime {
	type DmpMessageHandler = DmpQueue;
	type Event = Event;
	type OnSystemEvent = ();
	type OutboundXcmpMessageSource = XcmpQueue;
	type ReservedDmpWeight = ReservedDmpWeight;
	type ReservedXcmpWeight = ReservedXcmpWeight;
	type SelfParaId = parachain_info::Pallet<Runtime>;
	type XcmpMessageHandler = XcmpQueue;
}

impl parachain_info::Config for Runtime {}

impl cumulus_pallet_aura_ext::Config for Runtime {}

parameter_types! {
	pub const KsmLocation: MultiLocation = MultiLocation::parent();
	pub const RelayNetwork: NetworkId = NetworkId::Kusama;
	pub RelayChainOrigin: Origin = cumulus_pallet_xcm::Origin::Relay.into();
	pub Ancestry: MultiLocation = Parachain(ParachainInfo::parachain_id().into()).into();
}

/// Type for specifying how a `MultiLocation` can be converted into an
/// `AccountId`. This is used when determining ownership of accounts for asset
/// transacting and when attempting to use XCM `Transact` in order to determine
/// the dispatch Origin.
pub type LocationToAccountId = (
	// The parent (Relay-chain) origin converts to the default `AccountId`.
	ParentIsDefault<AccountId>,
	// Sibling parachain origins convert to AccountId via the `ParaId::into`.
	SiblingParachainConvertsVia<Sibling, AccountId>,
	// Straight up local `AccountId32` origins just alias directly to
	// `AccountId`.
	AccountId32Aliases<RelayNetwork, AccountId>,
);

/// Means for transacting assets on this chain.
pub type LocalAssetTransactor = CurrencyAdapter<
	// Use this currency:
	Balances,
	// Use this currency when it is a fungible asset matching the given
	// location or name:
	IsConcrete<KsmLocation>,
	// Do a simple punn to convert an AccountId32 MultiLocation into a native
	// chain account ID:
	LocationToAccountId,
	// Our chain's account ID type (we can't get away without mentioning it
	// explicitly):
	AccountId,
	// We don't track any teleports.
	(),
>;

/// This is the type we use to convert an (incoming) XCM origin into a local
/// `Origin` instance, ready for dispatching a transaction with Xcm's
/// `Transact`. There is an `OriginKind` which can biases the kind of local
/// `Origin` it will become.
pub type XcmOriginToTransactDispatchOrigin = (
	// Sovereign account converter; this attempts to derive an `AccountId` from
	// the origin location using `LocationToAccountId` and then turn that into
	// the usual `Signed` origin. Useful for foreign chains who want to have a
	// local sovereign account on this chain which they control.
	SovereignSignedViaLocation<LocationToAccountId, Origin>,
	// Native converter for Relay-chain (Parent) location; will converts to a
	// `Relay` origin when recognised.
	RelayChainAsNative<RelayChainOrigin, Origin>,
	// Native converter for sibling Parachains; will convert to a `SiblingPara`
	// origin when recognised.
	SiblingParachainAsNative<cumulus_pallet_xcm::Origin, Origin>,
	// Superuser converter for the Relay-chain (Parent) location. This will
	// allow it to issue a transaction from the Root origin.
	ParentAsSuperuser<Origin>,
	// Native signed account converter; this just converts an `AccountId32`
	// origin into a normal `Origin::Signed` origin of the same 32-byte value.
	SignedAccountId32AsNative<RelayNetwork, Origin>,
	// Xcm origins can be represented natively under the Xcm pallet's Xcm
	// origin.
	XcmPassthrough<Origin>,
);

parameter_types! {
	// One XCM operation is 1_000_000_000 weight - almost certainly a conservative estimate.
	pub UnitWeightCost: Weight = 1_000_000_000;
	pub const MaxInstructions: u32 = 100;
}

match_type! {
	pub type ParentOrParentsExecutivePlurality: impl Contains<MultiLocation> = {
		MultiLocation { parents: 1, interior: Here } |
		MultiLocation { parents: 1, interior: X1(Plurality { id: BodyId::Executive, .. }) }
	};
}

pub type Barrier = (
	TakeWeightCredit,
	AllowTopLevelPaidExecutionFrom<Everything>,
	AllowUnpaidExecutionFrom<ParentOrParentsExecutivePlurality>,
	AllowUnpaidExecutionFrom<Everything>,
	// ^^^ Parent and its exec plurality get free execution
);

pub struct XcmConfig;
impl Config for XcmConfig {
	type AssetClaims = PolkadotXcm;
	// How to withdraw and deposit an asset.
	type AssetTransactor = LocalAssetTransactor;
	type AssetTrap = PolkadotXcm;
	type Barrier = Barrier;
	type Call = Call;
	type IsReserve = NativeAsset;
	type IsTeleporter = NativeAsset;
	// <- should be enough to allow teleportation of KSM
	type LocationInverter = LocationInverter<Ancestry>;
	type OriginConverter = XcmOriginToTransactDispatchOrigin;
	type ResponseHandler = PolkadotXcm;
	type SubscriptionService = PolkadotXcm;
	type Trader = UsingComponents<IdentityFee<Balance>, KsmLocation, AccountId, Balances, ()>;
	type Weigher = FixedWeightBounds<UnitWeightCost, Call, MaxInstructions>;
	type XcmSender = XcmRouter;
}

parameter_types! {
	pub const MaxDownwardMessageWeight: Weight = MAXIMUM_BLOCK_WEIGHT / 10;
}

/// No local origins on this chain are allowed to dispatch XCM sends/executions.
pub type LocalOriginToLocation = SignedToAccountId32<Origin, AccountId, RelayNetwork>;

/// The means for routing XCM messages which are not for local execution into
/// the right message queues.
pub type XcmRouter = (
	// Two routers - use UMP to communicate with the relay chain:
	cumulus_primitives_utility::ParentAsUmp<ParachainSystem, ()>,
	// ..and XCMP to communicate with the sibling chains.
	XcmpQueue,
);

impl pallet_xcm::Config for Runtime {
	type AdvertisedXcmVersion = pallet_xcm::CurrentXcmVersion;
	type Call = Call;
	type Event = Event;
	type ExecuteXcmOrigin = EnsureXcmOrigin<Origin, LocalOriginToLocation>;
	type LocationInverter = LocationInverter<Ancestry>;
	type Origin = Origin;
	type SendXcmOrigin = EnsureXcmOrigin<Origin, LocalOriginToLocation>;
	type Weigher = FixedWeightBounds<UnitWeightCost, Call, MaxInstructions>;
	type XcmExecuteFilter = Everything;
	type XcmExecutor = XcmExecutor<XcmConfig>;
	type XcmReserveTransferFilter = Everything;
	type XcmRouter = XcmRouter;
	type XcmTeleportFilter = Everything;

	const VERSION_DISCOVERY_QUEUE_SIZE: u32 = 100;
}

impl cumulus_pallet_xcm::Config for Runtime {
	type Event = Event;
	type XcmExecutor = XcmExecutor<XcmConfig>;
}

impl cumulus_pallet_xcmp_queue::Config for Runtime {
	type ChannelInfo = ParachainSystem;
	type Event = Event;
	type ExecuteOverweightOrigin = EnsureRoot<AccountId>;
	type VersionWrapper = ();
	type XcmExecutor = XcmExecutor<XcmConfig>;
}

impl cumulus_pallet_dmp_queue::Config for Runtime {
	type Event = Event;
	type ExecuteOverweightOrigin = frame_system::EnsureRoot<AccountId>;
	type XcmExecutor = XcmExecutor<XcmConfig>;
}

parameter_types! {
	pub const StringLimit: u32 = 50;
}

impl pallet_hasher::Config<pallet_hasher::Instance1> for Runtime {
	type Event = Event;
	type ForceOrigin = frame_system::EnsureRoot<AccountId>;
	type Hasher = ArkworksPoseidonHasherBn254;
	type WeightInfo = pallet_hasher::weights::WebbWeight<Runtime>;
}

impl pallet_hasher::Config<pallet_hasher::Instance2> for Runtime {
	type Event = Event;
	type ForceOrigin = frame_system::EnsureRoot<AccountId>;
	type Hasher = ArkworksPoseidonHasherBls381;
	type WeightInfo = pallet_hasher::weights::WebbWeight<Runtime>;
}

impl pallet_randomness_collective_flip::Config for Runtime {}

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
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
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

impl pallet_mt::Config<pallet_mt::Instance1> for Runtime {
	type Currency = Balances;
	type DataDepositBase = LeafDepositBase;
	type DataDepositPerByte = LeafDepositPerByte;
	type DefaultZeroElement = NewDefaultZeroElement;
	type Element = Element;
	type Event = Event;
	type ForceOrigin = frame_system::EnsureRoot<AccountId>;
	type Hasher = HasherBn254;
	type LeafIndex = u32;
	type MaxTreeDepth = MaxTreeDepth;
	type RootHistorySize = RootHistorySize;
	type RootIndex = u32;
	type StringLimit = StringLimit;
	type TreeDeposit = TreeDeposit;
	type TreeId = u32;
	type Two = Two;
	type WeightInfo = pallet_mt::weights::WebbWeight<Runtime>;
}

impl pallet_mt::Config<pallet_mt::Instance2> for Runtime {
	type Currency = Balances;
	type DataDepositBase = LeafDepositBase;
	type DataDepositPerByte = LeafDepositPerByte;
	type DefaultZeroElement = NewDefaultZeroElement;
	type Element = Element;
	type Event = Event;
	type ForceOrigin = frame_system::EnsureRoot<AccountId>;
	type Hasher = HasherBls381;
	type LeafIndex = u32;
	type MaxTreeDepth = MaxTreeDepth;
	type RootHistorySize = RootHistorySize;
	type RootIndex = u32;
	type StringLimit = StringLimit;
	type TreeDeposit = TreeDeposit;
	type TreeId = u32;
	type Two = Two;
	type WeightInfo = pallet_mt::weights::WebbWeight<Runtime>;
}

impl pallet_verifier::Config<pallet_verifier::Instance1> for Runtime {
	type Event = Event;
	type ForceOrigin = frame_system::EnsureRoot<AccountId>;
	type Verifier = ArkworksVerifierBn254;
	type WeightInfo = pallet_verifier::weights::WebbWeight<Runtime>;
}

impl pallet_verifier::Config<pallet_verifier::Instance2> for Runtime {
	type Event = Event;
	type ForceOrigin = frame_system::EnsureRoot<AccountId>;
	type Verifier = ArkworksVerifierBls381;
	type WeightInfo = pallet_verifier::weights::WebbWeight<Runtime>;
}

impl pallet_verifier::Config<pallet_verifier::Instance3> for Runtime {
	type Event = Event;
	type ForceOrigin = frame_system::EnsureRoot<AccountId>;
	type Verifier = ArkworksVerifierBn254;
	type WeightInfo = pallet_verifier::weights::WebbWeight<Runtime>;
}

impl pallet_verifier::Config<pallet_verifier::Instance4> for Runtime {
	type Event = Event;
	type ForceOrigin = frame_system::EnsureRoot<AccountId>;
	type Verifier = ArkworksVerifierBls381;
	type WeightInfo = pallet_verifier::weights::WebbWeight<Runtime>;
}

impl pallet_asset_registry::Config for Runtime {
	type AssetId = webb_primitives::AssetId;
	type AssetNativeLocation = ();
	type Balance = Balance;
	type Event = Event;
	type NativeAssetId = NativeCurrencyId;
	type RegistryOrigin = frame_system::EnsureRoot<AccountId>;
	type StringLimit = RegistryStringLimit;
	type WeightInfo = ();
}

impl orml_tokens::Config for Runtime {
	type Amount = Amount;
	type Balance = Balance;
	type CurrencyId = webb_primitives::AssetId;
	type DustRemovalWhitelist = Nothing;
	type Event = Event;
	type ExistentialDeposits = AssetRegistry;
	type MaxLocks = ();
	type OnDust = ();
	type WeightInfo = ();
}

impl orml_currencies::Config for Runtime {
	type Event = Event;
	type GetNativeCurrencyId = NativeCurrencyId;
	type MultiCurrency = Tokens;
	type NativeCurrency = BasicCurrencyAdapter<Runtime, Balances, Amount, BlockNumber>;
	type WeightInfo = ();
}

parameter_types! {
	pub const MixerPalletId: PalletId = PalletId(*b"py/mixer");
	pub const NativeCurrencyId: u32 = 0;
	pub const RegistryStringLimit: u32 = 10;
}

impl pallet_mixer::Config<pallet_mixer::Instance1> for Runtime {
	type Currency = Currencies;
	type Event = Event;
	type NativeCurrencyId = NativeCurrencyId;
	type PalletId = MixerPalletId;
	type Tree = MerkleTreeBn254;
	type Verifier = MixerVerifierBn254;
	type WeightInfo = pallet_mixer::weights::WebbWeight<Runtime>;
}

impl pallet_mixer::Config<pallet_mixer::Instance2> for Runtime {
	type Currency = Currencies;
	type Event = Event;
	type NativeCurrencyId = NativeCurrencyId;
	type PalletId = MixerPalletId;
	type Tree = MerkleTreeBls381;
	type Verifier = MixerVerifierBls381;
	type WeightInfo = pallet_mixer::weights::WebbWeight<Runtime>;
}

parameter_types! {
	pub const AnchorPalletId: PalletId = PalletId(*b"py/anchr");
	pub const HistoryLength: u32 = 30;
	// Substrate parachain chain ID type
	pub const ChainType: [u8; 2] = [2, 1];
	pub const ChainIdentifier: ChainId = 1080;
}

impl pallet_linkable_tree::Config<pallet_linkable_tree::Instance1> for Runtime {
	type ChainId = ChainId;
	type ChainType = ChainType;
	type ChainIdentifier = ChainIdentifier;
	type Event = Event;
	type HistoryLength = HistoryLength;
	type Tree = MerkleTreeBn254;
	type WeightInfo = ();
}

impl pallet_linkable_tree::Config<pallet_linkable_tree::Instance2> for Runtime {
	type ChainId = ChainId;
	type ChainType = ChainType;
	type ChainIdentifier = ChainIdentifier;
	type Event = Event;
	type HistoryLength = HistoryLength;
	type Tree = MerkleTreeBls381;
	type WeightInfo = ();
}

impl pallet_anchor::Config<pallet_anchor::Instance1> for Runtime {
	type Currency = Currencies;
	type Event = Event;
	type LinkableTree = LinkableTreeBn254;
	type NativeCurrencyId = NativeCurrencyId;
	type PalletId = AnchorPalletId;
	type PostDepositHook = ();
	type Verifier = AnchorVerifierBn254;
	type WeightInfo = pallet_anchor::weights::WebbWeight<Runtime>;
}

impl pallet_anchor::Config<pallet_anchor::Instance2> for Runtime {
	type Currency = Currencies;
	type Event = Event;
	type LinkableTree = LinkableTreeBls381;
	type NativeCurrencyId = NativeCurrencyId;
	type PalletId = AnchorPalletId;
	type PostDepositHook = ();
	type Verifier = AnchorVerifierBls381;
	type WeightInfo = pallet_anchor::weights::WebbWeight<Runtime>;
}

impl pallet_anchor_handler::Config<pallet_anchor_handler::Instance1> for Runtime {
	type Anchor = AnchorBn254;
	type BridgeOrigin = pallet_bridge::EnsureBridge<Runtime, BridgeInstance>;
	type Event = Event;
}

impl pallet_anchor_handler::Config<pallet_anchor_handler::Instance2> for Runtime {
	type Anchor = AnchorBls381;
	type BridgeOrigin = pallet_bridge::EnsureBridge<Runtime, BridgeInstance>;
	type Event = Event;
}

parameter_types! {
	pub const ProposalLifetime: BlockNumber = 50;
	pub const BridgeAccountId: PalletId = PalletId(*b"dw/bridg");
}

type BridgeInstance = pallet_bridge::Instance1;
impl pallet_bridge::Config<BridgeInstance> for Runtime {
	type AdminOrigin = frame_system::EnsureRoot<Self::AccountId>;
	type BridgeAccountId = BridgeAccountId;
	type ChainId = ChainId;
	type ChainIdentifier = ChainIdentifier;
	type ChainType = ChainType;
	type Event = Event;
	type Proposal = Call;
	type ProposalLifetime = ProposalLifetime;
}

impl pallet_hello::Config for Runtime {
	type Call = Call;
	type Event = Event;
	type Origin = Origin;
	type XcmSender = XcmRouter;
}

parameter_types! {
	pub const CouncilMotionDuration: BlockNumber = 5 * DAYS;
	pub const CouncilMaxProposals: u32 = 100;
	pub const CouncilMaxMembers: u32 = 100;
}

type CouncilCollective = pallet_collective::Instance1;
impl pallet_collective::Config<CouncilCollective> for Runtime {
	type DefaultVote = pallet_collective::PrimeDefaultVote;
	type Event = Event;
	type MaxMembers = CouncilMaxMembers;
	type MaxProposals = CouncilMaxProposals;
	type MotionDuration = CouncilMotionDuration;
	type Origin = Origin;
	type Proposal = Call;
	type WeightInfo = pallet_collective::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
	pub const ProposalBond: Permill = Permill::from_percent(5);
	pub const ProposalBondMinimum: Balance = 1 * DOLLARS;
	pub const SpendPeriod: BlockNumber = 1 * DAYS;
	pub const Burn: Permill = Permill::from_percent(50);
	pub const TipCountdown: BlockNumber = 1 * DAYS;
	pub const TipFindersFee: Percent = Percent::from_percent(20);
	pub const TipReportDepositBase: Balance = 1 * DOLLARS;
	pub const DataDepositPerByte: Balance = 1 * CENTS;
	pub const BountyDepositBase: Balance = 1 * DOLLARS;
	pub const BountyDepositPayoutDelay: BlockNumber = 1 * DAYS;
	pub const TreasuryPalletId: PalletId = PalletId(*b"py/trsry");
	pub const BountyUpdatePeriod: BlockNumber = 14 * DAYS;
	pub const MaximumReasonLength: u32 = 300;
	pub const BountyCuratorDeposit: Permill = Permill::from_percent(50);
	pub const BountyValueMinimum: Balance = 5 * DOLLARS;
	pub const MaxApprovals: u32 = 100;
	pub const MaxActiveChildBountyCount: u32 = 5;
	pub const ChildBountyValueMinimum: Balance = 1 * DOLLARS;
	pub const ChildBountyCuratorDepositBase: Permill = Permill::from_percent(10);
}

impl pallet_treasury::Config for Runtime {
	type ApproveOrigin = EnsureOneOf<
		EnsureRoot<AccountId>,
		pallet_collective::EnsureProportionAtLeast<_3, _5, AccountId, CouncilCollective>,
	>;
	type Burn = Burn;
	type BurnDestination = ();
	type Currency = Balances;
	type Event = Event;
	type MaxApprovals = MaxApprovals;
	type OnSlash = ();
	type PalletId = TreasuryPalletId;
	type ProposalBond = ProposalBond;
	type ProposalBondMinimum = ProposalBondMinimum;
	type RejectOrigin = EnsureOneOf<
		EnsureRoot<AccountId>,
		pallet_collective::EnsureProportionMoreThan<_1, _2, AccountId, CouncilCollective>,
	>;
	type SpendFunds = Bounties;
	type SpendPeriod = SpendPeriod;
	type WeightInfo = pallet_treasury::weights::SubstrateWeight<Runtime>;
}

impl pallet_bounties::Config for Runtime {
	type BountyCuratorDeposit = BountyCuratorDeposit;
	type BountyDepositBase = BountyDepositBase;
	type BountyDepositPayoutDelay = BountyDepositPayoutDelay;
	type BountyUpdatePeriod = BountyUpdatePeriod;
	type BountyValueMinimum = BountyValueMinimum;
	type ChildBountyManager = ChildBounties;
	type DataDepositPerByte = DataDepositPerByte;
	type Event = Event;
	type MaximumReasonLength = MaximumReasonLength;
	type WeightInfo = pallet_bounties::weights::SubstrateWeight<Runtime>;
}

impl pallet_child_bounties::Config for Runtime {
	type ChildBountyCuratorDepositBase = ChildBountyCuratorDepositBase;
	type ChildBountyValueMinimum = ChildBountyValueMinimum;
	type Event = Event;
	type MaxActiveChildBountyCount = MaxActiveChildBountyCount;
	type WeightInfo = pallet_child_bounties::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
	pub const TokenWrapperPalletId: PalletId = PalletId(*b"dw/tkwrp");
	pub const WrappingFeeDivider: Balance = 100;
}

impl pallet_token_wrapper::Config for Runtime {
	type AssetRegistry = AssetRegistry;
	type Currency = Currencies;
	type Event = Event;
	type PalletId = TokenWrapperPalletId;
	type TreasuryId = TreasuryPalletId;
	type WeightInfo = pallet_token_wrapper::weights::WebbWeight<Runtime>;
	type WrappingFeeDivider = WrappingFeeDivider;
}

// Create the runtime by composing the FRAME pallets that were previously
// configured.
construct_runtime!(
	pub enum Runtime where
		Block = Block,
		NodeBlock = opaque::Block,
		UncheckedExtrinsic = UncheckedExtrinsic
	{
		// System support stuff.
		System: frame_system,
		ParachainSystem: cumulus_pallet_parachain_system::{
			Pallet, Call, Config, Storage, Inherent, Event<T>, ValidateUnsigned,
		} = 1,
		RandomnessCollectiveFlip: pallet_randomness_collective_flip::{Pallet, Storage} = 2,
		Timestamp: pallet_timestamp::{Pallet, Call, Storage, Inherent} = 3,
		ParachainInfo: parachain_info::{Pallet, Storage, Config} = 4,

		// Monetary stuff.
		Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>} = 10,
		TransactionPayment: pallet_transaction_payment::{Pallet, Storage} = 11,
		Treasury: pallet_treasury::{Pallet, Call, Storage, Config, Event<T>},

		// Collator support. the order of these 4 are important and shall not change.
		Authorship: pallet_authorship::{Pallet, Call, Storage} = 20,
		ParachainStaking: pallet_parachain_staking::{Pallet, Call, Storage, Event<T>, Config<T>} = 21,
		Session: pallet_session::{Pallet, Call, Storage, Event, Config<T>} = 22,
		Aura: pallet_aura::{Pallet, Storage, Config<T>} = 23,
		AuraExt: cumulus_pallet_aura_ext::{Pallet, Storage, Config} = 24,

		// XCM helpers.
		XcmpQueue: cumulus_pallet_xcmp_queue::{Pallet, Call, Storage, Event<T>} = 30,
		PolkadotXcm: pallet_xcm::{Pallet, Call, Event<T>, Origin} = 31,
		CumulusXcm: cumulus_pallet_xcm::{Pallet, Event<T>, Origin} = 32,
		DmpQueue: cumulus_pallet_dmp_queue::{Pallet, Call, Storage, Event<T>} = 33,

		// Handy utilities.
		Utility: pallet_utility::{Pallet, Call, Event} = 40,
		Multisig: pallet_multisig::{Pallet, Call, Storage, Event<T>} = 41,
		Proxy: pallet_proxy::{Pallet, Call, Storage, Event<T>} = 42,

		// The main stage. To include pallet-assets-freezer and pallet-uniques.
		Assets: pallet_assets::{Pallet, Call, Storage, Event<T>} = 50,
		Sudo: pallet_sudo::{Pallet, Call, Config<T>, Storage, Event<T>},
		Bounties: pallet_bounties::{Pallet, Call, Storage, Event<T>},
		ChildBounties: pallet_child_bounties::{Pallet, Call, Storage, Event<T>},

		// Hasher pallet
		HasherBn254: pallet_hasher::<Instance1>::{Pallet, Call, Storage, Event<T>, Config<T>},
		HasherBls381: pallet_hasher::<Instance2>::{Pallet, Call, Storage, Event<T>, Config<T>},

		AssetRegistry: pallet_asset_registry::{Pallet, Call, Storage, Event<T>, Config<T>},
		Currencies: orml_currencies::{Pallet, Call, Event<T>},
		Tokens: orml_tokens::{Pallet, Storage, Call, Event<T>},
		TokenWrapper: pallet_token_wrapper::{Pallet, Storage, Call, Event<T>},

		// Mixer Verifier
		MixerVerifierBn254: pallet_verifier::<Instance1>::{Pallet, Call, Storage, Event<T>, Config<T>},
		MixerVerifierBls381: pallet_verifier::<Instance2>::{Pallet, Call, Storage, Event<T>, Config<T>},

		// Anchor Verifier
		AnchorVerifierBn254: pallet_verifier::<Instance3>::{Pallet, Call, Storage, Event<T>, Config<T>},
		AnchorVerifierBls381: pallet_verifier::<Instance4>::{Pallet, Call, Storage, Event<T>, Config<T>},

		// Merkle Tree
		MerkleTreeBn254: pallet_mt::<Instance1>::{Pallet, Call, Storage, Event<T>, Config<T>},
		MerkleTreeBls381: pallet_mt::<Instance2>::{Pallet, Call, Storage, Event<T>, Config<T>},

		// Linkable Merkle Tree
		LinkableTreeBn254: pallet_linkable_tree::<Instance1>::{Pallet, Call, Storage, Event<T>},
		LinkableTreeBls381: pallet_linkable_tree::<Instance2>::{Pallet, Call, Storage, Event<T>},

		// Mixer
		MixerBn254: pallet_mixer::<Instance1>::{Pallet, Call, Storage, Event<T>, Config<T>},
		MixerBls381: pallet_mixer::<Instance2>::{Pallet, Call, Storage, Event<T>},

		// Anchor
		AnchorBn254: pallet_anchor::<Instance1>::{Pallet, Call, Storage, Event<T>},
		AnchorBls381: pallet_anchor::<Instance2>::{Pallet, Call, Storage, Event<T>},

		// Anchor Handler
		AnchorHandlerBn254: pallet_anchor_handler::<Instance1>::{Pallet, Call, Storage, Event<T>},
		AnchorHandlerBls381: pallet_anchor_handler::<Instance2>::{Pallet, Call, Storage, Event<T>},

		// Bridge
		Bridge: pallet_bridge::<Instance1>::{Pallet, Call, Storage, Event<T>},
		HelloXcm: pallet_hello::{Pallet, Call, Storage, Event<T>},

		Council: pallet_collective::<Instance1>::{Pallet, Call, Storage, Origin<T>, Event<T>, Config<T>}
	}
);

/// The address format for describing accounts.
pub type Address = sp_runtime::MultiAddress<AccountId, ()>;
/// Block type as expected by this runtime.
pub type Block = generic::Block<Header, UncheckedExtrinsic>;
/// The SignedExtension to the basic transaction logic.
pub type SignedExtra = (
	frame_system::CheckSpecVersion<Runtime>,
	frame_system::CheckTxVersion<Runtime>,
	frame_system::CheckGenesis<Runtime>,
	frame_system::CheckEra<Runtime>,
	frame_system::CheckNonce<Runtime>,
	frame_system::CheckWeight<Runtime>,
	pallet_transaction_payment::ChargeTransactionPayment<Runtime>,
);
/// Unchecked extrinsic type as expected by this runtime.
pub type UncheckedExtrinsic = generic::UncheckedExtrinsic<Address, Call, Signature, SignedExtra>;
/// Extrinsic type that has already been checked.
pub type CheckedExtrinsic = generic::CheckedExtrinsic<AccountId, Call, SignedExtra>;
/// Executive: handles dispatch to the various modules.
pub type Executive = frame_executive::Executive<
	Runtime,
	Block,
	frame_system::ChainContext<Runtime>,
	Runtime,
	AllPalletsWithoutSystemReversed,
	OnRuntimeUpgrade,
>;

pub struct OnRuntimeUpgrade;
impl frame_support::traits::OnRuntimeUpgrade for OnRuntimeUpgrade {
	fn on_runtime_upgrade() -> u64 {
		frame_support::migrations::migrate_from_pallet_version_to_storage_version::<
			AllPalletsWithSystem,
		>(&RocksDbWeight::get())
	}
}

impl_runtime_apis! {
	impl sp_consensus_aura::AuraApi<Block, AuraId> for Runtime {
		fn slot_duration() -> sp_consensus_aura::SlotDuration {
			sp_consensus_aura::SlotDuration::from_millis(Aura::slot_duration())
		}

		fn authorities() -> Vec<AuraId> {
			Aura::authorities().into_inner()
		}
	}

	impl sp_api::Core<Block> for Runtime {
		fn version() -> RuntimeVersion {
			VERSION
		}

		fn execute_block(block: Block) {
			Executive::execute_block(block)
		}

		fn initialize_block(header: &<Block as BlockT>::Header) {
			Executive::initialize_block(header)
		}
	}

	impl sp_api::Metadata<Block> for Runtime {
		fn metadata() -> OpaqueMetadata {
			OpaqueMetadata::new(Runtime::metadata().into())
		}
	}

	impl sp_block_builder::BlockBuilder<Block> for Runtime {
		fn apply_extrinsic(extrinsic: <Block as BlockT>::Extrinsic) -> ApplyExtrinsicResult {
			Executive::apply_extrinsic(extrinsic)
		}

		fn finalize_block() -> <Block as BlockT>::Header {
			Executive::finalize_block()
		}

		fn inherent_extrinsics(data: sp_inherents::InherentData) -> Vec<<Block as BlockT>::Extrinsic> {
			data.create_extrinsics()
		}

		fn check_inherents(
			block: Block,
			data: sp_inherents::InherentData,
		) -> sp_inherents::CheckInherentsResult {
			data.check_extrinsics(&block)
		}
	}

	impl sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block> for Runtime {
		fn validate_transaction(
			source: TransactionSource,
			tx: <Block as BlockT>::Extrinsic,
			block_hash: <Block as BlockT>::Hash,
		) -> TransactionValidity {
			Executive::validate_transaction(source, tx, block_hash)
		}
	}

	impl sp_offchain::OffchainWorkerApi<Block> for Runtime {
		fn offchain_worker(header: &<Block as BlockT>::Header) {
			Executive::offchain_worker(header)
		}
	}

	impl sp_session::SessionKeys<Block> for Runtime {
		fn generate_session_keys(seed: Option<Vec<u8>>) -> Vec<u8> {
			SessionKeys::generate(seed)
		}

		fn decode_session_keys(
			encoded: Vec<u8>,
		) -> Option<Vec<(Vec<u8>, KeyTypeId)>> {
			SessionKeys::decode_into_raw_public_keys(&encoded)
		}
	}

	impl frame_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Index> for Runtime {
		fn account_nonce(account: AccountId) -> Index {
			System::account_nonce(account)
		}
	}

	impl pallet_transaction_payment_rpc_runtime_api::TransactionPaymentApi<Block, Balance> for Runtime {
		fn query_info(
			uxt: <Block as BlockT>::Extrinsic,
			len: u32,
		) -> pallet_transaction_payment_rpc_runtime_api::RuntimeDispatchInfo<Balance> {
			TransactionPayment::query_info(uxt, len)
		}
		fn query_fee_details(
			uxt: <Block as BlockT>::Extrinsic,
			len: u32,
		) -> pallet_transaction_payment::FeeDetails<Balance> {
			TransactionPayment::query_fee_details(uxt, len)
		}
	}

	impl pallet_mt_rpc_runtime_api::MerkleTreeApi<Block, Element> for Runtime {
		fn get_leaf(tree_id: u32, index: u32) -> Option<Element> {
			let v = MerkleTreeBn254::leaves(tree_id, index);
			if v == Element::default() {
				None
			} else {
				Some(v)
			}
		}
	}

	impl cumulus_primitives_core::CollectCollationInfo<Block> for Runtime {
		fn collect_collation_info(header: &<Block as BlockT>::Header) -> cumulus_primitives_core::CollationInfo {
			ParachainSystem::collect_collation_info(header)
		}
	}

	#[cfg(feature = "runtime-benchmarks")]
	impl frame_benchmarking::Benchmark<Block> for Runtime {
		fn benchmark_metadata(extra: bool) -> (
			Vec<frame_benchmarking::BenchmarkList>,
			Vec<frame_support::traits::StorageInfo>,
		) {
			use frame_benchmarking::{list_benchmark, Benchmarking, BenchmarkList};
			use frame_support::traits::StorageInfoTrait;

			let mut list = Vec::<BenchmarkList>::new();

			list_benchmark!(list, extra, pallet_hasher, HasherBn254);
			list_benchmark!(list, extra, pallet_hasher, HasherBls381);
			list_benchmark!(list, extra, pallet_mt, MerkleTreeBn254);
			list_benchmark!(list, extra, pallet_mt, MerkleTreeBls381);
			list_benchmark!(list, extra, pallet_linkable_tree, LinkableTreeBn254);
			list_benchmark!(list, extra, pallet_linkable_tree, LinkableTreeBls381);
			list_benchmark!(list, extra, pallet_anchor, AnchorBn254);
			list_benchmark!(list, extra, pallet_anchor, AnchorBls381);
			list_benchmark!(list, extra, pallet_mixer, MixerBn254);
			list_benchmark!(list, extra, pallet_mixer, MixerBls381);
			list_benchmark!(list, extra, pallet_verifier, MixerVerifierBn254);
			list_benchmark!(list, extra, pallet_verifier, MixerVerifierBls381);
			list_benchmark!(list, extra, pallet_verifier, AnchorVerifierBn254);
			list_benchmark!(list, extra, pallet_verifier, AnchorVerifierBls381);
			list_benchmark!(list, extra, pallet_token_wrapper, TokenWrapper);

			let storage_info = AllPalletsWithSystem::storage_info();

			return (list, storage_info)
		}

		fn dispatch_benchmark(
			config: frame_benchmarking::BenchmarkConfig
		) -> Result<Vec<frame_benchmarking::BenchmarkBatch>, sp_runtime::RuntimeString> {
			use frame_benchmarking::{Benchmarking, BenchmarkBatch, add_benchmark, TrackedStorageKey};

			impl frame_system_benchmarking::Config for Runtime {}


			let whitelist: Vec<TrackedStorageKey> = vec![
				// Block Number
				hex_literal::hex!("26aa394eea5630e07c48ae0c9558cef702a5c1b19ab7a04f536c519aca4983ac").to_vec().into(),
				// Total Issuance
				hex_literal::hex!("c2261276cc9d1f8598ea4b6a74b15c2f57c875e4cff74148e4628f264b974c80").to_vec().into(),
				// Execution Phase
				hex_literal::hex!("26aa394eea5630e07c48ae0c9558cef7ff553b5a9862a516939d82b3d3d8661a").to_vec().into(),
				// Event Count
				hex_literal::hex!("26aa394eea5630e07c48ae0c9558cef70a98fdbe9ce6c55837576c60c7af3850").to_vec().into(),
				// System Events
				hex_literal::hex!("26aa394eea5630e07c48ae0c9558cef780d41e5e16056765bc8461851072c9d7").to_vec().into(),
			];

			let mut batches = Vec::<BenchmarkBatch>::new();
			let params = (&config, &whitelist);

			add_benchmark!(params, batches, pallet_hasher, HasherBn254);
			add_benchmark!(params, batches, pallet_hasher, HasherBls381);
			add_benchmark!(params, batches, pallet_mt, MerkleTreeBn254);
			add_benchmark!(params, batches, pallet_mt, MerkleTreeBls381);
			add_benchmark!(params, batches, pallet_linkable_tree, LinkableTreeBn254);
			add_benchmark!(params, batches, pallet_linkable_tree, LinkableTreeBls381);
			add_benchmark!(params, batches, pallet_anchor, AnchorBn254);
			add_benchmark!(params, batches, pallet_anchor, AnchorBls381);
			add_benchmark!(params, batches, pallet_mixer, MixerBn254);
			add_benchmark!(params, batches, pallet_mixer, MixerBls254);
			add_benchmark!(params, batches, pallet_verifier, MixerVerifierBn254);
			add_benchmark!(params, batches, pallet_verifier, MixerVerifierBls381);
			add_benchmark!(params, batches, pallet_verifier, AnchorVerifierBn254);
			add_benchmark!(params, batches, pallet_verifier, AnchorVerifierBls381);
			add_benchmark!(params, batches, pallet_token_wrapper, TokenWrapper);

			if batches.is_empty() { return Err("Benchmark not found for this pallet.".into()) }
			Ok(batches)
		}
	}
}

struct CheckInherents;

impl cumulus_pallet_parachain_system::CheckInherents<Block> for CheckInherents {
	fn check_inherents(
		block: &Block,
		relay_state_proof: &cumulus_pallet_parachain_system::RelayChainStateProof,
	) -> sp_inherents::CheckInherentsResult {
		let relay_chain_slot = relay_state_proof
			.read_slot()
			.expect("Could not read the relay chain slot from the proof");

		let inherent_data =
			cumulus_primitives_timestamp::InherentDataProvider::from_relay_chain_slot_and_duration(
				relay_chain_slot,
				sp_std::time::Duration::from_secs(6),
			)
			.create_inherent_data()
			.expect("Could not create the timestamp inherent data");

		inherent_data.check_extrinsics(block)
	}
}

cumulus_pallet_parachain_system::register_validate_block! {
	Runtime = Runtime,
	BlockExecutor = cumulus_pallet_aura_ext::BlockExecutor::<Runtime, Executive>,
	CheckInherents = CheckInherents,
}
