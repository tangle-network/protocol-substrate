use crate::{
	mock::*,
	test_utils::{get_hash_params, prove, setup_circuit_with_raw_inputs, setup_keys, setup_random_circuit, verify},
};
use arkworks_utils::utils::common::Curve;
use darkwebb_primitives::{
	merkle_tree::TreeInspector,
	types::vanchor::{ExtData, ProofData},
	AccountId,
};
use frame_benchmarking::account;
use frame_support::{assert_ok, traits::OnInitialize};

const SEED: u32 = 0;
const TREE_DEPTH: usize = 30;
const M: usize = 2;
const DEFAULT_BALANCE: u128 = 100_000_000;

pub fn get_account(id: u32) -> AccountId {
	account::<AccountId>("", id, SEED)
}

fn setup_environment(curve: Curve) -> (Vec<u8>, Vec<u8>) {
	let (_, params3, ..) = get_hash_params::<ark_bn254::Fr>(Curve::Bn254);
	// 1. Setup The Hasher Pallet.
	assert_ok!(HasherPallet::force_set_parameters(Origin::root(), params3.to_bytes()));
	// 2. Initialize MerkleTree pallet.
	<MerkleTree as OnInitialize<u64>>::on_initialize(1);
	// 3. Setup the VerifierPallet
	//    but to do so, we need to have a VerifyingKey

	let circuit = setup_random_circuit();
	let (proving_key_bytes, verifier_key_bytes) = setup_keys(circuit);

	assert_ok!(VerifierPallet::force_set_parameters(
		Origin::root(),
		verifier_key_bytes.clone()
	));
	// 4. and top-up some accounts with some balance
	for account_id in [
		account::<AccountId>("", 1, SEED),
		account::<AccountId>("", 2, SEED),
		account::<AccountId>("", 3, SEED),
		account::<AccountId>("", 4, SEED),
		account::<AccountId>("", 5, SEED),
		account::<AccountId>("", 6, SEED),
	] {
		assert_ok!(Balances::set_balance(Origin::root(), account_id, DEFAULT_BALANCE, 0));
	}

	// finally return the provingkey bytes
	(proving_key_bytes, verifier_key_bytes)
}

fn create_vanchor(asset_id: u32) -> u32 {
	let max_edges = M as u32;
	let depth = TREE_DEPTH as u8;
	assert_ok!(VAnchor::create(Origin::root(), max_edges, depth, asset_id));
	MerkleTree::next_tree_id() - 1
}

fn create_vanchor_with_deposits(amounts: &Vec<Balance>, leaves: &Vec<Element>) -> (u32, Element) {
	let tree_id = create_vanchor(0);

	// TODO: Use transact function to insert leafs
	for (leaf, amount) in leaves.iter().zip(amounts.iter()) {
		VAnchor::deposit(Origin::signed(get_account(1)), tree_id, *leaf, *amount).unwrap();
	}

	let on_chain_root = MerkleTree::get_root(tree_id).unwrap();

	(tree_id, on_chain_root)
}

#[test]
fn should_complete_2x2_transaction_with_deposit() {
	new_test_ext().execute_with(|| {
		let (pub_key_bytes, _) = setup_environment(Curve::Bn254);

		let recipient: AccountId = get_account(4);
		let relayer: AccountId = get_account(3);
		let ext_amount: Amount = 10;
		let fee: Balance = 2;

		let public_amount = 8;
		let in_chain_id = 0;
		let in_amounts = vec![5, 5];
		let out_chain_ids = vec![0, 0];
		let out_amounts = vec![5, 13];

		let (circuit, _chain_id_el, public_amount_el, root_set, nullifiers, leaves, commitments, ext_data_hash, _) =
			setup_circuit_with_raw_inputs(
				public_amount,
				recipient.clone(),
				relayer.clone(),
				ext_amount,
				fee,
				in_chain_id,
				in_amounts.clone(),
				out_chain_ids,
				out_amounts,
			);

		let proof = prove(circuit, pub_key_bytes);

		let (tree_id, on_chain_root) = create_vanchor_with_deposits(&in_amounts, &leaves);
		assert_eq!(root_set[0], on_chain_root);

		let output1 = commitments[0].clone();
		let output2 = commitments[1].clone();
		let ext_data = ExtData::<AccountId, Amount, Balance, Element>::new(
			recipient.clone(),
			relayer.clone(),
			ext_amount,
			fee,
			output1,
			output2,
		);

		let proof_data = ProofData::new(
			proof,
			root_set,
			nullifiers,
			commitments,
			public_amount_el,
			ext_data_hash,
		);

		assert_ok!(VAnchor::transact(
			Origin::signed(recipient.clone()),
			tree_id,
			proof_data,
			ext_data
		));

		let relayer_balance_after = Balances::free_balance(relayer);
		assert_eq!(relayer_balance_after, DEFAULT_BALANCE + fee);

		let recipient_balance_after = Balances::free_balance(recipient);
		assert_eq!(recipient_balance_after, DEFAULT_BALANCE - ext_amount as u128);
	});
}

#[test]
fn should_complete_2x2_transaction_with_withdraw() {
	new_test_ext().execute_with(|| {
		let (proving_key_bytes, verifying_key_bytes) = setup_environment(Curve::Bn254);

		let recipient: AccountId = get_account(4);
		let relayer: AccountId = get_account(3);
		let ext_amount: Amount = -5;
		let fee: Balance = 2;

		let public_amount = -7;
		let in_chain_id = 0;
		let in_amounts = vec![5, 5];
		let out_chain_ids = vec![0, 0];
		// After withdrawing -7
		let out_amounts = vec![1, 2];

		let (
			circuit,
			_chain_id_el,
			public_amount_el,
			root_set,
			nullifiers,
			leaves,
			commitments,
			ext_data_hash,
			public_inputs,
		) = setup_circuit_with_raw_inputs(
			public_amount,
			recipient.clone(),
			relayer.clone(),
			ext_amount,
			fee,
			in_chain_id,
			in_amounts.clone(),
			out_chain_ids,
			out_amounts,
		);

		let proof = prove(circuit, proving_key_bytes);
		let ver = verify(public_inputs, &verifying_key_bytes, &proof);
		assert!(ver);

		let (tree_id, on_chain_root) = create_vanchor_with_deposits(&in_amounts, &leaves);
		assert_eq!(root_set[0], on_chain_root);

		let output1 = commitments[0].clone();
		let output2 = commitments[1].clone();
		let ext_data = ExtData::<AccountId, Amount, Balance, Element>::new(
			recipient.clone(),
			relayer.clone(),
			ext_amount,
			fee,
			output1,
			output2,
		);

		let proof_data = ProofData::new(
			proof,
			root_set,
			nullifiers,
			commitments,
			public_amount_el,
			ext_data_hash,
		);

		assert_ok!(VAnchor::transact(
			Origin::signed(recipient.clone()),
			tree_id,
			proof_data,
			ext_data
		));

		let relayer_balance_after = Balances::free_balance(relayer);
		assert_eq!(relayer_balance_after, DEFAULT_BALANCE + fee);

		let recipient_balance_after = Balances::free_balance(recipient);
		assert_eq!(recipient_balance_after, DEFAULT_BALANCE + (-ext_amount as u128));
	});
}
