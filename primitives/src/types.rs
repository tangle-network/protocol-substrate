use frame_support::pallet_prelude::*;
use codec::{Encode, Decode};

// Deposit details used in hasher / verifier pallets for
// tracking the reserved deposits of maintainers of various
// parameters
#[derive(Clone, Default, Encode, Decode, Eq, PartialEq, RuntimeDebug)]
pub struct DepositDetails<AccountId, Balance> {
	pub depositor: AccountId,
	pub deposit: Balance,
}