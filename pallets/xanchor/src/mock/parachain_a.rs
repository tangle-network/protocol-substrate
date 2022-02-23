#![allow(clippy::zero_prefixed_literal)]
//! Parachain runtime mock.
use crate as pallet_xanchor;

use codec::{Decode, Encode};
use frame_support::{
	construct_runtime,
	dispatch::DispatchResult,
	parameter_types,
	traits::{Everything, Nothing, SortedMembers},
	weights::{constants::WEIGHT_PER_SECOND, Weight},
	PalletId,
};
use frame_support::traits::{OnInitialize, GenesisBuild};
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
use webb_primitives::{Amount, BlockNumber, ChainId};
use pallet_xcm::XcmPassthrough;
use polkadot_core_primitives::BlockNumber as RelayBlockNumber;
use polkadot_parachain::primitives::{
	DmpMessageHandler, Id as ParaId, Sibling, XcmpMessageFormat, XcmpMessageHandler,
};
pub use webb_primitives::{
	hasher::{HasherModule, InstanceHasher},
	types::ElementTrait,
	AccountId
};
use std::ops::Mul;
use xcm::{latest::prelude::*, VersionedXcm};
use xcm_builder::{
	AccountId32Aliases, AllowUnpaidExecutionFrom, CurrencyAdapter as XcmCurrencyAdapter,
	EnsureXcmOrigin, FixedRateOfFungible, FixedWeightBounds, IsConcrete, LocationInverter,
	NativeAsset, ParentAsSuperuser, ParentIsPreset, RelayChainAsNative, SiblingParachainAsNative,
	SiblingParachainConvertsVia, SignedAccountId32AsNative, SignedToAccountId32,
	SovereignSignedViaLocation,
};
use pallet_democracy::{AccountVote, Conviction, Vote};
use xcm_executor::{Config, XcmExecutor};
use arkworks_utils::utils::common::{setup_params_x5_3, Curve};
use ark_bn254::Fr as Bn254Fr;
use frame_support::assert_ok;
use frame_benchmarking::account;
use super::{AccountOne, AccountTwo, AccountThree, AccountFour, AccountFive, AccountSix, para_account_id, PARAID_A, INITIAL_BALANCE, Element};

pub type Balance = u128;
/// Type for storing the id of an asset.
pub type OrmlAssetId = u32;

pub type ParachainPalletXcm = pallet_xcm::Pallet<Runtime>;

pub fn para_ext(para_id: u32) -> sp_io::TestExternalities {
	let mut t = frame_system::GenesisConfig::default().build_storage::<Runtime>().unwrap();

	pallet_balances::GenesisConfig::<Runtime> {
		balances: vec![
			(AccountOne::get(), INITIAL_BALANCE.mul(1u128)),
			(AccountTwo::get(), INITIAL_BALANCE.mul(2u128)),
			(AccountThree::get(), INITIAL_BALANCE.mul(3u128)),
			(AccountFour::get(), INITIAL_BALANCE.mul(4u128)),
			(AccountFive::get(), INITIAL_BALANCE.mul(5u128)),
			(AccountSix::get(), INITIAL_BALANCE.mul(6u128)),
			(para_account_id(para_id), INITIAL_BALANCE),
		],
	}
	.assimilate_storage(&mut t)
	.unwrap();
	pallet_democracy::GenesisConfig::<Runtime>::default()
		.assimilate_storage(&mut t)
		.unwrap();
	let mut ext = sp_io::TestExternalities::new(t);
	ext.execute_with(|| {
		next_block();
		MsgQueue::set_para_id(para_id.into());
	});
	ext
}

pub fn next_block() {
	System::set_block_number(System::block_number() + 1);
	Scheduler::on_initialize(System::block_number());
	Democracy::on_initialize(System::block_number());
}

pub fn fast_forward_to(n: u64) {
	while System::block_number() < n {
		next_block();
	}
}

const SEED: u32 = 0;

