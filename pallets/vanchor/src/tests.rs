use crate::{
	mock::*,
	test_utils::{
		deconstruct_public_inputs_el, get_hash_params, prove, setup_circuit_with_data_raw,
		setup_circuit_with_input_utxos_raw, setup_keys, setup_random_circuit, verify, Utxos,
	},
	Error,
};
use arkworks_utils::utils::common::Curve;
use frame_benchmarking::account;
use frame_support::{assert_err, assert_ok, traits::OnInitialize};
use webb_primitives::{
	types::vanchor::{ExtData, ProofData},
	AccountId,
};

const SEED: u32 = 0;
const TREE_DEPTH: usize = 30;
const M: usize = 2;
const DEFAULT_BALANCE: u128 = 10;
const BIG_DEFAULT_BALANCE: u128 = 20;
const BIGGER_DEFAULT_BALANCE: u128 = 30;

const TRANSACTOR_ACCOUNT_ID: u32 = 0;
const RECIPIENT_ACCOUNT_ID: u32 = 1;
const BIG_TRANSACTOR_ACCOUNT_ID: u32 = 2;
const BIGGER_TRANSACTOR_ACCOUNT_ID: u32 = 3;
const RELAYER_ACCOUNT_ID: u32 = 4;

pub fn get_account(id: u32) -> AccountId {
	account::<AccountId>("", id, SEED)
}

fn setup_environment(_curve: Curve) -> (Vec<u8>, Vec<u8>) {
	let (_, params3, ..) = get_hash_params::<ark_bn254::Fr>(Curve::Bn254);
	// 1. Setup The Hasher Pallet.
	assert_ok!(HasherPallet::force_set_parameters(Origin::root(), params3.to_bytes()));
	// 2. Initialize MerkleTree pallet.
	<MerkleTree as OnInitialize<u64>>::on_initialize(1);
	// 3. Setup the VerifierPallet
	//    but to do so, we need to have a VerifyingKey

	let circuit = setup_random_circuit();
	let (proving_key_bytes, verifier_key_bytes) = setup_keys(circuit);

	assert_ok!(VerifierPallet::force_set_parameters(Origin::root(), verifier_key_bytes.clone()));

	let transactor = account::<AccountId>("", TRANSACTOR_ACCOUNT_ID, SEED);
	let big_transactor = account::<AccountId>("", BIG_TRANSACTOR_ACCOUNT_ID, SEED);
	let bigger_transactor = account::<AccountId>("", BIGGER_TRANSACTOR_ACCOUNT_ID, SEED);

	// Transactor should have some amount
	assert_ok!(Balances::set_balance(Origin::root(), transactor, DEFAULT_BALANCE, 0));

	// Big transactor should have even more amount
	assert_ok!(Balances::set_balance(Origin::root(), big_transactor, BIG_DEFAULT_BALANCE, 0));

	assert_ok!(Balances::set_balance(Origin::root(), bigger_transactor, BIGGER_DEFAULT_BALANCE, 0));

	// finally return the provingkey bytes
	(proving_key_bytes, verifier_key_bytes)
}

fn create_vanchor(asset_id: u32) -> u32 {
	let max_edges = M as u32;
	let depth = TREE_DEPTH as u8;
	assert_ok!(VAnchor::create(Origin::root(), max_edges, depth, asset_id));
	MerkleTree::next_tree_id() - 1
}

