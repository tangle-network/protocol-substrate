#![allow(clippy::zero_prefixed_literal)]
//! Parachain runtime mock.
use crate as pallet_xanchor;

use codec::{Decode, Encode};
use webb_primitives::{Amount, BlockNumber, ChainId};
use frame_support::{
	construct_runtime,
	dispatch::DispatchResult,
	ord_parameter_types, parameter_types,
	traits::{Everything, Nothing, SortedMembers},
	weights::{constants::WEIGHT_PER_SECOND, Weight},
	Deserialize, PalletId, Serialize,
};
use frame_system::{pallet_prelude::OriginFor, EnsureRoot, EnsureSignedBy};
use orml_currencies::BasicCurrencyAdapter;
use pallet_anchor::BalanceOf;
use sp_core::H256;
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, Hash, IdentityLookup},
	Perbill,
};
use sp_std::{convert::TryFrom, prelude::*};

pub use webb_primitives::{
	hasher::{HasherModule, InstanceHasher},
	types::ElementTrait,
	AccountId,
};
use pallet_xcm::XcmPassthrough;
use polkadot_core_primitives::BlockNumber as RelayBlockNumber;
use polkadot_parachain::primitives::{DmpMessageHandler, Id as ParaId, Sibling, XcmpMessageFormat, XcmpMessageHandler};
use xcm::{latest::prelude::*, VersionedXcm};
use xcm_builder::{
	AccountId32Aliases, AllowUnpaidExecutionFrom, CurrencyAdapter as XcmCurrencyAdapter, EnsureXcmOrigin,
	FixedRateOfFungible, FixedWeightBounds, IsConcrete, LocationInverter, NativeAsset, ParentAsSuperuser,
	ParentIsDefault, RelayChainAsNative, SiblingParachainAsNative, SiblingParachainConvertsVia,
	SignedAccountId32AsNative, SignedToAccountId32, SovereignSignedViaLocation,
};
use xcm_executor::{Config, XcmExecutor};

pub type Balance = u128;

/// Type for storing the id of an asset.
pub type OrmlAssetId = u32;

parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub BlockWeights: frame_system::limits::BlockWeights =
		frame_system::limits::BlockWeights::simple_max(1_000_000);
}

impl frame_system::Config for Runtime {
	type AccountData = pallet_balances::AccountData<Balance>;
	type AccountId = AccountId;
	type BaseCallFilter = Everything;
	type BlockHashCount = BlockHashCount;
	type BlockLength = ();
	type BlockNumber = u64;
	type BlockWeights = ();
	type Call = Call;
	type DbWeight = ();
	type Event = Event;
	type Hash = H256;
	type Hashing = ::sp_runtime::traits::BlakeTwo256;
	type Header = Header;
	type Index = u64;
	type Lookup = IdentityLookup<Self::AccountId>;
	type OnKilledAccount = ();
	type OnNewAccount = ();
	type OnSetCode = ();
	type Origin = Origin;
	type PalletInfo = PalletInfo;
	type SS58Prefix = ();
	type SystemWeightInfo = ();
	type Version = ();
	type MaxConsumers = frame_support::traits::ConstU32<16>;
}

parameter_types! {
	pub ExistentialDeposit: Balance = 1;
	pub const MaxLocks: u32 = 50;
	pub const MaxReserves: u32 = 50;
}

impl pallet_balances::Config for Runtime {
	type AccountStore = System;
	type Balance = Balance;
	type DustRemoval = ();
	type Event = Event;
	type ExistentialDeposit = ExistentialDeposit;
	type MaxLocks = MaxLocks;
	type MaxReserves = MaxReserves;
	type ReserveIdentifier = [u8; 8];
	type WeightInfo = ();
}

parameter_types! {
	pub const ReservedXcmpWeight: Weight = WEIGHT_PER_SECOND / 4;
	pub const ReservedDmpWeight: Weight = WEIGHT_PER_SECOND / 4;
}

impl cumulus_pallet_xcm::Config for Runtime {
	type Event = Event;
	type XcmExecutor = XcmExecutor<XcmConfig>;
}