pub fn setup_environment(curve: Curve) -> Vec<u8> {
	match curve {
		Curve::Bn254 => {
			let params3 = setup_params_x5_3::<Bn254Fr>(curve);

			// 1. Setup The Hasher Pallet.
			assert_ok!(HasherPallet::force_set_parameters(Origin::root(), params3.to_bytes()));
			// 2. Initialize MerkleTree pallet.
			<MerkleTree as OnInitialize<u64>>::on_initialize(1);
			// 3. Setup the VerifierPallet
			//    but to do so, we need to have a VerifyingKey
			let pk_bytes = include_bytes!(
				"../../../../protocol-substrate-fixtures/fixed-anchor/bn254/x5/proving_key_uncompressed.bin"
			);
			let vk_bytes = include_bytes!(
				"../../../../protocol-substrate-fixtures/fixed-anchor/bn254/x5/verifying_key.bin"
			);

			assert_ok!(VerifierPallet::force_set_parameters(Origin::root(), vk_bytes.to_vec()));

			for account_id in [
				account::<AccountId>("", 1, SEED),
				account::<AccountId>("", 2, SEED),
				account::<AccountId>("", 3, SEED),
				account::<AccountId>("", 4, SEED),
				account::<AccountId>("", 5, SEED),
				account::<AccountId>("", 6, SEED),
			] {
				assert_ok!(Balances::set_balance(Origin::root(), account_id, 100_000_000, 0));
			}

			// finally return the provingkey bytes
			pk_bytes.to_vec()
		},
		Curve::Bls381 => {
			unimplemented!()
		},
	}
}

pub fn setup_environment_withdraw(curve: Curve) -> Vec<u8> {
	for account_id in [
		account::<AccountId>("", 1, SEED),
		account::<AccountId>("", 2, SEED),
		account::<AccountId>("", 3, SEED),
		account::<AccountId>("", 4, SEED),
		account::<AccountId>("", 5, SEED),
		account::<AccountId>("", 6, SEED),
	] {
		assert_ok!(Balances::set_balance(Origin::root(), account_id, 100_000_000, 0));
	}

	match curve {
		Curve::Bn254 => {
			let params3 = setup_params_x5_3::<Bn254Fr>(curve);

			// 1. Setup The Hasher Pallet.
			assert_ok!(HasherPallet::force_set_parameters(Origin::root(), params3.to_bytes()));
			// 2. Initialize MerkleTree pallet.
			<MerkleTree as OnInitialize<u64>>::on_initialize(1);
			// 3. Setup the VerifierPallet
			//    but to do so, we need to have a VerifyingKey
			let pk_bytes = include_bytes!(
				"../../../../protocol-substrate-fixtures/fixed-anchor/bn254/x5/proving_key_uncompressed.bin"
			);
			let vk_bytes = include_bytes!(
				"../../../../protocol-substrate-fixtures/fixed-anchor/bn254/x5/verifying_key.bin"
			);

			assert_ok!(VerifierPallet::force_set_parameters(Origin::root(), vk_bytes.to_vec()));

			// finally return the provingkey bytes
			pk_bytes.to_vec()
		},
		Curve::Bls381 => {
			unimplemented!()
		},
	}
}

// Governance System Tests
pub fn aye(who: AccountId) -> AccountVote<BalanceOf<Runtime, ()>> {
	AccountVote::Standard {
		vote: Vote { aye: true, conviction: Conviction::None },
		balance: Balances::free_balance(&who),
	}
}

pub fn nay(who: AccountId) -> AccountVote<BalanceOf<Runtime, ()>> {
	AccountVote::Standard {
		vote: Vote { aye: false, conviction: Conviction::None },
		balance: Balances::free_balance(&who),
	}
}

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
	type MaxConsumers = frame_support::traits::ConstU32<16>;
	type OnKilledAccount = ();
	type OnNewAccount = ();
	type OnSetCode = ();
	type Origin = Origin;
	type PalletInfo = PalletInfo;
	type SS58Prefix = ();
	type SystemWeightInfo = ();
	type Version = ();
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

parameter_types! {
	pub const KsmLocation: MultiLocation = MultiLocation::parent();
	pub const RelayNetwork: NetworkId = NetworkId::Kusama;
	pub RelayChainOrigin: Origin = cumulus_pallet_xcm::Origin::Relay.into();
	pub Ancestry: MultiLocation = Parachain(MsgQueue::parachain_id().into()).into();
}

pub type LocationToAccountId = (
	ParentIsPreset<AccountId>,
	SiblingParachainConvertsVia<ParaId, AccountId>,
	SiblingParachainConvertsVia<Sibling, AccountId>,
	AccountId32Aliases<RelayNetwork, AccountId>,
);

pub type XcmOriginToCallOrigin = (
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
	pub const MaxInstructions: u32 = 100;
}

pub type LocalAssetTransactor =
	XcmCurrencyAdapter<Balances, IsConcrete<KsmLocation>, LocationToAccountId, AccountId, ()>;

pub type XcmRouter = super::ParachainXcmRouter<MsgQueue>;
pub type Barrier = AllowUnpaidExecutionFrom<Everything>;