fn create_vanchor_with_deposits(proving_key_bytes: &Vec<u8>) -> (u32, Utxos) {
	let tree_id = create_vanchor(0);

	let transactor = get_account(TRANSACTOR_ACCOUNT_ID);
	let recipient = get_account(RECIPIENT_ACCOUNT_ID);
	let relayer: AccountId = get_account(RELAYER_ACCOUNT_ID);
	let ext_amount: Amount = DEFAULT_BALANCE as i128;
	let fee: Balance = 0;

	let public_amount = DEFAULT_BALANCE as i128;
	let in_chain_id = 0;
	let in_amounts = vec![0, 0];
	let out_chain_ids = vec![0, 0];
	let out_amounts = vec![DEFAULT_BALANCE, 0];

	let (circuit, public_inputs, _, out_utxos) = setup_circuit_with_data_raw(
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

	// Deconstructing public inputs
	let (_chain_id, public_amount, root_set, nullifiers, commitments, ext_data_hash) =
		deconstruct_public_inputs_el(&public_inputs);

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
	let proof_data =
		ProofData::new(proof, public_amount, root_set, nullifiers, commitments, ext_data_hash);

	assert_ok!(VAnchor::transact(Origin::signed(transactor), tree_id, proof_data, ext_data));

	(tree_id, out_utxos)
}

#[test]
fn should_complete_2x2_transaction_with_deposit() {
	new_test_ext().execute_with(|| {
		let (proving_key_bytes, verifying_key_bytes) = setup_environment(Curve::Bn254);
		let tree_id = create_vanchor(0);

		let transactor = get_account(TRANSACTOR_ACCOUNT_ID);
		let recipient: AccountId = get_account(RECIPIENT_ACCOUNT_ID);
		let relayer: AccountId = get_account(RELAYER_ACCOUNT_ID);

		let ext_amount: Amount = DEFAULT_BALANCE as i128;
		let public_amount = DEFAULT_BALANCE as i128;
		let fee: Balance = 0;

		let in_chain_id = 0;
		let in_amounts = vec![0, 0];
		let out_chain_ids = vec![0, 0];
		let out_amounts = vec![DEFAULT_BALANCE, 0];

		let (circuit, public_inputs_f, ..) = setup_circuit_with_data_raw(
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

		let proof = prove(circuit, &proving_key_bytes);

		// Check locally
		let res = verify(&public_inputs_f, &verifying_key_bytes, &proof);
		assert!(res);

		// Deconstructing public inputs
		let (_chain_id, public_amount, root_set, nullifiers, commitments, ext_data_hash) =
			deconstruct_public_inputs_el(&public_inputs_f);

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
		let proof_data =
			ProofData::new(proof, public_amount, root_set, nullifiers, commitments, ext_data_hash);

		assert_ok!(VAnchor::transact(
			Origin::signed(transactor.clone()),
			tree_id,
			proof_data,
			ext_data
		));

		// Relayer balance should be zero since the fee was zero
		let relayer_balance_after = Balances::free_balance(relayer);
		assert_eq!(relayer_balance_after, 0);

		// Transactor balance should be zero, since they deposited all the
		// money to the mixer
		let transactor_balance_after = Balances::free_balance(transactor);
		assert_eq!(transactor_balance_after, 0);
	});
}

#[test]
fn should_complete_2x2_transaction_with_withdraw() {
	new_test_ext().execute_with(|| {
		let (proving_key_bytes, verifying_key_bytes) = setup_environment(Curve::Bn254);
		let (tree_id, in_utxos) = create_vanchor_with_deposits(&proving_key_bytes);

		let transactor: AccountId = get_account(TRANSACTOR_ACCOUNT_ID);
		let recipient: AccountId = get_account(RECIPIENT_ACCOUNT_ID);
		let relayer: AccountId = get_account(RELAYER_ACCOUNT_ID);
		let ext_amount: Amount = -5;
		let fee: Balance = 2;

		let public_amount = -7;
		let out_chain_ids = vec![0, 0];
		// After withdrawing -7
		let out_amounts = vec![1, 2];

		let (circuit, public_inputs_f, ..) = setup_circuit_with_input_utxos_raw(
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
		let res = verify(&public_inputs_f, &verifying_key_bytes, &proof);
		assert!(res);

		// Deconstructing public inputs
		let (_chain_id, public_amount, root_set, nullifiers, commitments, ext_data_hash) =
			deconstruct_public_inputs_el(&public_inputs_f);

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
		let proof_data =
			ProofData::new(proof, public_amount, root_set, nullifiers, commitments, ext_data_hash);

		assert_ok!(VAnchor::transact(
			Origin::signed(transactor.clone()),
			tree_id,
			proof_data,
			ext_data
		));

		// Should be equal to the `fee` since the transaction was sucessful
		let relayer_balance_after = Balances::free_balance(relayer);
		assert_eq!(relayer_balance_after, fee);

		// Should be equal to the amount that is withdrawn
		let recipient_balance_after = Balances::free_balance(recipient);
		assert_eq!(recipient_balance_after, ext_amount.unsigned_abs());
	});
}

#[test]
fn should_not_complete_transaction_if_ext_data_is_invalid() {
	new_test_ext().execute_with(|| {
		let (proving_key_bytes, verifying_key_bytes) = setup_environment(Curve::Bn254);
		let tree_id = create_vanchor(0);

		let transactor = get_account(TRANSACTOR_ACCOUNT_ID);
		let recipient: AccountId = get_account(RECIPIENT_ACCOUNT_ID);
		let relayer: AccountId = get_account(RELAYER_ACCOUNT_ID);

		let ext_amount: Amount = DEFAULT_BALANCE as i128;
		let public_amount = DEFAULT_BALANCE as i128;
		let fee: Balance = 0;

		let in_chain_id = 0;
		let in_amounts = vec![0, 0];
		let out_chain_ids = vec![0, 0];
		let out_amounts = vec![DEFAULT_BALANCE, 0];

		let (circuit, public_inputs_f, ..) = setup_circuit_with_data_raw(
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

		let proof = prove(circuit, &proving_key_bytes);

		// Check locally
		let res = verify(&public_inputs_f, &verifying_key_bytes, &proof);
		assert!(res);

		// Deconstructing public inputs
		let (_chain_id, public_amount, root_set, nullifiers, commitments, ext_data_hash) =
			deconstruct_public_inputs_el(&public_inputs_f);

		// Constructing external data
		let output1 = commitments[0].clone();

		// INVALID output commitment
		let output2 = Element::from_bytes(&[0u8; 32]);
		let ext_data = ExtData::<AccountId, Amount, Balance, Element>::new(
			recipient.clone(),
			relayer.clone(),
			ext_amount,
			fee,
			output1,
			output2,
		);

		// Constructing proof data
		let proof_data =
			ProofData::new(proof, public_amount, root_set, nullifiers, commitments, ext_data_hash);

		assert_err!(
			VAnchor::transact(Origin::signed(transactor.clone()), tree_id, proof_data, ext_data),
			Error::<Test, _>::InvalidExtData,
		);

		// Relayer balance should be zero since the fee was zero and the transaction
		// failed
		let relayer_balance_after = Balances::free_balance(relayer);
		assert_eq!(relayer_balance_after, 0);

		// Transactor balance should be the default one, since the deposit failed
		let transactor_balance_after = Balances::free_balance(transactor);
		assert_eq!(transactor_balance_after, DEFAULT_BALANCE);

		// Recipient balance should be zero since the withdraw was not successful
		let recipient_balance_after = Balances::free_balance(recipient);
		assert_eq!(recipient_balance_after, 0);
	});
}

#[test]
fn should_not_complete_withdraw_if_out_amount_sum_is_too_big() {
	new_test_ext().execute_with(|| {
		let (proving_key_bytes, _) = setup_environment(Curve::Bn254);
		let (tree_id, in_utxos) = create_vanchor_with_deposits(&proving_key_bytes);

		let transactor = get_account(TRANSACTOR_ACCOUNT_ID);
		let recipient: AccountId = get_account(RECIPIENT_ACCOUNT_ID);
		let relayer: AccountId = get_account(RELAYER_ACCOUNT_ID);

		let public_amount = -7;
		let ext_amount: Amount = -5;
		let fee: Balance = 2;

		let out_chain_ids = vec![0, 0];
		// Withdraw amount too big
		let out_amounts = vec![100, 200];

		let (circuit, public_inputs_el, ..) = setup_circuit_with_input_utxos_raw(
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

		// Deconstructing public inputs
		let (_chain_id, public_amount, root_set, nullifiers, commitments, ext_data_hash) =
			deconstruct_public_inputs_el(&public_inputs_el);

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
		let proof_data =
			ProofData::new(proof, public_amount, root_set, nullifiers, commitments, ext_data_hash);

		// Should fail with invalid external data error
		assert_err!(
			VAnchor::transact(Origin::signed(transactor.clone()), tree_id, proof_data, ext_data),
			Error::<Test, _>::InvalidTransactionProof
		);

		// Should be zero, since transaction failed
		let relayer_balance_after = Balances::free_balance(relayer);
		assert_eq!(relayer_balance_after, 0);

		// Transactors balance is zero since they deposited all of their money to the
		// mixer
		let transactor_balance_after = Balances::free_balance(transactor);
		assert_eq!(transactor_balance_after, 0);

		// Recipient balance is zero, since the withdraw failed
		let recipient_balance_after = Balances::free_balance(recipient);
		assert_eq!(recipient_balance_after, 0);
	});
}

#[test]
fn should_not_complete_withdraw_if_out_amount_sum_is_too_small() {
	new_test_ext().execute_with(|| {
		let (proving_key_bytes, _) = setup_environment(Curve::Bn254);
		let (tree_id, in_utxos) = create_vanchor_with_deposits(&proving_key_bytes);

		let transactor = get_account(TRANSACTOR_ACCOUNT_ID);
		let recipient: AccountId = get_account(RECIPIENT_ACCOUNT_ID);
		let relayer: AccountId = get_account(RELAYER_ACCOUNT_ID);

		let ext_amount: Amount = -5;
		let fee: Balance = 2;

		let public_amount = -7;
		let out_chain_ids = vec![0, 0];
		// Withdraw amount too small
		let out_amounts = vec![1, 0];

		let (circuit, public_inputs_el, ..) = setup_circuit_with_input_utxos_raw(
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

		// Deconstructing public inputs
		let (_chain_id, public_amount, root_set, nullifiers, commitments, ext_data_hash) =
			deconstruct_public_inputs_el(&public_inputs_el);

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
		let proof_data =
			ProofData::new(proof, public_amount, root_set, nullifiers, commitments, ext_data_hash);

		// Should fail with invalid external data error
		assert_err!(
			VAnchor::transact(Origin::signed(transactor.clone()), tree_id, proof_data, ext_data),
			Error::<Test, _>::InvalidTransactionProof
		);

		// Should be zero, since transaction failed
		let relayer_balance_after = Balances::free_balance(relayer);
		assert_eq!(relayer_balance_after, 0);

		// Transactors balance is zero since they deposited all of their money to the
		// mixer
		let transactor_balance_after = Balances::free_balance(transactor);
		assert_eq!(transactor_balance_after, 0);

		// Recipient balance is zero, since the withdraw failed
		let recipient_balance_after = Balances::free_balance(recipient);
		assert_eq!(recipient_balance_after, 0);
	});
}

#[test]
fn should_not_be_able_to_double_spend() {
	new_test_ext().execute_with(|| {
		let (proving_key_bytes, verifying_key_bytes) = setup_environment(Curve::Bn254);
		let (tree_id, in_utxos) = create_vanchor_with_deposits(&proving_key_bytes);

		let transactor: AccountId = get_account(TRANSACTOR_ACCOUNT_ID);
		let recipient: AccountId = get_account(RECIPIENT_ACCOUNT_ID);
		let relayer: AccountId = get_account(RELAYER_ACCOUNT_ID);
		let ext_amount: Amount = -5;
		let fee: Balance = 2;

		let public_amount = -7;
		let out_chain_ids = vec![0, 0];
		// After withdrawing -7
		let out_amounts = vec![1, 2];

		let (circuit, public_inputs_f, ..) = setup_circuit_with_input_utxos_raw(
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
		let res = verify(&public_inputs_f, &verifying_key_bytes, &proof);
		assert!(res);

		// Deconstructing public inputs
		let (_chain_id, public_amount, root_set, nullifiers, commitments, ext_data_hash) =
			deconstruct_public_inputs_el(&public_inputs_f);

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
		let proof_data =
			ProofData::new(proof, public_amount, root_set, nullifiers, commitments, ext_data_hash);

		assert_ok!(VAnchor::transact(
			Origin::signed(transactor.clone()),
			tree_id,
			proof_data.clone(),
			ext_data.clone()
		));
		assert_err!(
			VAnchor::transact(Origin::signed(transactor.clone()), tree_id, proof_data, ext_data),
			Error::<Test, _>::AlreadyRevealedNullifier
		);

		// Fee is paid out once
		let relayer_balance_after = Balances::free_balance(relayer);
		assert_eq!(relayer_balance_after, fee);

		// Recipient is paid out once
		let recipient_balance_after = Balances::free_balance(recipient);
		assert_eq!(recipient_balance_after, (-ext_amount as u128));

		// Transactor is 0 after one deposit
		let transactor_balance_after = Balances::free_balance(transactor);
		assert_eq!(transactor_balance_after, 0);
	});
}

#[test]
fn should_not_be_able_to_exceed_max_fee() {
	new_test_ext().execute_with(|| {
		let (proving_key_bytes, verifying_key_bytes) = setup_environment(Curve::Bn254);
		let tree_id = create_vanchor(0);

		let transactor = get_account(TRANSACTOR_ACCOUNT_ID);
		let recipient: AccountId = get_account(RECIPIENT_ACCOUNT_ID);
		let relayer: AccountId = get_account(RELAYER_ACCOUNT_ID);

		let ext_amount: Amount = DEFAULT_BALANCE as i128;
		let public_amount = 4;
		let fee: Balance = 6;

		let in_chain_id = 0;
		let in_amounts = vec![0, 0];
		let out_chain_ids = vec![0, 0];
		let out_amounts = vec![4, 0];

		let (circuit, public_inputs_f, ..) = setup_circuit_with_data_raw(
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

		let proof = prove(circuit, &proving_key_bytes);

		// Check locally
		let res = verify(&public_inputs_f, &verifying_key_bytes, &proof);
		assert!(res);

		// Deconstructing public inputs
		let (_chain_id, public_amount, root_set, nullifiers, commitments, ext_data_hash) =
			deconstruct_public_inputs_el(&public_inputs_f);

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
		let proof_data =
			ProofData::new(proof, public_amount, root_set, nullifiers, commitments, ext_data_hash);

		assert_err!(
			VAnchor::transact(Origin::signed(transactor.clone()), tree_id, proof_data, ext_data),
			Error::<Test, _>::InvalidFee
		);

		// Relayer balance should be zero since the fee was zero
		let relayer_balance_after = Balances::free_balance(relayer);
		assert_eq!(relayer_balance_after, 0);

		// Transactor balance should not be changed, since the transaction has failed
		let transactor_balance_after = Balances::free_balance(transactor);
		assert_eq!(transactor_balance_after, DEFAULT_BALANCE);
	});
}

#[test]
fn should_not_be_able_to_exceed_max_deposit() {
	new_test_ext().execute_with(|| {
		let (proving_key_bytes, verifying_key_bytes) = setup_environment(Curve::Bn254);
		let tree_id = create_vanchor(0);

		let transactor = get_account(BIG_TRANSACTOR_ACCOUNT_ID);
		let recipient: AccountId = get_account(RECIPIENT_ACCOUNT_ID);
		let relayer: AccountId = get_account(RELAYER_ACCOUNT_ID);

		let ext_amount: Amount = BIG_DEFAULT_BALANCE as i128;
		let public_amount = BIG_DEFAULT_BALANCE as i128;
		let fee: Balance = 0;

		let in_chain_id = 0;
		let in_amounts = vec![0, 0];
		let out_chain_ids = vec![0, 0];
		let out_amounts = vec![BIG_DEFAULT_BALANCE, 0];

		let (circuit, public_inputs_f, ..) = setup_circuit_with_data_raw(
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

		let proof = prove(circuit, &proving_key_bytes);

		// Check locally
		let res = verify(&public_inputs_f, &verifying_key_bytes, &proof);
		assert!(res);

		// Deconstructing public inputs
		let (_chain_id, public_amount, root_set, nullifiers, commitments, ext_data_hash) =
			deconstruct_public_inputs_el(&public_inputs_f);

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
		let proof_data =
			ProofData::new(proof, public_amount, root_set, nullifiers, commitments, ext_data_hash);

		assert_err!(
			VAnchor::transact(Origin::signed(transactor.clone()), tree_id, proof_data, ext_data),
			Error::<Test, _>::InvalidDepositAmount
		);

		// Relayer balance should be zero since the fee was zero
		let relayer_balance_after = Balances::free_balance(relayer);
		assert_eq!(relayer_balance_after, 0);

		// Transactor balance should not be changed, since the transaction has failed
		let transactor_balance_after = Balances::free_balance(transactor);
		assert_eq!(transactor_balance_after, BIG_DEFAULT_BALANCE);
	});
}

#[test]
fn should_not_be_able_to_exceed_external_amount() {
	new_test_ext().execute_with(|| {
		let (proving_key_bytes, verifying_key_bytes) = setup_environment(Curve::Bn254);
		let tree_id = create_vanchor(0);

		let transactor = get_account(BIGGER_TRANSACTOR_ACCOUNT_ID);
		let recipient: AccountId = get_account(RECIPIENT_ACCOUNT_ID);
		let relayer: AccountId = get_account(RELAYER_ACCOUNT_ID);

		// The external amount will be 3 more than allowed
		let ext_amount: Amount = 23;
		let public_amount = 20;
		let fee: Balance = 3;

		let in_chain_id = 0;
		let in_amounts = vec![0, 0];
		let out_chain_ids = vec![0, 0];
		let out_amounts = vec![20, 0];

		let (circuit, public_inputs_f, ..) = setup_circuit_with_data_raw(
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

		let proof = prove(circuit, &proving_key_bytes);

		// Check locally
		let res = verify(&public_inputs_f, &verifying_key_bytes, &proof);
		assert!(res);

		// Deconstructing public inputs
		let (_chain_id, public_amount, root_set, nullifiers, commitments, ext_data_hash) =
			deconstruct_public_inputs_el(&public_inputs_f);

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
		let proof_data =
			ProofData::new(proof, public_amount, root_set, nullifiers, commitments, ext_data_hash);

		assert_err!(
			VAnchor::transact(Origin::signed(transactor.clone()), tree_id, proof_data, ext_data),
			Error::<Test, _>::InvalidExtAmount
		);

		// Relayer balance should be zero since the transaction failed
		let relayer_balance_after = Balances::free_balance(relayer);
		assert_eq!(relayer_balance_after, 0);

		// Transactor balance should not be changed, since the transaction has failed
		let transactor_balance_after = Balances::free_balance(transactor);
		assert_eq!(transactor_balance_after, BIGGER_DEFAULT_BALANCE);
	});
}

#[test]
fn should_not_be_able_to_withdraw_less_than_minimum() {
	new_test_ext().execute_with(|| {
		let (proving_key_bytes, verifying_key_bytes) = setup_environment(Curve::Bn254);
		let (tree_id, in_utxos) = create_vanchor_with_deposits(&proving_key_bytes);

		let transactor: AccountId = get_account(TRANSACTOR_ACCOUNT_ID);
		let recipient: AccountId = get_account(RECIPIENT_ACCOUNT_ID);
		let relayer: AccountId = get_account(RELAYER_ACCOUNT_ID);
		let ext_amount: Amount = -2;
		let fee: Balance = 4;

		let public_amount = -6;
		let out_chain_ids = vec![0, 0];
		// After withdrawing -7
		let out_amounts = vec![2, 2];

		let (circuit, public_inputs_f, ..) = setup_circuit_with_input_utxos_raw(
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
		let res = verify(&public_inputs_f, &verifying_key_bytes, &proof);
		assert!(res);

		// Deconstructing public inputs
		let (_chain_id, public_amount, root_set, nullifiers, commitments, ext_data_hash) =
			deconstruct_public_inputs_el(&public_inputs_f);

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
		let proof_data =
			ProofData::new(proof, public_amount, root_set, nullifiers, commitments, ext_data_hash);

		assert_err!(
			VAnchor::transact(Origin::signed(transactor.clone()), tree_id, proof_data, ext_data),
			Error::<Test, _>::InvalidWithdrawAmount
		);

		// Fee is not paid out
		let relayer_balance_after = Balances::free_balance(relayer);
		assert_eq!(relayer_balance_after, 0);

		// Recipient is not paid
		let recipient_balance_after = Balances::free_balance(recipient);
		assert_eq!(recipient_balance_after, 0);

		let transactor_balance_after = Balances::free_balance(transactor);
		assert_eq!(transactor_balance_after, 0);
	});
}