parameter_types! {
	pub const KsmLocation: MultiLocation = MultiLocation::parent();
	pub const RelayNetwork: NetworkId = NetworkId::Kusama;
	pub RelayChainOrigin: Origin = cumulus_pallet_xcm::Origin::Relay.into();
	pub Ancestry: MultiLocation = Parachain(MsgQueue::parachain_id().into()).into();
}

pub type LocationToAccountId = (
	ParentIsDefault<AccountId>,
	SiblingParachainConvertsVia<ParaId, AccountId>,
	SiblingParachainConvertsVia<Sibling, AccountId>,
	AccountId32Aliases<RelayNetwork, AccountId>,
);

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
	pub const UnitWeightCost: Weight = 1;
	pub KsmPerSecond: (AssetId, u128) = (Concrete(Parent.into()), 1);
	pub const MaxInstructions: u32 = 10_00;
}

pub type LocalAssetTransactor =
	XcmCurrencyAdapter<Balances, IsConcrete<KsmLocation>, LocationToAccountId, AccountId, ()>;

pub type XcmRouter = super::ParachainXcmRouter<MsgQueue>;
pub type Barrier = AllowUnpaidExecutionFrom<Everything>;

pub struct XcmConfig;
impl Config for XcmConfig {
	type AssetClaims = ();
	type AssetTransactor = LocalAssetTransactor;
	type AssetTrap = ();
	type Barrier = Barrier;
	type Call = Call;
	type IsReserve = NativeAsset;
	type IsTeleporter = ();
	type LocationInverter = LocationInverter<Ancestry>;
	type OriginConverter = XcmOriginToTransactDispatchOrigin;
	type ResponseHandler = ();
	type SubscriptionService = ();
	type Trader = FixedRateOfFungible<KsmPerSecond, ()>;
	type Weigher = FixedWeightBounds<UnitWeightCost, Call, MaxInstructions>;
	type XcmSender = XcmRouter;
}

#[frame_support::pallet]
pub mod mock_msg_queue {
	use super::*;
	use frame_support::pallet_prelude::*;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		type XcmExecutor: ExecuteXcm<Self::Call>;
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::storage]
	#[pallet::getter(fn parachain_id)]
	pub(super) type ParachainId<T: Config> = StorageValue<_, ParaId, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn received_dmp)]
	/// A queue of received DMP messages
	pub(super) type ReceivedDmp<T: Config> = StorageValue<_, Vec<Xcm<T::Call>>, ValueQuery>;

	impl<T: Config> Get<ParaId> for Pallet<T> {
		fn get() -> ParaId {
			Self::parachain_id()
		}
	}

	pub type MessageId = [u8; 32];

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		// XCMP
		/// Some XCM was executed OK.
		Success(Option<T::Hash>),
		/// Some XCM failed.
		Fail(Option<T::Hash>, XcmError),
		/// Bad XCM version used.
		BadVersion(Option<T::Hash>),
		/// Bad XCM format used.
		BadFormat(Option<T::Hash>),

		// DMP
		/// Downward message is invalid XCM.
		InvalidFormat(MessageId),
		/// Downward message is unsupported version of XCM.
		UnsupportedVersion(MessageId),
		/// Downward message executed with the given outcome.
		ExecutedDownward(MessageId, Outcome),
	}

	impl<T: Config> Pallet<T> {
		pub fn set_para_id(para_id: ParaId) {
			ParachainId::<T>::put(para_id);
		}

		fn handle_xcmp_message(
			sender: ParaId,
			_sent_at: RelayBlockNumber,
			xcm: VersionedXcm<T::Call>,
			max_weight: Weight,
		) -> Result<Weight, XcmError> {
			let hash = Encode::using_encoded(&xcm, T::Hashing::hash);
			let (result, event) = match Xcm::<T::Call>::try_from(xcm) {
				Ok(xcm) => {
					let location = (1, Parachain(sender.into()));
					match T::XcmExecutor::execute_xcm(location, xcm, max_weight) {
						Outcome::Error(e) => (Err(e), Event::Fail(Some(hash), e)),
						Outcome::Complete(w) => (Ok(w), Event::Success(Some(hash))),
						// As far as the caller is concerned, this was dispatched without error, so
						// we just report the weight used.
						Outcome::Incomplete(w, e) => (Ok(w), Event::Fail(Some(hash), e)),
					}
				}
				Err(()) => (Err(XcmError::UnhandledXcmVersion), Event::BadVersion(Some(hash))),
			};
			Self::deposit_event(event);
			result
		}
	}

	impl<T: Config> XcmpMessageHandler for Pallet<T> {
		fn handle_xcmp_messages<'a, I: Iterator<Item = (ParaId, RelayBlockNumber, &'a [u8])>>(
			iter: I,
			max_weight: Weight,
		) -> Weight {
			for (sender, sent_at, data) in iter {
				let mut data_ref = data;
				let _ =
					XcmpMessageFormat::decode(&mut data_ref).expect("Simulator encodes with versioned xcm format; qed");

				let mut remaining_fragments = data_ref;
				while !remaining_fragments.is_empty() {
					if let Ok(xcm) = VersionedXcm::<T::Call>::decode(&mut remaining_fragments) {
						let _ = Self::handle_xcmp_message(sender, sent_at, xcm, max_weight);
					} else {
						debug_assert!(false, "Invalid incoming XCMP message data");
					}
				}
			}
			max_weight
		}
	}

	impl<T: Config> DmpMessageHandler for Pallet<T> {
		fn handle_dmp_messages(iter: impl Iterator<Item = (RelayBlockNumber, Vec<u8>)>, limit: Weight) -> Weight {
			for (_i, (_sent_at, data)) in iter.enumerate() {
				let id = sp_io::hashing::blake2_256(&data[..]);
				let maybe_msg = VersionedXcm::<T::Call>::decode(&mut &data[..]).map(Xcm::<T::Call>::try_from);
				match maybe_msg {
					Err(_) => {
						Self::deposit_event(Event::InvalidFormat(id));
					}
					Ok(Err(())) => {
						Self::deposit_event(Event::UnsupportedVersion(id));
					}
					Ok(Ok(x)) => {
						let outcome = T::XcmExecutor::execute_xcm(Parent, x.clone(), limit);
						<ReceivedDmp<T>>::append(x);
						Self::deposit_event(Event::ExecutedDownward(id, outcome));
					}
				}
			}
			limit
		}
	}
}

