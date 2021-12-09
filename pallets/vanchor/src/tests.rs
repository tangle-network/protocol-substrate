use crate::{
	mock::*,
	test_utils::{
		get_hash_params, prove, setup_circuit_with_data_raw, setup_circuit_with_input_utxos_raw, setup_keys,
		setup_random_circuit, verify, Utxos,
	},
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
		account::<AccountId>("", 0, SEED),
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

fn create_vanchor_with_deposits(amounts: Vec<Balance>, proving_key_bytes: &Vec<u8>) -> (u32, Utxos) {
	let tree_id = create_vanchor(0);

	let transactor = get_account(0);
	let recipient: AccountId = get_account(0);
	let relayer: AccountId = get_account(0);
	let ext_amount: Amount = 10;
	let fee: Balance = 0;

	let public_amount = 10;
	let in_chain_id = 0;
	let in_amounts = vec![0, 0];

	let out_chain_ids = vec![0, 0];

	let (circuit, public_inputs_el, _, _, out_utxos) = setup_circuit_with_data_raw(
		public_amount,
		recipient.clone(),
		relayer.clone(),
		ext_amount,
		fee,
		in_chain_id,
		in_amounts.clone(),
		out_chain_ids,
		amounts,
	);

	let proof = prove(circuit, proving_key_bytes);

	// Deconstructing public inputs
	let public_amount = public_inputs_el[1];
	let root_set = public_inputs_el[1..3].to_vec();
	let nullifiers = public_inputs_el[3..5].to_vec();
	let commitments = public_inputs_el[5..7].to_vec();
	let ext_data_hash = public_inputs_el[8];

	// Constructing external data
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

	// Constructing proof data
	let proof_data = ProofData::new(proof, public_amount, root_set, nullifiers, commitments, ext_data_hash);

	assert_ok!(VAnchor::transact(
		Origin::signed(transactor),
		tree_id,
		proof_data,
		ext_data
	));

	(tree_id, out_utxos)
}

#[test]
fn should_complete_2x2_transaction_with_deposit() {
	new_test_ext().execute_with(|| {
		let (proving_key_bytes, verifying_key_bytes) = setup_environment(Curve::Bn254);
		let initial_amounts = vec![5, 5];
		let (tree_id, in_utxos) = create_vanchor_with_deposits(initial_amounts, &proving_key_bytes);

		let recipient: AccountId = get_account(4);
		let relayer: AccountId = get_account(3);
		let ext_amount: Amount = 10;
		let fee: Balance = 2;

		let public_amount = 8;
		let out_chain_ids = vec![0, 0];
		let out_amounts = vec![5, 13];

		let (circuit, public_inputs_el, public_inputs_f, ..) = setup_circuit_with_input_utxos_raw(
			public_amount,
			recipient.clone(),
			relayer.clone(),
			ext_amount,
			fee,
			in_utxos,
			out_chain_ids,
			out_amounts.to_vec(),
		);

		let proof = prove(circuit, &proving_key_bytes);

		// Check locally
		let res = verify(public_inputs_f, &verifying_key_bytes, &proof);
		assert!(res);

		// Deconstructing public inputs
		let public_amount = public_inputs_el[1];
		let root_set = public_inputs_el[1..3].to_vec();
		let nullifiers = public_inputs_el[3..5].to_vec();
		let commitments = public_inputs_el[5..7].to_vec();
		let ext_data_hash = public_inputs_el[8];

		// Constructing external data
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

		// Constructing proof data
		let proof_data = ProofData::new(proof, public_amount, root_set, nullifiers, commitments, ext_data_hash);

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
		let initial_amounts = vec![5, 5];
		let (tree_id, in_utxos) = create_vanchor_with_deposits(initial_amounts, &proving_key_bytes);

		let recipient: AccountId = get_account(4);
		let relayer: AccountId = get_account(3);
		let ext_amount: Amount = -5;
		let fee: Balance = 2;

		let public_amount = -7;
		let out_chain_ids = vec![0, 0];
		// After withdrawing -7
		let out_amounts = vec![1, 2];

		let (circuit, public_inputs_el, public_inputs_f, ..) = setup_circuit_with_input_utxos_raw(
			public_amount,
			recipient.clone(),
			relayer.clone(),
			ext_amount,
			fee,
			in_utxos,
			out_chain_ids,
			out_amounts.to_vec(),
		);

		let proof = prove(circuit, &proving_key_bytes);

		// Check locally
		let res = verify(public_inputs_f, &verifying_key_bytes, &proof);
		assert!(res);

		// Deconstructing public inputs
		let public_amount = public_inputs_el[1];
		let root_set = public_inputs_el[1..3].to_vec();
		let nullifiers = public_inputs_el[3..5].to_vec();
		let commitments = public_inputs_el[5..7].to_vec();
		let ext_data_hash = public_inputs_el[8];

		// Constructing external data
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

		// Constructing proof data
		let proof_data = ProofData::new(proof, public_amount, root_set, nullifiers, commitments, ext_data_hash);

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