pub struct XcmConfig;
impl Config for XcmConfig {
	type Call = Call;
	type XcmSender = XcmRouter;
	type AssetTransactor = LocalAssetTransactor;
	type OriginConverter = XcmOriginToCallOrigin;
	type IsReserve = NativeAsset;
	type IsTeleporter = ();
	type LocationInverter = LocationInverter<Ancestry>;
	type Barrier = Barrier;
	type Weigher = FixedWeightBounds<UnitWeightCost, Call, MaxInstructions>;
	type Trader = FixedRateOfFungible<KsmPerSecond, ()>;
	type ResponseHandler = ();
	type AssetTrap = ();
	type AssetClaims = ();
	type SubscriptionService = ();
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
	#[pallet::without_storage_info]
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
						Outcome::Error(e) => (Err(e.clone()), Event::Fail(Some(hash), e)),
						Outcome::Complete(w) => (Ok(w), Event::Success(Some(hash))),
						// As far as the caller is concerned, this was dispatched without error, so
						// we just report the weight used.
						Outcome::Incomplete(w, e) => (Ok(w), Event::Fail(Some(hash), e)),
					}
				},
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
				let _ = XcmpMessageFormat::decode(&mut data_ref)
					.expect("Simulator encodes with versioned xcm format; qed");

				let mut remaining_fragments = &data_ref[..];
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
		fn handle_dmp_messages(
			iter: impl Iterator<Item = (RelayBlockNumber, Vec<u8>)>,
			limit: Weight,
		) -> Weight {
			for (_i, (_sent_at, data)) in iter.enumerate() {
				let id = sp_io::hashing::blake2_256(&data[..]);
				let maybe_msg =
					VersionedXcm::<T::Call>::decode(&mut &data[..]).map(Xcm::<T::Call>::try_from);
				match maybe_msg {
					Err(_) => {
						Self::deposit_event(Event::InvalidFormat(id));
					},
					Ok(Err(())) => {
						Self::deposit_event(Event::UnsupportedVersion(id));
					},
					Ok(Ok(x)) => {
						let outcome = T::XcmExecutor::execute_xcm(Parent, x.clone(), limit);
						<ReceivedDmp<T>>::append(x);
						Self::deposit_event(Event::ExecutedDownward(id, outcome));
					},
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
	type Event = Event;
	type SendXcmOrigin = EnsureXcmOrigin<Origin, LocalOriginToLocation>;
	type XcmRouter = XcmRouter;
	type ExecuteXcmOrigin = EnsureXcmOrigin<Origin, LocalOriginToLocation>;
	type XcmExecuteFilter = Everything;
	type XcmExecutor = XcmExecutor<XcmConfig>;
	type XcmTeleportFilter = Nothing;
	type XcmReserveTransferFilter = Everything;
	type Weigher = FixedWeightBounds<UnitWeightCost, Call, MaxInstructions>;
	type LocationInverter = LocationInverter<Ancestry>;
	type Origin = Origin;
	type Call = Call;
	const VERSION_DISCOVERY_QUEUE_SIZE: u32 = 100;
	type AdvertisedXcmVersion = pallet_xcm::CurrentXcmVersion;
}

impl cumulus_pallet_xcm::Config for Runtime {
	type Event = Event;
	type XcmExecutor = XcmExecutor<XcmConfig>;
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
	// Substrate standalone chain ID type
	pub const ChainType: [u8; 2] = [2, 0];
	// This identifier should equal the para ID.
	// Note: this can cause issues if they do not match in production.
	pub const ChainIdentifier: ChainId = PARAID_A as u64;
}

impl pallet_linkable_tree::Config for Runtime {
	type ChainId = ChainId;
	type ChainType = ChainType;
	type ChainIdentifier = ChainIdentifier;
	type Event = Event;
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
	type BaseDeposit = ();
	type ByteDeposit = ();
	type Currency = ();
	type Event = Event;
	type ManagerOrigin = frame_system::EnsureRoot<AccountId>;
	type MaxSize = frame_support::traits::ConstU32<1024>;
	type WeightInfo = ();
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
	type NoPreimagePostponement = NoPreimagePostponement;
	type Origin = Origin;
	type OriginPrivilegeCmp = frame_support::traits::EqualPrivilegeOnly;
	type PalletsOrigin = OriginCaller;
	type PreimageProvider = Preimage;
	type ScheduleOrigin = EnsureRoot<AccountId>;
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

pub struct OneToFive;
impl SortedMembers<AccountId> for OneToFive {
	fn sorted_members() -> Vec<AccountId> {
		(1..=5).into_iter().map(|x| sp_runtime::AccountId32::new([x; 32])).collect()
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

impl crate::types::DemocracyGovernanceDelegate<Runtime, Call, BalanceOf<Runtime, ()>>
	for Democracy
{
	fn propose(
		origin: OriginFor<Runtime>,
		proposal: Call,
		value: BalanceOf<Runtime, ()>,
	) -> DispatchResult {
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
