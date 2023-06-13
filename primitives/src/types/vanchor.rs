use super::{ElementTrait, IntoAbiToken, Token};
use codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use sp_std::{vec, vec::Vec};

#[derive(Clone, Encode, Decode, TypeInfo, MaxEncodedLen)]
pub struct VAnchorMetadata<AccountId, CurrencyId> {
	/// Creator account
	pub creator: Option<AccountId>,
	/// Option of specifying a fungible asset. When None, the asset is the
	/// native currency.
	pub asset: CurrencyId,
}

#[derive(Clone, Encode, Decode, Debug, Eq, PartialEq, TypeInfo)]
pub struct ProofData<E> {
	pub proof: Vec<u8>,
	pub public_amount: E,
	pub roots: Vec<E>,
	pub input_nullifiers: Vec<E>,
	pub output_commitments: Vec<E>,
	pub ext_data_hash: E,
}

impl<E: ElementTrait> ProofData<E> {
	pub fn new(
		proof: Vec<u8>,
		public_amount: E,
		roots: Vec<E>,
		input_nullifiers: Vec<E>,
		output_commitments: Vec<E>,
		ext_data_hash: E,
	) -> Self {
		Self { proof, public_amount, roots, input_nullifiers, output_commitments, ext_data_hash }
	}
}

#[derive(Encode, Decode, Default, Debug, Clone, Eq, PartialEq, TypeInfo)]
pub struct ExtData<AccountId: Encode, Amount: Encode, Balance: Encode, CurrencyId: Encode> {
	pub recipient: AccountId,
	pub relayer: AccountId,
	pub ext_amount: Amount,
	pub fee: Balance,
	pub refund: Balance,
	pub token: CurrencyId,
	pub encrypted_output1: Vec<u8>,
	pub encrypted_output2: Vec<u8>,
}

impl<I: Encode, A: Encode, B: Encode, C: Encode> ExtData<I, A, B, C> {
	#[allow(clippy::too_many_arguments)]
	pub fn new(
		recipient: I,
		relayer: I,
		ext_amount: A,
		fee: B,
		refund: B,
		token: C,
		encrypted_output1: Vec<u8>,
		encrypted_output2: Vec<u8>,
	) -> Self {
		Self {
			recipient,
			relayer,
			ext_amount,
			fee,
			refund,
			token,
			encrypted_output1,
			encrypted_output2,
		}
	}
}

impl<I: Encode, A: Encode, B: Encode, C: Encode> IntoAbiToken for ExtData<I, A, B, C> {
	fn into_abi(&self) -> Token {
		let recipient = Token::Bytes(self.recipient.encode());
		let ext_amount = Token::Bytes(self.ext_amount.encode());
		let relayer = Token::Bytes(self.relayer.encode());
		let fee = Token::Bytes(self.fee.encode());
		let refund = Token::Bytes(self.refund.encode());
		let token = Token::Bytes(self.token.encode());
		let encrypted_output1 = Token::Bytes(self.encrypted_output1.clone());
		let encrypted_output2 = Token::Bytes(self.encrypted_output2.clone());
		// tuple(bytes recipient,bytes extAmount,bytes relayer,bttes fee,bytes
		// refund,bytes token,bytes encryptedOutput1,bytes encryptedOutput2)
		let ext_data_args = vec![
			recipient,
			ext_amount,
			relayer,
			fee,
			refund,
			token,
			encrypted_output1,
			encrypted_output2,
		];
		Token::Tuple(ext_data_args)
	}
}
