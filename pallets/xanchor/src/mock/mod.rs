pub mod parachain_a;
pub mod parachain_b;
pub mod relay_chain;
pub(crate) mod test_utils;

use codec::{Decode, Encode};
pub use webb_primitives::{AccountId, types::ElementTrait};
use frame_support::ord_parameter_types;
use polkadot_parachain::primitives::Id as ParaId;
use sp_runtime::traits::AccountIdConversion;
use xcm_simulator::{decl_test_network, decl_test_parachain, decl_test_relay_chain};

use frame_support::{ Deserialize, Serialize };

ord_parameter_types! {
	pub const AccountOne: AccountId = sp_runtime::AccountId32::new([1u8; 32]);
	pub const AccountTwo: AccountId = sp_runtime::AccountId32::new([2u8; 32]);
	pub const AccountThree: AccountId = sp_runtime::AccountId32::new([3u8; 32]);
	pub const AccountFour: AccountId = sp_runtime::AccountId32::new([4u8; 32]);
	pub const AccountFive: AccountId = sp_runtime::AccountId32::new([5u8; 32]);
	pub const AccountSix: AccountId = sp_runtime::AccountId32::new([6u8; 32]);
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

pub const INITIAL_BALANCE: u128 = 1_000_000_000;
pub const PARAID_A: u32 = 2000;
pub const PARAID_B: u32 = 3000;

decl_test_parachain! {
	pub struct ParaA {
		Runtime = parachain1::Runtime,
		XcmpMessageHandler = parachain_a::MsgQueue,
		DmpMessageHandler = parachain_a::MsgQueue,
		new_ext = parachain_a::para_ext(PARAID_A),
	}
}

decl_test_parachain! {
	pub struct ParaB {
		Runtime = parachain2::Runtime,
		XcmpMessageHandler = parachain_b::MsgQueue,
		DmpMessageHandler = parachain_b::MsgQueue,
		new_ext = parachain_b::para_ext(PARAID_B),
	}
}

decl_test_relay_chain! {
	pub struct Relay {
		Runtime = relay_chain::Runtime,
		XcmConfig = relay_chain::XcmConfig,
		new_ext = relay_ext(),
	}
}

decl_test_network! {
	pub struct MockNet {
		relay_chain = Relay,
		parachains = vec![
			(PARAID_A, ParaA),
			(PARAID_B, ParaB),
		],
	}
}

pub fn para_account_id(id: u32) -> relay_chain::AccountId {
	ParaId::from(id).into_account()
}

pub fn relay_ext() -> sp_io::TestExternalities {
	use relay_chain::{Runtime, System};

	let mut t = frame_system::GenesisConfig::default().build_storage::<Runtime>().unwrap();

	pallet_balances::GenesisConfig::<Runtime> {
		balances: vec![
			(AccountOne::get(), INITIAL_BALANCE),
			(AccountTwo::get(), INITIAL_BALANCE),
			(AccountThree::get(), INITIAL_BALANCE),
			(AccountFour::get(), INITIAL_BALANCE),
			(AccountFive::get(), INITIAL_BALANCE),
			(AccountSix::get(), INITIAL_BALANCE),
			(para_account_id(PARAID_A), INITIAL_BALANCE),
			(para_account_id(PARAID_B), INITIAL_BALANCE),
		],
	}
	.assimilate_storage(&mut t)
	.unwrap();

	let mut ext = sp_io::TestExternalities::new(t);
	ext.execute_with(|| System::set_block_number(1));
	ext
}

pub type RelayChainPalletXcm = pallet_xcm::Pallet<relay_chain::Runtime>;