impl mock_msg_queue::Config for Runtime {
	type Event = Event;
	type XcmExecutor = XcmExecutor<XcmConfig>;
}

pub type LocalOriginToLocation = SignedToAccountId32<Origin, AccountId, RelayNetwork>;

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
	type XcmTeleportFilter = Nothing;

	const VERSION_DISCOVERY_QUEUE_SIZE: u32 = 100;
}

parameter_types! {
	pub const ParameterDeposit: u64 = 1;
	pub const StringLimit: u32 = 50;
	pub const MetadataDepositBase: u64 = 1;
	pub const MetadataDepositPerByte: u64 = 1;
}

impl pallet_verifier::Config for Runtime {
	type Event = Event;
	type ForceOrigin = frame_system::EnsureRoot<AccountId>;
	type Verifier = webb_primitives::verifying::ArkworksVerifierBn254;
	type WeightInfo = ();
}

impl pallet_hasher::Config for Runtime {
	type Event = Event;
	type ForceOrigin = frame_system::EnsureRoot<AccountId>;
	type Hasher = webb_primitives::hashing::ArkworksPoseidonHasherBn254;
	type WeightInfo = ();
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
	pub const MockZeroElement: Element = Element([0; 32]);
}

#[derive(Debug, Encode, Decode, Default, Copy, Clone, PartialEq, Eq, scale_info::TypeInfo, Serialize, Deserialize)]
pub struct Element([u8; 32]);

impl Element {
	pub const fn zero() -> Self {
		Element([0; 32])
	}
}

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

impl pallet_mt::Config for Runtime {
	type Currency = Balances;
	type DataDepositBase = LeafDepositBase;
	type DataDepositPerByte = LeafDepositPerByte;
	type DefaultZeroElement = MockZeroElement;
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
	pub const NativeCurrencyId: OrmlAssetId = 0;
	pub const RegistryStringLimit: u32 = 10;
}

