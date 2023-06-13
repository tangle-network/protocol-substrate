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
	// (bytes recipient,bytes extAmount,bytes relayer,bytes fee,bytes
	// refund,bytes token,bytes encryptedOutput1,bytes encryptedOutput2)
	fn into_abi(&self) -> Token {
		// make sure every field is encoded as BE bytes
		// Recipient is already encoded as BE bytes
		let recipient = Token::Bytes(self.recipient.encode());
		// Ext amount is encoded as LE bytes, so we need to reverse it
		let mut ext_amount_bytes = self.ext_amount.encode();
		ext_amount_bytes.reverse();
		let ext_amount = Token::Bytes(ext_amount_bytes);
		// Relayer is already encoded as BE bytes
		let relayer = Token::Bytes(self.relayer.encode());
		// Fee is encoded as LE bytes, so we need to reverse it
		let mut fee_bytes = self.fee.encode();
		fee_bytes.reverse();
		let fee = Token::Bytes(fee_bytes);
		// Refund is encoded as LE bytes, so we need to reverse it
		let mut refund_bytes = self.refund.encode();
		refund_bytes.reverse();
		let refund = Token::Bytes(refund_bytes);
		// Token is already encoded as BE bytes
		let token = Token::Bytes(self.token.encode());
		// Do not reverse encrypted output bytes
		// Encrypted output(s) is already encoded as BE bytes
		let encrypted_output1 = Token::Bytes(self.encrypted_output1.clone());
		let encrypted_output2 = Token::Bytes(self.encrypted_output2.clone());
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
