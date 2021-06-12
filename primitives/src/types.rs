use frame_support::pallet_prelude::*;
use codec::{Encode, Decode};

#[derive(Clone, Default, Encode, Decode, Eq, PartialEq, RuntimeDebug)]
pub struct DepositDetails<AccountId, Balance> {
	pub depositor: AccountId,
	pub deposit: Balance,
}