/// Tokens Configurations
impl orml_tokens::Config for Runtime {
	type Amount = Amount;
	type Balance = u128;
	type CurrencyId = OrmlAssetId;
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

impl pallet_asset_registry::Config for Runtime {
	type AssetId = webb_primitives::AssetId;
	type AssetNativeLocation = ();
	type Balance = u128;
	type Event = Event;
	type NativeAssetId = NativeCurrencyId;
	type RegistryOrigin = frame_system::EnsureRoot<AccountId>;
	type StringLimit = RegistryStringLimit;
	type WeightInfo = ();
}

parameter_types! {
	pub const HistoryLength: u32 = 30;
	pub const AnchorPalletId: PalletId = PalletId(*b"py/anchr");
	pub const ChainIdentifier: ChainId = 0;
}

impl pallet_linkable_tree::Config for Runtime {
	type ChainId = ChainId;
	type Event = Event;
	type ChainIdentifier = ChainIdentifier;
	type HistoryLength = HistoryLength;
	type Tree = MerkleTree;
	type WeightInfo = ();
}

impl pallet_anchor::Config for Runtime {
	type Currency = Currencies;
	type Event = Event;
	type LinkableTree = LinkableTree;
	type NativeCurrencyId = NativeCurrencyId;
	type PalletId = AnchorPalletId;
	type PostDepositHook = XAnchor;
	type Verifier = VerifierPallet;
	type WeightInfo = ();
}

impl pallet_xanchor::Config for Runtime {
	type Anchor = Anchor;
	type Call = Call;
	type DemocracyGovernanceDelegate = Democracy;
	type DemocracyOrigin = EnsureRoot<AccountId>;
	type Event = Event;
	type Origin = Origin;
	type ParaId = MsgQueue;
	type XcmSender = XcmRouter;
}

impl pallet_preimage::Config for Runtime {
	type Event = Event;
	type WeightInfo = ();
	type Currency = ();
	type ManagerOrigin = frame_system::EnsureRoot<AccountId>;
	type MaxSize = frame_support::traits::ConstU32<1024>;
	type BaseDeposit = ();
	type ByteDeposit = ();
}

parameter_types! {
	pub MaximumSchedulerWeight: Weight = Perbill::from_percent(80) * BlockWeights::get().max_block;
	pub const NoPreimagePostponement: Option<u64> = Some(2);
}

impl pallet_scheduler::Config for Runtime {
	type Call = Call;
	type Event = Event;
	type MaxScheduledPerBlock = ();
	type MaximumWeight = MaximumSchedulerWeight;
	type Origin = Origin;
	type OriginPrivilegeCmp = frame_support::traits::EqualPrivilegeOnly;
	type PalletsOrigin = OriginCaller;
	type ScheduleOrigin = EnsureRoot<AccountId>;
	type PreimageProvider = Preimage;
	type NoPreimagePostponement = NoPreimagePostponement;
	type WeightInfo = ();
}

parameter_types! {
	pub const LaunchPeriod: u64 = 2;
	pub const VotingPeriod: u64 = 2;
	pub const FastTrackVotingPeriod: u64 = 2;
	pub const MinimumDeposit: u64 = 1;
	pub const EnactmentPeriod: u64 = 2;
	pub const VoteLockingPeriod: u64 = 3;
	pub const CooloffPeriod: u64 = 2;
	pub const MaxVotes: u32 = 100;
	pub const MaxProposals: u32 = 100;
	pub static PreimageByteDeposit: u64 = 0;
	pub static InstantAllowed: bool = false;
}

ord_parameter_types! {
	pub const AccountOne: AccountId = sp_runtime::AccountId32::new([1u8; 32]);
	pub const AccountTwo: AccountId = sp_runtime::AccountId32::new([2u8; 32]);
	pub const AccountThree: AccountId = sp_runtime::AccountId32::new([3u8; 32]);
	pub const AccountFour: AccountId = sp_runtime::AccountId32::new([4u8; 32]);
	pub const AccountFive: AccountId = sp_runtime::AccountId32::new([5u8; 32]);
	pub const AccountSix: AccountId = sp_runtime::AccountId32::new([6u8; 32]);
}

pub struct OneToFive;
impl SortedMembers<AccountId> for OneToFive {
	fn sorted_members() -> Vec<AccountId> {
		(1..=5)
			.into_iter()
			.map(|x| sp_runtime::AccountId32::new([x; 32]))
			.collect()
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn add(_m: &AccountId) {}
}

impl pallet_democracy::Config for Runtime {
	type BlacklistOrigin = EnsureRoot<AccountId>;
	type CancelProposalOrigin = EnsureRoot<AccountId>;
	type CancellationOrigin = EnsureSignedBy<AccountFour, AccountId>;
	type CooloffPeriod = CooloffPeriod;
	type Currency = pallet_balances::Pallet<Self>;
	type EnactmentPeriod = EnactmentPeriod;
	type Event = Event;
	type ExternalDefaultOrigin = EnsureSignedBy<AccountOne, AccountId>;
	type ExternalMajorityOrigin = EnsureSignedBy<AccountThree, AccountId>;
	type ExternalOrigin = EnsureSignedBy<AccountTwo, AccountId>;
	type FastTrackOrigin = EnsureSignedBy<AccountFive, AccountId>;
	type FastTrackVotingPeriod = FastTrackVotingPeriod;
	type InstantAllowed = InstantAllowed;
	type InstantOrigin = EnsureSignedBy<AccountSix, AccountId>;
	type LaunchPeriod = LaunchPeriod;
	type MaxProposals = MaxProposals;
	type MaxVotes = MaxVotes;
	type MinimumDeposit = MinimumDeposit;
	type OperationalPreimageOrigin = EnsureSignedBy<AccountSix, AccountId>;
	type PalletsOrigin = OriginCaller;
	type PreimageByteDeposit = PreimageByteDeposit;
	type Proposal = Call;
	type Scheduler = Scheduler;
	type Slash = ();
	type VetoOrigin = EnsureSignedBy<OneToFive, AccountId>;
	type VoteLockingPeriod = VoteLockingPeriod;
	type VotingPeriod = VotingPeriod;
	type WeightInfo = ();
}

impl crate::types::DemocracyGovernanceDelegate<Runtime, Call, BalanceOf<Runtime, ()>> for Democracy {
	fn propose(origin: OriginFor<Runtime>, proposal: Call, value: BalanceOf<Runtime, ()>) -> DispatchResult {
		let encoded_proposal = proposal.encode();
		let proposal_hash = BlakeTwo256::hash(&encoded_proposal[..]);
		Democracy::note_preimage(origin.clone(), encoded_proposal)?;
		Democracy::propose(origin, proposal_hash, value)?;
		Ok(())
	}
}

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Runtime>;
type Block = frame_system::mocking::MockBlock<Runtime>;

construct_runtime!(
	pub enum Runtime where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system::{Pallet, Call, Storage, Config, Event<T>},
		Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
		Democracy: pallet_democracy::{Pallet, Call, Storage, Config<T>, Event<T>},
		Preimage: pallet_preimage::{Pallet, Call, Storage, Event<T>},
		Scheduler: pallet_scheduler::{Pallet, Call, Storage, Event<T>},
		MsgQueue: mock_msg_queue::{Pallet, Storage, Event<T>},
		PolkadotXcm: pallet_xcm::{Pallet, Call, Event<T>, Origin},
		CumulusXcm: cumulus_pallet_xcm::{Pallet, Event<T>, Origin},
		HasherPallet: pallet_hasher::{Pallet, Call, Storage, Event<T>},
		VerifierPallet: pallet_verifier::{Pallet, Call, Storage, Event<T>},
		MerkleTree: pallet_mt::{Pallet, Call, Storage, Event<T>},
		Currencies: orml_currencies::{Pallet, Call, Event<T>},
		Tokens: orml_tokens::{Pallet, Storage, Call, Event<T>},
		AssetRegistry: pallet_asset_registry::{Pallet, Call, Storage, Event<T>},
		Anchor: pallet_anchor::{Pallet, Call, Storage, Event<T>},
		LinkableTree: pallet_linkable_tree::{Pallet, Call, Storage, Event<T>},
		XAnchor: pallet_xanchor::{Pallet, Call, Storage, Event<T>},
	}
);
