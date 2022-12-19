use crate::{
	mock::*,
	test_utils::{deconstruct_public_inputs_el, setup_utxos, setup_zk_circuit},
	Error, Instance1, MaxDepositAmount, MinWithdrawAmount,
};
use ark_ff::{BigInteger, PrimeField};
use arkworks_setups::{common::setup_params, utxo::Utxo, Curve};
use frame_benchmarking::account;
use frame_support::{assert_err, assert_ok, traits::OnInitialize};
use orml_traits::MultiCurrency;
use pallet_asset_registry::AssetType;
use pallet_linkable_tree::LinkableTreeConfigration;
use sp_core::hashing::keccak_256;
use std::convert::TryInto;
use webb_primitives::{
	linkable_tree::LinkableTreeInspector,
	merkle_tree::TreeInspector,
	types::vanchor::{ExtData, ProofData},
	utils::compute_chain_id_type,
	AccountId,
};

type Bn254Fr = ark_bn254::Fr;

const SEED: u32 = 0;
const TREE_DEPTH: usize = 30;
const EDGE_CT: usize = 1;
const DEFAULT_BALANCE: u128 = 10_000;
const BIG_DEFAULT_BALANCE: u128 = 20_000;
const BIGGER_DEFAULT_BALANCE: u128 = 30_000;

const TRANSACTOR_ACCOUNT_ID: u32 = 0;
const RECIPIENT_ACCOUNT_ID: u32 = 1;
const BIG_TRANSACTOR_ACCOUNT_ID: u32 = 2;
const BIGGER_TRANSACTOR_ACCOUNT_ID: u32 = 3;
const RELAYER_ACCOUNT_ID: u32 = 4;

pub fn get_account(id: u32) -> AccountId {
	account::<AccountId>("", id, SEED)
}

fn setup_environment() -> (Vec<u8>, Vec<u8>, Vec<u8>, Vec<u8>) {
	let curve = Curve::Bn254;
	let params3 = setup_params::<ark_bn254::Fr>(curve, 5, 3);
	// 1. Setup The Hasher Pallet.
	assert_ok!(Hasher1::force_set_parameters(RuntimeOrigin::root(), params3.to_bytes()));
	// 2. Initialize MerkleTree pallet.
	<MerkleTree1 as OnInitialize<u64>>::on_initialize(1);
	// 3. Setup the VerifierPallet
	//    but to do so, we need to have a VerifyingKey

	let pk_2_2_bytes = include_bytes!(
		"../../../substrate-fixtures/vanchor/bn254/x5/2-2-2/proving_key_uncompressed.bin"
	)
	.to_vec();
	let vk_2_2_bytes =
		include_bytes!("../../../substrate-fixtures/vanchor/bn254/x5/2-2-2/verifying_key.bin")
			.to_vec();

	let pk_2_16_bytes = include_bytes!(
		"../../../substrate-fixtures/vanchor/bn254/x5/2-16-2/proving_key_uncompressed.bin"
	)
	.to_vec();
	let vk_2_16_bytes =
		include_bytes!("../../../substrate-fixtures/vanchor/bn254/x5/2-16-2/verifying_key.bin")
			.to_vec();

	assert_ok!(VAnchorVerifier1::force_set_parameters(
		RuntimeOrigin::root(),
		(2, 2),
		vk_2_2_bytes.clone()
	));
	assert_ok!(VAnchorVerifier1::force_set_parameters(
		RuntimeOrigin::root(),
		(2, 16),
		vk_2_16_bytes.clone()
	));

	let transactor = account::<AccountId>("", TRANSACTOR_ACCOUNT_ID, SEED);
	let relayer = account::<AccountId>("", RELAYER_ACCOUNT_ID, SEED);
	let big_transactor = account::<AccountId>("", BIG_TRANSACTOR_ACCOUNT_ID, SEED);
	let bigger_transactor = account::<AccountId>("", BIGGER_TRANSACTOR_ACCOUNT_ID, SEED);

	// Set balances
	assert_ok!(Balances::set_balance(RuntimeOrigin::root(), transactor, DEFAULT_BALANCE, 0));
	assert_ok!(Balances::set_balance(RuntimeOrigin::root(), relayer, DEFAULT_BALANCE, 0));
	assert_ok!(Balances::set_balance(
		RuntimeOrigin::root(),
		big_transactor,
		BIG_DEFAULT_BALANCE,
		0
	));
	assert_ok!(Balances::set_balance(
		RuntimeOrigin::root(),
		bigger_transactor,
		BIGGER_DEFAULT_BALANCE,
		0
	));

	// set configurable storage
	assert_ok!(VAnchor1::set_max_deposit_amount(RuntimeOrigin::root(), 10, 1));
	assert_ok!(VAnchor1::set_min_withdraw_amount(RuntimeOrigin::root(), 3, 2));

	// finally return the provingkey bytes
	(pk_2_2_bytes, vk_2_2_bytes, pk_2_16_bytes, vk_2_16_bytes)
}

fn create_vanchor(asset_id: u32) -> u32 {
	let max_edges = EDGE_CT as u32;
	let depth = TREE_DEPTH as u8;
	assert_ok!(VAnchor1::create(RuntimeOrigin::root(), max_edges, depth, asset_id));
	MerkleTree1::next_tree_id() - 1
}

fn create_vanchor_with_deposits(
	proving_key_2x2_bytes: Vec<u8>,
	asset_id: Option<u32>,
) -> (u32, [Utxo<Bn254Fr>; 2]) {
	let tree_id = create_vanchor(asset_id.unwrap_or_default());

	let transactor = get_account(TRANSACTOR_ACCOUNT_ID);
	let recipient = get_account(RECIPIENT_ACCOUNT_ID);
	let relayer: AccountId = get_account(RELAYER_ACCOUNT_ID);
	let ext_amount: Amount = 10_i128;
	let fee: Balance = 0;

	let public_amount = 10_i128;

	let chain_type = [2, 0];
	let chain_id = compute_chain_id_type(ChainIdentifier::get(), chain_type);
	let in_chain_ids = [chain_id; 2];
	let in_amounts = [0, 0];
	let in_indices = [0, 1];
	let out_chain_ids = [chain_id; 2];
	let out_amounts = [10, 0];

	let in_utxos = setup_utxos(in_chain_ids, in_amounts, Some(in_indices));
	// We are adding indicies to out utxos, since they will be used as an input utxos in next
	// transaction
	let out_utxos = setup_utxos(out_chain_ids, out_amounts, Some(in_indices));

	let output1 = out_utxos[0].commitment.into_repr().to_bytes_be();
	let output2 = out_utxos[1].commitment.into_repr().to_bytes_be();
	let ext_data = ExtData::<AccountId, Amount, Balance, AssetId>::new(
		recipient,
		relayer,
		ext_amount,
		fee,
		0,
		asset_id.unwrap_or_default(),
		output1.to_vec(), // Mock encryption value, not meant to be used in production
		output2.to_vec(), // Mock encryption value, not meant to be used in production
	);

	let ext_data_hash = keccak_256(&ext_data.encode_abi());

	let custom_root = MerkleTree1::get_default_root(tree_id).unwrap();
	let neighbor_roots: [Element; EDGE_CT] = <LinkableTree1 as LinkableTreeInspector<
		LinkableTreeConfigration<Test, Instance1>,
	>>::get_neighbor_roots(tree_id)
	.unwrap()
	.try_into()
	.unwrap();
	let (proof, public_inputs) = setup_zk_circuit(
		public_amount,
		chain_id,
		ext_data_hash.to_vec(),
		in_utxos,
		out_utxos.clone(),
		proving_key_2x2_bytes,
		neighbor_roots,
		custom_root,
	);

	// Deconstructing public inputs
	let (_chain_id, public_amount, root_set, nullifiers, commitments, ext_data_hash) =
		deconstruct_public_inputs_el(&public_inputs);

	// Constructing proof data
	let proof_data =
		ProofData::new(proof, public_amount, root_set, nullifiers, commitments, ext_data_hash);

	assert_ok!(VAnchor1::transact(
		RuntimeOrigin::signed(transactor),
		tree_id,
		proof_data,
		ext_data
	));

	(tree_id, out_utxos)
}

#[test]
fn should_complete_2x2_transaction_with_deposit() {
	new_test_ext().execute_with(|| {
		let (proving_key_2x2_bytes, _, _, _) = setup_environment();
		let tree_id = create_vanchor(0);

		let transactor = get_account(TRANSACTOR_ACCOUNT_ID);
		let recipient: AccountId = get_account(RECIPIENT_ACCOUNT_ID);
		let relayer: AccountId = get_account(RELAYER_ACCOUNT_ID);

		let ext_amount: Amount = 10_i128;
		let public_amount = 10_i128;
		let fee: Balance = 0;

		let chain_type = [2, 0];
		let chain_id = compute_chain_id_type(ChainIdentifier::get(), chain_type);
		let in_chain_ids = [chain_id; 2];
		let in_amounts = [0, 0];
		let in_indices = [0, 1];
		let out_chain_ids = [chain_id; 2];
		let out_amounts = [10, 0];

		let in_utxos = setup_utxos(in_chain_ids, in_amounts, Some(in_indices));
		let out_utxos = setup_utxos(out_chain_ids, out_amounts, None);

		let output1 = out_utxos[0].commitment.into_repr().to_bytes_be();
		let output2 = out_utxos[1].commitment.into_repr().to_bytes_be();
		let ext_data = ExtData::<AccountId, Amount, Balance, AssetId>::new(
			recipient.clone(),
			relayer.clone(),
			ext_amount,
			fee,
			0,
			0,
			// Mock encryption value, not meant to be used in production
			output1.to_vec(),
			// Mock encryption value, not meant to be used in production
			output2.to_vec(),
		);

		let ext_data_hash = keccak_256(&ext_data.encode_abi());

		let custom_root = MerkleTree1::get_default_root(tree_id).unwrap();
		let neighbor_roots: [Element; EDGE_CT] = <LinkableTree1 as LinkableTreeInspector<
			LinkableTreeConfigration<Test, Instance1>,
		>>::get_neighbor_roots(tree_id)
		.unwrap()
		.try_into()
		.unwrap();

		let (proof, public_inputs) = setup_zk_circuit(
			public_amount,
			chain_id,
			ext_data_hash.to_vec(),
			in_utxos,
			out_utxos,
			proving_key_2x2_bytes,
			neighbor_roots,
			custom_root,
		);

		// Deconstructing public inputs
		let (_chain_id, public_amount, root_set, nullifiers, commitments, ext_data_hash) =
			deconstruct_public_inputs_el(&public_inputs);

		// Constructing external data
		let output1 = commitments[0];
		let output2 = commitments[1];
		let ext_data = ExtData::<AccountId, Amount, Balance, AssetId>::new(
			recipient.clone(),
			relayer.clone(),
			ext_amount,
			fee,
			0,
			0,
			output1.to_vec(),
			output2.to_vec(),
		);

		// Constructing proof data
		let proof_data =
			ProofData::new(proof, public_amount, root_set, nullifiers, commitments, ext_data_hash);

		let relayer_balance_before = Balances::free_balance(relayer.clone());
		let recipient_balance_before = Balances::free_balance(recipient.clone());
		let transactor_balance_before = Balances::free_balance(transactor.clone());
		assert_ok!(VAnchor1::transact(
			RuntimeOrigin::signed(transactor.clone()),
			tree_id,
			proof_data,
			ext_data
		));

		// Recipient balance should be ext amount since the fee was zero
		let recipient_balance_after = Balances::free_balance(recipient);
		assert_eq!(recipient_balance_after, recipient_balance_before);

		// Relayer balance should be zero since the fee was zero
		let relayer_balance_after = Balances::free_balance(relayer);
		assert_eq!(relayer_balance_after, relayer_balance_before);

		// Transactor balance should be zero, since they deposited all the
		// money to the mixer
		let transactor_balance_after = Balances::free_balance(transactor);
		assert_eq!(transactor_balance_after, transactor_balance_before - ext_amount.unsigned_abs());
	});
}

#[test]
fn should_complete_2x2_transaction_with_withdraw() {
	new_test_ext().execute_with(|| {
		let (proving_key_2x2_bytes, _, _, _) = setup_environment();
		let (tree_id, in_utxos) = create_vanchor_with_deposits(proving_key_2x2_bytes.clone(), None);
		let custom_root = MerkleTree1::get_root(tree_id).unwrap();

		let transactor: AccountId = get_account(TRANSACTOR_ACCOUNT_ID);
		let recipient: AccountId = get_account(RECIPIENT_ACCOUNT_ID);
		let relayer: AccountId = get_account(RELAYER_ACCOUNT_ID);
		let ext_amount: Amount = -5;
		let fee: Balance = 2;

		let public_amount = -7;

		let chain_type = [2, 0];
		let chain_id = compute_chain_id_type(ChainIdentifier::get(), chain_type);
		let out_chain_ids = [chain_id; 2];
		// After withdrawing -7
		let out_amounts = [1, 2];

		let out_utxos = setup_utxos(out_chain_ids, out_amounts, None);

		let output1 = out_utxos[0].commitment.into_repr().to_bytes_be();
		let output2 = out_utxos[1].commitment.into_repr().to_bytes_be();
		let ext_data = ExtData::<AccountId, Amount, Balance, AssetId>::new(
			recipient.clone(),
			relayer.clone(),
			ext_amount,
			fee,
			0,
			0,
			output1.to_vec(),
			output2.to_vec(),
		);

		let ext_data_hash = keccak_256(&ext_data.encode_abi());

		let neighbor_roots = <LinkableTree1 as LinkableTreeInspector<
			LinkableTreeConfigration<Test, Instance1>,
		>>::get_neighbor_roots(tree_id)
		.unwrap()
		.try_into()
		.unwrap();

		let (proof, public_inputs) = setup_zk_circuit(
			public_amount,
			chain_id,
			ext_data_hash.to_vec(),
			in_utxos,
			out_utxos,
			proving_key_2x2_bytes,
			neighbor_roots,
			custom_root,
		);

		// Deconstructing public inputs
		let (_chain_id, public_amount, root_set, nullifiers, commitments, ext_data_hash) =
			deconstruct_public_inputs_el(&public_inputs);

		// Constructing external data
		let output1 = commitments[0];
		let output2 = commitments[1];
		let ext_data = ExtData::<AccountId, Amount, Balance, AssetId>::new(
			recipient.clone(),
			relayer.clone(),
			ext_amount,
			fee,
			0,
			0,
			output1.to_vec(),
			output2.to_vec(),
		);

		// Constructing proof data
		let proof_data =
			ProofData::new(proof, public_amount, root_set, nullifiers, commitments, ext_data_hash);

		let relayer_balance_before = Balances::free_balance(relayer.clone());
		let recipient_balance_before = Balances::free_balance(recipient.clone());
		assert_ok!(VAnchor1::transact(
			RuntimeOrigin::signed(transactor),
			tree_id,
			proof_data,
			ext_data
		));

		// Should be equal to the `fee` since the transaction was sucessful
		let relayer_balance_after = Balances::free_balance(relayer);
		assert_eq!(relayer_balance_after, relayer_balance_before + fee);

		// Should be equal to the amount that is withdrawn
		let recipient_balance_after = Balances::free_balance(recipient);
		assert_eq!(recipient_balance_after, recipient_balance_before + ext_amount.unsigned_abs());
	});
}

#[test]
fn should_complete_2x2_transaction_with_withdraw_unwrap_and_refund_native_token() {
	new_test_ext().execute_with(|| {
		let (proving_key_2x2_bytes, verifying_key_2x2_bytes, _, _) = setup_environment();
		// Register a new wrapped asset / pool share over native assets
		assert_ok!(AssetRegistry::register(
			RuntimeOrigin::root(),
			b"webbWEBB".to_vec(),
			AssetType::PoolShare(vec![0]),
			0
		));
		let asset_id = AssetRegistry::next_asset_id() - 1;
		// Mint some wrapped asset / pool share by depositing the native asset
		let alice = get_account(TRANSACTOR_ACCOUNT_ID);
		assert_ok!(TokenWrapper::wrap(
			RuntimeOrigin::signed(alice.clone()),
			NativeCurrencyId::get(),
			asset_id,
			1_000,
			alice.clone(),
		));
		assert_eq!(Currencies::free_balance(asset_id, &alice), 1_000);

		/**** Create deposits with the newly wrapped token *** */
		let tree_id = create_vanchor(asset_id);
		let recipient = get_account(RECIPIENT_ACCOUNT_ID);
		let relayer: AccountId = get_account(RELAYER_ACCOUNT_ID);
		let ext_amount: Amount = 10_i128;
		let fee: Balance = 0;
		let refund: Balance = 0;
		let public_amount = ext_amount - (fee as i128);
		// Format other metdata: chain identifiers, input/output metadata
		let chain_type = [2, 0];
		let chain_id = compute_chain_id_type(ChainIdentifier::get(), chain_type);
		let in_chain_ids = [chain_id; 2];
		let in_amounts = [0, 0];
		let in_indices = [0, 1];
		let out_chain_ids = [chain_id; 2];
		let out_amounts = [10, 0];

		let in_utxos = setup_utxos(in_chain_ids, in_amounts, Some(in_indices));
		// We are adding indicies to out utxos, since they will be used as an input utxos in next
		// transaction
		let out_utxos = setup_utxos(out_chain_ids, out_amounts, Some(in_indices));

		let output1 = out_utxos[0].commitment.into_repr().to_bytes_be();
		let output2 = out_utxos[1].commitment.into_repr().to_bytes_be();
		let ext_data = ExtData::<AccountId, Amount, Balance, AssetId>::new(
			recipient.clone(),
			relayer.clone(),
			ext_amount,
			fee,
			refund,
			0,
			output1.to_vec(), // Mock encryption value, not meant to be used in production
			output2.to_vec(), // Mock encryption value, not meant to be used in production
		);

		let ext_data_hash = keccak_256(&ext_data.encode_abi());

		let custom_root = MerkleTree1::get_default_root(tree_id).unwrap();
		let neighbor_roots: [Element; EDGE_CT] = <LinkableTree1 as LinkableTreeInspector<
			LinkableTreeConfigration<Test, Instance1>,
		>>::get_neighbor_roots(tree_id)
		.unwrap()
		.try_into()
		.unwrap();

		let (proof, public_inputs) = setup_zk_circuit(
			public_amount,
			chain_id,
			ext_data_hash.to_vec(),
			in_utxos,
			out_utxos.clone(),
			proving_key_2x2_bytes.clone(),
			neighbor_roots,
			custom_root,
		);

		use arkworks_setups::common::verify;
		match verify::<ark_bn254::Bn254>(&public_inputs, &verifying_key_2x2_bytes, &proof) {
			Ok(res) => println!("Proof verification result: {}", res),
			Err(e) => panic!("Proof verification failed: {:?}", e),
		}

		// Deconstructing public inputs
		let (_chain_id, public_amount, root_set, nullifiers, commitments, ext_data_hash) =
			deconstruct_public_inputs_el(&public_inputs);

		// Constructing proof data
		let proof_data =
			ProofData::new(proof, public_amount, root_set, nullifiers, commitments, ext_data_hash);

		assert_ok!(VAnchor1::transact(RuntimeOrigin::signed(alice), tree_id, proof_data, ext_data));

		/**** Withdraw and unwrap **** */
		let custom_root = MerkleTree1::get_root(tree_id).unwrap();
		let ext_amount: Amount = -5;
		let fee: Balance = 2;
		let refund: Balance = 10;
		let public_amount = ext_amount - (fee as i128);
		let chain_type = [2, 0];
		let chain_id = compute_chain_id_type(ChainIdentifier::get(), chain_type);
		let out_chain_ids = [chain_id; 2];
		// After withdrawing -7
		let out_amounts = [1, 2];

		let in_utxos = out_utxos;
		let out_utxos = setup_utxos(out_chain_ids, out_amounts, None);

		let output1 = out_utxos[0].commitment.into_repr().to_bytes_be();
		let output2 = out_utxos[1].commitment.into_repr().to_bytes_be();
		let ext_data = ExtData::<AccountId, Amount, Balance, AssetId>::new(
			recipient.clone(),
			relayer.clone(),
			ext_amount,
			fee,
			refund,
			NativeCurrencyId::get(),
			output1.to_vec(),
			output2.to_vec(),
		);

		let ext_data_hash = keccak_256(&ext_data.encode_abi());

		let neighbor_roots = <LinkableTree1 as LinkableTreeInspector<
			LinkableTreeConfigration<Test, Instance1>,
		>>::get_neighbor_roots(tree_id)
		.unwrap()
		.try_into()
		.unwrap();

		let (proof, public_inputs) = setup_zk_circuit(
			public_amount,
			chain_id,
			ext_data_hash.to_vec(),
			in_utxos,
			out_utxos,
			proving_key_2x2_bytes,
			neighbor_roots,
			custom_root,
		);

		// Deconstructing public inputs
		let (_chain_id, public_amount, root_set, nullifiers, commitments, ext_data_hash) =
			deconstruct_public_inputs_el(&public_inputs);

		// Constructing proof data
		let proof_data =
			ProofData::new(proof, public_amount, root_set, nullifiers, commitments, ext_data_hash);

		let recipient_balance_before = Balances::free_balance(recipient.clone());
		let relayer_balance_before = Balances::free_balance(relayer.clone());
		let relayer_balance_wrapped_token_before = Currencies::free_balance(asset_id, &relayer);
		assert_ok!(VAnchor1::transact(
			RuntimeOrigin::signed(get_account(RELAYER_ACCOUNT_ID)),
			tree_id,
			proof_data,
			ext_data
		));

		// Should be equal to the `fee` since the transaction was sucessful
		let relayer_balance_after = Balances::free_balance(relayer.clone());
		let relayer_balance_wrapped_token_after = Currencies::free_balance(asset_id, &relayer);
		let recipient_balance_after = Balances::free_balance(recipient);

		// The relayer is paid a fee in the wrapped/pooled token. Therefore,
		// we expect the relayer's wrapped token balance to be PLUS the fee.
		assert_eq!(relayer_balance_wrapped_token_after, relayer_balance_wrapped_token_before + fee);
		// The relayer pays a refund to the recipient on successful transaction. Therefore,
		// we expect the relayer's native balance to be MINUS the refund.
		assert_eq!(relayer_balance_after, relayer_balance_before - refund);

		// For this test we are unwrapping into the native currency, therefore the
		// total balance of the recipient is the refund + the ext_amount.unsigned_abs()
		assert_eq!(
			recipient_balance_after,
			recipient_balance_before + refund + ext_amount.unsigned_abs()
		);
	});
}

#[test]
fn should_complete_2x2_transaction_with_withdraw_unwrap_and_refund_non_native_tokens() {
	new_test_ext().execute_with(|| {
		let (proving_key_2x2_bytes, _, _, _) = setup_environment();
		let alice = get_account(TRANSACTOR_ACCOUNT_ID);
		// Register a new asset that will be wrapped
		assert_ok!(AssetRegistry::register(
			RuntimeOrigin::root(),
			b"temp".to_vec(),
			AssetType::Token,
			0
		));
		let first_asset_id = AssetRegistry::next_asset_id() - 1;
		assert_ok!(Tokens::set_balance(RuntimeOrigin::root(), alice, first_asset_id, 10_000, 0));
		assert_ok!(AssetRegistry::register(
			RuntimeOrigin::root(),
			b"webbTemp".to_vec(),
			AssetType::PoolShare(vec![first_asset_id]),
			0
		));
		let pooled_asset_id = AssetRegistry::next_asset_id() - 1;
		// Mint some wrapped asset / pool share by depositing the native asset
		let alice = get_account(TRANSACTOR_ACCOUNT_ID);
		// assert_ok!(TokenWrapper::wrap(
		// 	RuntimeOrigin::signed(alice.clone()),
		// 	first_asset_id,
		// 	pooled_asset_id,
		// 	1_000,
		// 	alice.clone(),
		// ));

		/**** Create deposits with the newly wrapped token *** */
		let tree_id = create_vanchor(pooled_asset_id);
		let recipient = get_account(RECIPIENT_ACCOUNT_ID);
		let relayer: AccountId = get_account(RELAYER_ACCOUNT_ID);
		let ext_amount: Amount = 1_000_i128;
		let fee: Balance = 0;
		let refund: Balance = 0;
		let public_amount = ext_amount - (fee as i128);
		// Format other metdata: chain identifiers, input/output metadata
		let chain_type = [2, 0];
		let chain_id = compute_chain_id_type(ChainIdentifier::get(), chain_type);
		let in_chain_ids = [chain_id; 2];
		let in_amounts = [0, 0];
		let in_indices = [0, 1];
		let out_chain_ids = [chain_id; 2];
		let out_amounts = [10, 0];

		let in_utxos = setup_utxos(in_chain_ids, in_amounts, Some(in_indices));
		// We are adding indicies to out utxos, since they will be used as an input utxos in next
		// transaction
		let out_utxos = setup_utxos(out_chain_ids, out_amounts, Some(in_indices));

		let output1 = out_utxos[0].commitment.into_repr().to_bytes_be();
		let output2 = out_utxos[1].commitment.into_repr().to_bytes_be();
		let ext_data = ExtData::<AccountId, Amount, Balance, AssetId>::new(
			recipient.clone(),
			relayer.clone(),
			ext_amount,
			fee,
			refund,
			first_asset_id,
			// Mock encryption value, not meant to be used in production
			output1.to_vec(),
			// Mock encryption value, not meant to be used in production
			output2.to_vec(),
		);

		let ext_data_hash = keccak_256(&ext_data.encode_abi());

		let custom_root = MerkleTree1::get_default_root(tree_id).unwrap();
		let neighbor_roots: [Element; EDGE_CT] = <LinkableTree1 as LinkableTreeInspector<
			LinkableTreeConfigration<Test, Instance1>,
		>>::get_neighbor_roots(tree_id)
		.unwrap()
		.try_into()
		.unwrap();

		let (proof, public_inputs) = setup_zk_circuit(
			public_amount,
			chain_id,
			ext_data_hash.to_vec(),
			in_utxos,
			out_utxos.clone(),
			proving_key_2x2_bytes.clone(),
			neighbor_roots,
			custom_root,
		);

		// Deconstructing public inputs
		let (_chain_id, public_amount, root_set, nullifiers, commitments, ext_data_hash) =
			deconstruct_public_inputs_el(&public_inputs);

		// Constructing proof data
		let proof_data =
			ProofData::new(proof, public_amount, root_set, nullifiers, commitments, ext_data_hash);

		assert_ok!(VAnchor1::transact(RuntimeOrigin::signed(alice), tree_id, proof_data, ext_data));

		/**** Withdraw and unwrap **** */
		let custom_root = MerkleTree1::get_root(tree_id).unwrap();
		let ext_amount: Amount = -5;
		let fee: Balance = 2;
		let refund: Balance = 10;
		let public_amount = ext_amount - (fee as i128);
		let chain_type = [2, 0];
		let chain_id = compute_chain_id_type(ChainIdentifier::get(), chain_type);
		let out_chain_ids = [chain_id; 2];
		// After withdrawing -7
		let out_amounts = [1, 2];

		let in_utxos = out_utxos;
		let out_utxos = setup_utxos(out_chain_ids, out_amounts, None);

		let output1 = out_utxos[0].commitment.into_repr().to_bytes_be();
		let output2 = out_utxos[1].commitment.into_repr().to_bytes_be();
		let ext_data = ExtData::<AccountId, Amount, Balance, AssetId>::new(
			recipient.clone(),
			relayer.clone(),
			ext_amount,
			fee,
			refund,
			first_asset_id,
			output1.to_vec(),
			output2.to_vec(),
		);

		let ext_data_hash = keccak_256(&ext_data.encode_abi());

		let neighbor_roots = <LinkableTree1 as LinkableTreeInspector<
			LinkableTreeConfigration<Test, Instance1>,
		>>::get_neighbor_roots(tree_id)
		.unwrap()
		.try_into()
		.unwrap();

		let (proof, public_inputs) = setup_zk_circuit(
			public_amount,
			chain_id,
			ext_data_hash.to_vec(),
			in_utxos,
			out_utxos,
			proving_key_2x2_bytes,
			neighbor_roots,
			custom_root,
		);

		// Deconstructing public inputs
		let (_chain_id, public_amount, root_set, nullifiers, commitments, ext_data_hash) =
			deconstruct_public_inputs_el(&public_inputs);

		// Constructing external data
		let output1 = commitments[0];
		let output2 = commitments[1];
		let ext_data = ExtData::<AccountId, Amount, Balance, AssetId>::new(
			recipient.clone(),
			relayer.clone(),
			ext_amount,
			fee,
			refund,
			first_asset_id,
			output1.to_vec(),
			output2.to_vec(),
		);

		// Constructing proof data
		let proof_data =
			ProofData::new(proof, public_amount, root_set, nullifiers, commitments, ext_data_hash);

		// Fetching balances of things before transaction occurs
		let recipient_balance_before = Balances::free_balance(&recipient);
		let relayer_balance_before = Balances::free_balance(&relayer);
		let relayer_balance_wrapped_token_before =
			Currencies::free_balance(pooled_asset_id, &relayer);
		let recipient_balance_wrapped_token_before =
			Currencies::free_balance(pooled_asset_id, &recipient);
		let recipient_balance_unwrapped_token_before =
			Currencies::free_balance(first_asset_id, &recipient);
		assert_ok!(VAnchor1::transact(
			RuntimeOrigin::signed(get_account(RELAYER_ACCOUNT_ID)),
			tree_id,
			proof_data,
			ext_data
		));

		// Native balances of relayer / recipient. Since refund is in native tokens
		// we expect that the relayer's balance after is MINUS the refund amount.
		// We expect that the recipient's balance after is PLUS the refund amount.
		let relayer_balance_after = Balances::free_balance(&relayer);
		let recipient_balance_after = Balances::free_balance(&recipient);
		assert_eq!(relayer_balance_after, relayer_balance_before - refund);
		assert_eq!(recipient_balance_after, recipient_balance_before + refund);

		// Pooled token balances of relayer / recipient. Since the relayer receives a fee,
		// we expect that the relayer's balance after is PLUS the fee amount in the pooled token.
		// We expect that the recipient's balance after is PLUS the external amount in the pooled
		// token. Together the EXT_AMOUNT + FEE = PUBLIC_AMOUNT.
		let relayer_balance_wrapped_token_after =
			Currencies::free_balance(pooled_asset_id, &relayer);
		let recipient_balance_wrapped_token_after =
			Currencies::free_balance(pooled_asset_id, &recipient);
		let recipient_balance_unwrapped_token_after =
			Currencies::free_balance(first_asset_id, &recipient);
		assert_eq!(relayer_balance_wrapped_token_after, relayer_balance_wrapped_token_before + fee);
		assert_eq!(recipient_balance_wrapped_token_after, recipient_balance_wrapped_token_before);
		assert_eq!(
			recipient_balance_unwrapped_token_after,
			recipient_balance_unwrapped_token_before + ext_amount.unsigned_abs()
		);
	});
}

#[test]
fn should_complete_register_and_transact() {
	new_test_ext().execute_with(|| {
		let (proving_key_2x2_bytes, _, _, _) = setup_environment();
		let (tree_id, in_utxos) = create_vanchor_with_deposits(proving_key_2x2_bytes.clone(), None);
		let custom_root = MerkleTree1::get_root(tree_id).unwrap();

		let transactor: AccountId = get_account(TRANSACTOR_ACCOUNT_ID);
		let recipient: AccountId = get_account(RECIPIENT_ACCOUNT_ID);
		let relayer: AccountId = get_account(RELAYER_ACCOUNT_ID);
		let ext_amount: Amount = -5;
		let fee: Balance = 2;

		let public_amount = -7;

		let chain_type = [2, 0];
		let chain_id = compute_chain_id_type(ChainIdentifier::get(), chain_type);
		let out_chain_ids = [chain_id; 2];
		// After withdrawing -7
		let out_amounts = [1, 2];

		let out_utxos = setup_utxos(out_chain_ids, out_amounts, None);

		let output1 = out_utxos[0].commitment.into_repr().to_bytes_be();
		let output2 = out_utxos[1].commitment.into_repr().to_bytes_be();
		let ext_data = ExtData::<AccountId, Amount, Balance, AssetId>::new(
			recipient.clone(),
			relayer.clone(),
			ext_amount,
			fee,
			0,
			0,
			output1.to_vec(),
			output2.to_vec(),
		);

		let ext_data_hash = keccak_256(&ext_data.encode_abi());

		let neighbor_roots = <LinkableTree1 as LinkableTreeInspector<
			LinkableTreeConfigration<Test, Instance1>,
		>>::get_neighbor_roots(tree_id)
		.unwrap()
		.try_into()
		.unwrap();

		let (proof, public_inputs) = setup_zk_circuit(
			public_amount,
			chain_id,
			ext_data_hash.to_vec(),
			in_utxos,
			out_utxos,
			proving_key_2x2_bytes,
			neighbor_roots,
			custom_root,
		);

		// Deconstructing public inputs
		let (_chain_id, public_amount, root_set, nullifiers, commitments, ext_data_hash) =
			deconstruct_public_inputs_el(&public_inputs);

		// Constructing external data
		let output1 = commitments[0];
		let output2 = commitments[1];
		let ext_data = ExtData::<AccountId, Amount, Balance, AssetId>::new(
			recipient.clone(),
			relayer.clone(),
			ext_amount,
			fee,
			0,
			0,
			output1.to_vec(),
			output2.to_vec(),
		);

		// Constructing proof data
		let proof_data =
			ProofData::new(proof, public_amount, root_set, nullifiers, commitments, ext_data_hash);

		let relayer_balance_before = Balances::free_balance(relayer.clone());
		let recipient_balance_before = Balances::free_balance(recipient.clone());
		assert_ok!(VAnchor1::register_and_transact(
			RuntimeOrigin::signed(transactor.clone()),
			transactor,
			[0u8; 32].to_vec(),
			tree_id,
			proof_data,
			ext_data
		));

		// Should be equal to the `fee` since the transaction was sucessful
		let relayer_balance_after = Balances::free_balance(relayer);
		assert_eq!(relayer_balance_after, relayer_balance_before + fee);

		// Should be equal to the amount that is withdrawn
		let recipient_balance_after = Balances::free_balance(recipient);
		assert_eq!(recipient_balance_after, recipient_balance_before + ext_amount.unsigned_abs());
	});
}

#[test]
fn should_not_complete_transaction_if_ext_data_is_invalid() {
	new_test_ext().execute_with(|| {
		let (proving_key_2x2_bytes, _, _, _) = setup_environment();
		let tree_id = create_vanchor(0);

		let transactor = get_account(TRANSACTOR_ACCOUNT_ID);
		let recipient: AccountId = get_account(RECIPIENT_ACCOUNT_ID);
		let relayer: AccountId = get_account(RELAYER_ACCOUNT_ID);

		let ext_amount: Amount = 10_i128;
		let public_amount = 10_i128;
		let fee: Balance = 0;

		let chain_type = [2, 0];
		let chain_id = compute_chain_id_type(ChainIdentifier::get(), chain_type);
		let in_chain_ids = [chain_id; 2];
		let in_amounts = [0, 0];
		let in_indices = [0, 1];
		let out_chain_ids = [chain_id; 2];
		let out_amounts = [10, 0];

		let in_utxos = setup_utxos(in_chain_ids, in_amounts, Some(in_indices));
		let out_utxos = setup_utxos(out_chain_ids, out_amounts, None);

		let output1 = out_utxos[0].commitment.into_repr().to_bytes_be();
		let output2 = out_utxos[1].commitment.into_repr().to_bytes_be();
		let ext_data = ExtData::<AccountId, Amount, Balance, AssetId>::new(
			recipient.clone(),
			relayer.clone(),
			ext_amount,
			fee,
			0,
			0,
			// Mock encryption value, not meant to be used in production
			output1.to_vec(),
			// Mock encryption value, not meant to be used in production
			output2.to_vec(),
		);

		let ext_data_hash = keccak_256(&ext_data.encode_abi());

		let custom_root = MerkleTree1::get_default_root(tree_id).unwrap();
		let neighbor_roots: [Element; EDGE_CT] = <LinkableTree1 as LinkableTreeInspector<
			LinkableTreeConfigration<Test, Instance1>,
		>>::get_neighbor_roots(tree_id)
		.unwrap()
		.try_into()
		.unwrap();

		let (proof, public_inputs) = setup_zk_circuit(
			public_amount,
			chain_id,
			ext_data_hash.to_vec(),
			in_utxos,
			out_utxos,
			proving_key_2x2_bytes,
			neighbor_roots,
			custom_root,
		);

		// Deconstructing public inputs
		let (_chain_id, public_amount, root_set, nullifiers, commitments, ext_data_hash) =
			deconstruct_public_inputs_el(&public_inputs);

		// Constructing external data
		let output1 = commitments[0];

		// INVALID output commitment
		let output2 = Element::from_bytes(&[0u8; 32]);
		let ext_data = ExtData::<AccountId, Amount, Balance, AssetId>::new(
			recipient.clone(),
			relayer.clone(),
			ext_amount,
			fee,
			0,
			0,
			output1.to_vec(),
			output2.to_vec(),
		);

		// Constructing proof data
		let proof_data =
			ProofData::new(proof, public_amount, root_set, nullifiers, commitments, ext_data_hash);

		let relayer_balance_before = Balances::free_balance(relayer.clone());
		let transactor_balance_before = Balances::free_balance(transactor.clone());
		let recipient_balance_before = Balances::free_balance(recipient.clone());
		assert_err!(
			VAnchor1::transact(
				RuntimeOrigin::signed(transactor.clone()),
				tree_id,
				proof_data,
				ext_data
			),
			Error::<Test, Instance1>::InvalidExtData,
		);

		// Relayer balance should be zero since the fee was zero and the transaction
		// failed
		let relayer_balance_after = Balances::free_balance(relayer);
		assert_eq!(relayer_balance_after, relayer_balance_before);

		// Transactor balance should be the default one, since the deposit failed
		let transactor_balance_after = Balances::free_balance(transactor);
		assert_eq!(transactor_balance_after, transactor_balance_before);

		// Recipient balance should be zero since the withdraw was not successful
		let recipient_balance_after = Balances::free_balance(recipient);
		assert_eq!(recipient_balance_after, recipient_balance_before);
	});
}

#[test]
#[cfg(not(tarpaulin))]
fn should_not_complete_withdraw_if_out_amount_sum_is_too_big() {
	new_test_ext().execute_with(|| {
		let (proving_key_2x2_bytes, _, _, _) = setup_environment();
		let (tree_id, in_utxos) = create_vanchor_with_deposits(proving_key_2x2_bytes.clone(), None);
		let custom_root = MerkleTree1::get_root(tree_id).unwrap();

		let transactor = get_account(TRANSACTOR_ACCOUNT_ID);
		let recipient: AccountId = get_account(RECIPIENT_ACCOUNT_ID);
		let relayer: AccountId = get_account(RELAYER_ACCOUNT_ID);

		let public_amount = -7;
		let ext_amount: Amount = -5;
		let fee: Balance = 2;

		let chain_type = [2, 0];
		let chain_id = compute_chain_id_type(ChainIdentifier::get(), chain_type);
		let out_chain_ids = [chain_id; 2];
		// Withdraw amount too big
		let out_amounts = [100, 200];

		let out_utxos = setup_utxos(out_chain_ids, out_amounts, None);

		let output1 = out_utxos[0].commitment.into_repr().to_bytes_be();
		let output2 = out_utxos[1].commitment.into_repr().to_bytes_be();
		let ext_data = ExtData::<AccountId, Amount, Balance, AssetId>::new(
			recipient.clone(),
			relayer.clone(),
			ext_amount,
			fee,
			0,
			0,
			// Mock encryption value, not meant to be used in production
			output1.to_vec(),
			// Mock encryption value, not meant to be used in production
			output2.to_vec(),
		);

		let ext_data_hash = keccak_256(&ext_data.encode_abi());

		let neighbor_roots = <LinkableTree1 as LinkableTreeInspector<
			LinkableTreeConfigration<Test, Instance1>,
		>>::get_neighbor_roots(tree_id)
		.unwrap()
		.try_into()
		.unwrap();

		let (proof, public_inputs) = setup_zk_circuit(
			public_amount,
			chain_id,
			ext_data_hash.to_vec(),
			in_utxos,
			out_utxos,
			proving_key_2x2_bytes,
			neighbor_roots,
			custom_root,
		);

		// Deconstructing public inputs
		let (_chain_id, public_amount, root_set, nullifiers, commitments, ext_data_hash) =
			deconstruct_public_inputs_el(&public_inputs);

		// Constructing external data
		let output1 = commitments[0];
		let output2 = commitments[1];
		let ext_data = ExtData::<AccountId, Amount, Balance, AssetId>::new(
			recipient.clone(),
			relayer.clone(),
			ext_amount,
			fee,
			0,
			0,
			output1.to_vec(),
			output2.to_vec(),
		);

		// Constructing proof data
		let proof_data =
			ProofData::new(proof, public_amount, root_set, nullifiers, commitments, ext_data_hash);

		let relayer_balance_before = Balances::free_balance(relayer.clone());
		let transactor_balance_before = Balances::free_balance(transactor.clone());
		let recipient_balance_before = Balances::free_balance(recipient.clone());
		// Should fail with invalid external data error
		assert_err!(
			VAnchor1::transact(
				RuntimeOrigin::signed(transactor.clone()),
				tree_id,
				proof_data,
				ext_data
			),
			Error::<Test, Instance1>::InvalidTransactionProof
		);

		// Should be zero, since transaction failed
		let relayer_balance_after = Balances::free_balance(relayer);
		assert_eq!(relayer_balance_after, relayer_balance_before);

		// Transactors balance is zero since they deposited all of their money to the
		// mixer
		let transactor_balance_after = Balances::free_balance(transactor);
		assert_eq!(transactor_balance_after, transactor_balance_before);

		// Recipient balance is zero, since the withdraw failed
		let recipient_balance_after = Balances::free_balance(recipient);
		assert_eq!(recipient_balance_after, recipient_balance_before);
	});
}

#[test]
#[cfg(not(tarpaulin))]
fn should_not_complete_withdraw_if_out_amount_sum_is_too_small() {
	new_test_ext().execute_with(|| {
		let (proving_key_2x2_bytes, _, _, _) = setup_environment();
		let (tree_id, in_utxos) = create_vanchor_with_deposits(proving_key_2x2_bytes.clone(), None);
		let custom_root = MerkleTree1::get_root(tree_id).unwrap();

		let transactor = get_account(TRANSACTOR_ACCOUNT_ID);
		let recipient: AccountId = get_account(RECIPIENT_ACCOUNT_ID);
		let relayer: AccountId = get_account(RELAYER_ACCOUNT_ID);

		let ext_amount: Amount = -5;
		let fee: Balance = 2;

		let public_amount = -7;

		let chain_type = [2, 0];
		let chain_id = compute_chain_id_type(ChainIdentifier::get(), chain_type);
		let out_chain_ids = [chain_id; 2];
		// Withdraw amount too small
		let out_amounts = [1, 0];

		let out_utxos = setup_utxos(out_chain_ids, out_amounts, None);

		let output1 = out_utxos[0].commitment.into_repr().to_bytes_be();
		let output2 = out_utxos[1].commitment.into_repr().to_bytes_be();
		let ext_data = ExtData::<AccountId, Amount, Balance, AssetId>::new(
			recipient.clone(),
			relayer.clone(),
			ext_amount,
			fee,
			0,
			0,
			// Mock encryption value, not meant to be used in production
			output1.to_vec(),
			// Mock encryption value, not meant to be used in production
			output2.to_vec(),
		);

		let ext_data_hash = keccak_256(&ext_data.encode_abi());

		let neighbor_roots = <LinkableTree1 as LinkableTreeInspector<
			LinkableTreeConfigration<Test, Instance1>,
		>>::get_neighbor_roots(tree_id)
		.unwrap()
		.try_into()
		.unwrap();

		let (proof, public_inputs) = setup_zk_circuit(
			public_amount,
			chain_id,
			ext_data_hash.to_vec(),
			in_utxos,
			out_utxos,
			proving_key_2x2_bytes,
			neighbor_roots,
			custom_root,
		);

		// Deconstructing public inputs
		let (_chain_id, public_amount, root_set, nullifiers, commitments, ext_data_hash) =
			deconstruct_public_inputs_el(&public_inputs);

		// Constructing external data
		let output1 = commitments[0];
		let output2 = commitments[1];
		let ext_data = ExtData::<AccountId, Amount, Balance, AssetId>::new(
			recipient.clone(),
			relayer.clone(),
			ext_amount,
			fee,
			0,
			0,
			output1.to_vec(),
			output2.to_vec(),
		);

		// Constructing proof data
		let proof_data =
			ProofData::new(proof, public_amount, root_set, nullifiers, commitments, ext_data_hash);

		let relayer_balance_before = Balances::free_balance(relayer.clone());
		let transactor_balance_before = Balances::free_balance(transactor.clone());
		let recipient_balance_before = Balances::free_balance(recipient.clone());
		// Should fail with invalid external data error
		assert_err!(
			VAnchor1::transact(
				RuntimeOrigin::signed(transactor.clone()),
				tree_id,
				proof_data,
				ext_data
			),
			Error::<Test, Instance1>::InvalidTransactionProof
		);

		// Should be zero, since transaction failed
		let relayer_balance_after = Balances::free_balance(relayer);
		assert_eq!(relayer_balance_after, relayer_balance_before);

		// Transactors balance is zero since they deposited all of their money to the mixer
		let transactor_balance_after = Balances::free_balance(transactor);
		assert_eq!(transactor_balance_after, transactor_balance_before);

		// Recipient balance is zero, since the withdraw failed
		let recipient_balance_after = Balances::free_balance(recipient);
		assert_eq!(recipient_balance_after, recipient_balance_before);
	});
}

#[test]
fn should_not_be_able_to_double_spend() {
	new_test_ext().execute_with(|| {
		let (proving_key_2x2_bytes, _, _, _) = setup_environment();
		let (tree_id, in_utxos) = create_vanchor_with_deposits(proving_key_2x2_bytes.clone(), None);
		let custom_root = MerkleTree1::get_root(tree_id).unwrap();

		let transactor: AccountId = get_account(TRANSACTOR_ACCOUNT_ID);
		let recipient: AccountId = get_account(RECIPIENT_ACCOUNT_ID);
		let relayer: AccountId = get_account(RELAYER_ACCOUNT_ID);
		let ext_amount: Amount = -5;
		let fee: Balance = 2;

		let public_amount = -7;

		let chain_type = [2, 0];
		let chain_id = compute_chain_id_type(ChainIdentifier::get(), chain_type);
		let out_chain_ids = [chain_id; 2];
		// After withdrawing -7
		let out_amounts = [1, 2];

		let out_utxos = setup_utxos(out_chain_ids, out_amounts, None);

		let output1 = out_utxos[0].commitment.into_repr().to_bytes_be();
		let output2 = out_utxos[1].commitment.into_repr().to_bytes_be();
		let ext_data = ExtData::<AccountId, Amount, Balance, AssetId>::new(
			recipient.clone(),
			relayer.clone(),
			ext_amount,
			fee,
			0,
			0,
			// Mock encryption value, not meant to be used in production
			output1.to_vec(),
			// Mock encryption value, not meant to be used in production
			output2.to_vec(),
		);

		let ext_data_hash = keccak_256(&ext_data.encode_abi());

		let neighbor_roots = <LinkableTree1 as LinkableTreeInspector<
			LinkableTreeConfigration<Test, Instance1>,
		>>::get_neighbor_roots(tree_id)
		.unwrap()
		.try_into()
		.unwrap();

		let (proof, public_inputs) = setup_zk_circuit(
			public_amount,
			chain_id,
			ext_data_hash.to_vec(),
			in_utxos,
			out_utxos,
			proving_key_2x2_bytes,
			neighbor_roots,
			custom_root,
		);

		// Deconstructing public inputs
		let (_chain_id, public_amount, root_set, nullifiers, commitments, ext_data_hash) =
			deconstruct_public_inputs_el(&public_inputs);

		// Constructing external data
		let output1 = commitments[0];
		let output2 = commitments[1];
		let ext_data = ExtData::<AccountId, Amount, Balance, AssetId>::new(
			recipient.clone(),
			relayer.clone(),
			ext_amount,
			fee,
			0,
			0,
			output1.to_vec(),
			output2.to_vec(),
		);

		// Constructing proof data
		let proof_data =
			ProofData::new(proof, public_amount, root_set, nullifiers, commitments, ext_data_hash);

		let relayer_balance_before = Balances::free_balance(relayer.clone());
		let transactor_balance_before = Balances::free_balance(transactor.clone());
		let recipient_balance_before = Balances::free_balance(recipient.clone());
		assert_ok!(VAnchor1::transact(
			RuntimeOrigin::signed(transactor.clone()),
			tree_id,
			proof_data.clone(),
			ext_data.clone()
		));
		assert_err!(
			VAnchor1::transact(
				RuntimeOrigin::signed(transactor.clone()),
				tree_id,
				proof_data,
				ext_data
			),
			Error::<Test, Instance1>::AlreadyRevealedNullifier
		);

		// Fee is paid out once
		let relayer_balance_after = Balances::free_balance(relayer);
		assert_eq!(relayer_balance_after, relayer_balance_before + fee);

		// Recipient is paid out once
		let recipient_balance_after = Balances::free_balance(recipient);
		assert_eq!(recipient_balance_after, recipient_balance_before + ext_amount.unsigned_abs());

		// Transactor is 0 after one deposit
		let transactor_balance_after = Balances::free_balance(transactor);
		assert_eq!(transactor_balance_after, transactor_balance_before);
	});
}

#[test]
fn should_not_be_able_to_exceed_max_fee() {
	new_test_ext().execute_with(|| {
		let (proving_key_2x2_bytes, _, _, _) = setup_environment();
		let tree_id = create_vanchor(0);

		let transactor = get_account(TRANSACTOR_ACCOUNT_ID);
		let recipient: AccountId = get_account(RECIPIENT_ACCOUNT_ID);
		let relayer: AccountId = get_account(RELAYER_ACCOUNT_ID);

		let ext_amount: Amount = 10_i128;
		let public_amount = 4;
		let fee: Balance = 6;

		let chain_type = [2, 0];
		let chain_id = compute_chain_id_type(ChainIdentifier::get(), chain_type);
		let in_chain_ids = [chain_id; 2];
		let in_amounts = [0, 0];
		let in_indices = [0, 1];
		let out_chain_ids = [chain_id; 2];
		let out_amounts = [4, 0];

		let in_utxos = setup_utxos(in_chain_ids, in_amounts, Some(in_indices));
		let out_utxos = setup_utxos(out_chain_ids, out_amounts, None);

		let output1 = out_utxos[0].commitment.into_repr().to_bytes_be();
		let output2 = out_utxos[1].commitment.into_repr().to_bytes_be();
		let ext_data = ExtData::<AccountId, Amount, Balance, AssetId>::new(
			recipient.clone(),
			relayer.clone(),
			ext_amount,
			fee,
			0,
			0,
			// Mock encryption value, not meant to be used in production
			output1.to_vec(),
			// Mock encryption value, not meant to be used in production
			output2.to_vec(),
		);

		let ext_data_hash = keccak_256(&ext_data.encode_abi());

		let custom_root = MerkleTree1::get_default_root(tree_id).unwrap();
		let neighbor_roots: [Element; EDGE_CT] = <LinkableTree1 as LinkableTreeInspector<
			LinkableTreeConfigration<Test, Instance1>,
		>>::get_neighbor_roots(tree_id)
		.unwrap()
		.try_into()
		.unwrap();

		let (proof, public_inputs) = setup_zk_circuit(
			public_amount,
			chain_id,
			ext_data_hash.to_vec(),
			in_utxos,
			out_utxos,
			proving_key_2x2_bytes,
			neighbor_roots,
			custom_root,
		);

		// Deconstructing public inputs
		let (_chain_id, public_amount, root_set, nullifiers, commitments, ext_data_hash) =
			deconstruct_public_inputs_el(&public_inputs);

		// Constructing external data
		let output1 = commitments[0];
		let output2 = commitments[1];
		let ext_data = ExtData::<AccountId, Amount, Balance, AssetId>::new(
			recipient,
			relayer.clone(),
			ext_amount,
			fee,
			0,
			0,
			output1.to_vec(),
			output2.to_vec(),
		);

		// Constructing proof data
		let proof_data =
			ProofData::new(proof, public_amount, root_set, nullifiers, commitments, ext_data_hash);

		let relayer_balance_before = Balances::free_balance(relayer.clone());
		let transactor_balance_before = Balances::free_balance(transactor.clone());
		assert_err!(
			VAnchor1::transact(
				RuntimeOrigin::signed(transactor.clone()),
				tree_id,
				proof_data,
				ext_data
			),
			Error::<Test, Instance1>::InvalidFee
		);

		// Relayer balance should be zero since the fee was zero
		let relayer_balance_after = Balances::free_balance(relayer);
		assert_eq!(relayer_balance_after, relayer_balance_before);

		// Transactor balance should not be changed, since the transaction has failed
		let transactor_balance_after = Balances::free_balance(transactor);
		assert_eq!(transactor_balance_after, transactor_balance_before);
	});
}

#[test]
fn should_not_be_able_to_exceed_max_deposit() {
	new_test_ext().execute_with(|| {
		let (proving_key_2x2_bytes, _, _, _) = setup_environment();
		let tree_id = create_vanchor(0);

		let transactor = get_account(BIG_TRANSACTOR_ACCOUNT_ID);
		let recipient: AccountId = get_account(RECIPIENT_ACCOUNT_ID);
		let relayer: AccountId = get_account(RELAYER_ACCOUNT_ID);

		let more_than_max_balance = 20;
		let ext_amount: Amount = more_than_max_balance as i128;
		let public_amount = ext_amount as i128;
		let fee: Balance = 0;

		let chain_type = [2, 0];
		let chain_id = compute_chain_id_type(ChainIdentifier::get(), chain_type);
		let in_chain_ids = [chain_id; 2];
		let in_amounts = [0, 0];
		let in_indices = [0, 1];
		let out_chain_ids = [chain_id; 2];
		let out_amounts = [more_than_max_balance, 0];

		let in_utxos = setup_utxos(in_chain_ids, in_amounts, Some(in_indices));
		let out_utxos = setup_utxos(out_chain_ids, out_amounts, None);

		let output1 = out_utxos[0].commitment.into_repr().to_bytes_be();
		let output2 = out_utxos[1].commitment.into_repr().to_bytes_be();
		let ext_data = ExtData::<AccountId, Amount, Balance, AssetId>::new(
			recipient.clone(),
			relayer.clone(),
			ext_amount,
			fee,
			0,
			0,
			// Mock encryption value, not meant to be used in production
			output1.to_vec(),
			// Mock encryption value, not meant to be used in production
			output2.to_vec(),
		);

		let ext_data_hash = keccak_256(&ext_data.encode_abi());

		let custom_root = MerkleTree1::get_default_root(tree_id).unwrap();
		let neighbor_roots: [Element; EDGE_CT] = <LinkableTree1 as LinkableTreeInspector<
			LinkableTreeConfigration<Test, Instance1>,
		>>::get_neighbor_roots(tree_id)
		.unwrap()
		.try_into()
		.unwrap();

		let (proof, public_inputs) = setup_zk_circuit(
			public_amount,
			chain_id,
			ext_data_hash.to_vec(),
			in_utxos,
			out_utxos,
			proving_key_2x2_bytes,
			neighbor_roots,
			custom_root,
		);

		// Deconstructing public inputs
		let (_chain_id, public_amount, root_set, nullifiers, commitments, ext_data_hash) =
			deconstruct_public_inputs_el(&public_inputs);

		// Constructing external data
		let output1 = commitments[0];
		let output2 = commitments[1];
		let ext_data = ExtData::<AccountId, Amount, Balance, AssetId>::new(
			recipient,
			relayer.clone(),
			ext_amount,
			fee,
			0,
			0,
			output1.to_vec(),
			output2.to_vec(),
		);

		// Constructing proof data
		let proof_data =
			ProofData::new(proof, public_amount, root_set, nullifiers, commitments, ext_data_hash);

		let relayer_balance_before = Balances::free_balance(relayer.clone());
		let transactor_balance_before = Balances::free_balance(transactor.clone());
		assert_err!(
			VAnchor1::transact(
				RuntimeOrigin::signed(transactor.clone()),
				tree_id,
				proof_data,
				ext_data
			),
			Error::<Test, Instance1>::InvalidDepositAmount
		);

		// Relayer balance should be zero since the fee was zero
		let relayer_balance_after = Balances::free_balance(relayer);
		assert_eq!(relayer_balance_after, relayer_balance_before);

		// Transactor balance should not be changed, since the transaction has failed
		let transactor_balance_after = Balances::free_balance(transactor);
		assert_eq!(transactor_balance_after, transactor_balance_before);
	});
}

#[test]
fn should_not_be_able_to_exceed_external_amount() {
	new_test_ext().execute_with(|| {
		let (proving_key_2x2_bytes, _, _, _) = setup_environment();
		let tree_id = create_vanchor(0);

		let transactor = get_account(BIGGER_TRANSACTOR_ACCOUNT_ID);
		let recipient: AccountId = get_account(RECIPIENT_ACCOUNT_ID);
		let relayer: AccountId = get_account(RELAYER_ACCOUNT_ID);

		// The external amount will be 3 more than allowed
		let ext_amount: Amount = 23;
		let public_amount = 20;
		let fee: Balance = 3;

		let chain_type = [2, 0];
		let chain_id = compute_chain_id_type(ChainIdentifier::get(), chain_type);
		let in_chain_ids = [chain_id; 2];
		let in_amounts = [0, 0];
		let in_indices = [0, 1];
		let out_chain_ids = [chain_id; 2];
		let out_amounts = [20, 0];

		let in_utxos = setup_utxos(in_chain_ids, in_amounts, Some(in_indices));
		let out_utxos = setup_utxos(out_chain_ids, out_amounts, None);

		let output1 = out_utxos[0].commitment.into_repr().to_bytes_be();
		let output2 = out_utxos[1].commitment.into_repr().to_bytes_be();
		let ext_data = ExtData::<AccountId, Amount, Balance, AssetId>::new(
			recipient.clone(),
			relayer.clone(),
			ext_amount,
			fee,
			0,
			0,
			// Mock encryption value, not meant to be used in production
			output1.to_vec(),
			// Mock encryption value, not meant to be used in production
			output2.to_vec(),
		);

		let ext_data_hash = keccak_256(&ext_data.encode_abi());

		let custom_root = MerkleTree1::get_default_root(tree_id).unwrap();
		let neighbor_roots: [Element; EDGE_CT] = <LinkableTree1 as LinkableTreeInspector<
			LinkableTreeConfigration<Test, Instance1>,
		>>::get_neighbor_roots(tree_id)
		.unwrap()
		.try_into()
		.unwrap();

		let (proof, public_inputs) = setup_zk_circuit(
			public_amount,
			chain_id,
			ext_data_hash.to_vec(),
			in_utxos,
			out_utxos,
			proving_key_2x2_bytes,
			neighbor_roots,
			custom_root,
		);

		// Deconstructing public inputs
		let (_chain_id, public_amount, root_set, nullifiers, commitments, ext_data_hash) =
			deconstruct_public_inputs_el(&public_inputs);

		// Constructing external data
		let output1 = commitments[0];
		let output2 = commitments[1];
		let ext_data = ExtData::<AccountId, Amount, Balance, AssetId>::new(
			recipient,
			relayer.clone(),
			ext_amount,
			fee,
			0,
			0,
			output1.to_vec(),
			output2.to_vec(),
		);

		// Constructing proof data
		let proof_data =
			ProofData::new(proof, public_amount, root_set, nullifiers, commitments, ext_data_hash);

		let relayer_balance_before = Balances::free_balance(relayer.clone());
		let transactor_balance_before = Balances::free_balance(transactor.clone());
		assert_err!(
			VAnchor1::transact(
				RuntimeOrigin::signed(transactor.clone()),
				tree_id,
				proof_data,
				ext_data
			),
			Error::<Test, Instance1>::InvalidExtAmount
		);

		// Relayer balance should be zero since the transaction failed
		let relayer_balance_after = Balances::free_balance(relayer);
		assert_eq!(relayer_balance_after, relayer_balance_before);

		// Transactor balance should not be changed, since the transaction has failed
		let transactor_balance_after = Balances::free_balance(transactor);
		assert_eq!(transactor_balance_after, transactor_balance_before);
	});
}

#[test]
fn should_not_be_able_to_withdraw_less_than_minimum() {
	new_test_ext().execute_with(|| {
		let (proving_key_2x2_bytes, _, _, _) = setup_environment();
		let (tree_id, in_utxos) = create_vanchor_with_deposits(proving_key_2x2_bytes.clone(), None);
		let custom_root = MerkleTree1::get_root(tree_id).unwrap();

		let transactor: AccountId = get_account(TRANSACTOR_ACCOUNT_ID);
		let recipient: AccountId = get_account(RECIPIENT_ACCOUNT_ID);
		let relayer: AccountId = get_account(RELAYER_ACCOUNT_ID);
		let ext_amount: Amount = -2;
		let fee: Balance = 4;

		let public_amount = -6;

		let chain_type = [2, 0];
		let chain_id = compute_chain_id_type(ChainIdentifier::get(), chain_type);
		let out_chain_ids = [chain_id; 2];
		// After withdrawing -7
		let out_amounts = [2, 2];

		let out_utxos = setup_utxos(out_chain_ids, out_amounts, None);

		let output1 = out_utxos[0].commitment.into_repr().to_bytes_be();
		let output2 = out_utxos[1].commitment.into_repr().to_bytes_be();
		let ext_data = ExtData::<AccountId, Amount, Balance, AssetId>::new(
			recipient.clone(),
			relayer.clone(),
			ext_amount,
			fee,
			0,
			0,
			output1.to_vec(), // Mock encryption value, not meant to be used in production
			output2.to_vec(), // Mock encryption value, not meant to be used in production
		);

		let ext_data_hash = keccak_256(&ext_data.encode_abi());

		let neighbor_roots = <LinkableTree1 as LinkableTreeInspector<
			LinkableTreeConfigration<Test, Instance1>,
		>>::get_neighbor_roots(tree_id)
		.unwrap()
		.try_into()
		.unwrap();

		let (proof, public_inputs) = setup_zk_circuit(
			public_amount,
			chain_id,
			ext_data_hash.to_vec(),
			in_utxos,
			out_utxos,
			proving_key_2x2_bytes,
			neighbor_roots,
			custom_root,
		);

		// Deconstructing public inputs
		let (_chain_id, public_amount, root_set, nullifiers, commitments, ext_data_hash) =
			deconstruct_public_inputs_el(&public_inputs);

		// Constructing external data
		let output1 = commitments[0];
		let output2 = commitments[1];
		let ext_data = ExtData::<AccountId, Amount, Balance, AssetId>::new(
			recipient.clone(),
			relayer.clone(),
			ext_amount,
			fee,
			0,
			0,
			output1.to_vec(),
			output2.to_vec(),
		);

		// Constructing proof data
		let proof_data =
			ProofData::new(proof, public_amount, root_set, nullifiers, commitments, ext_data_hash);

		let relayer_balance_before = Balances::free_balance(relayer.clone());
		let transactor_balance_before = Balances::free_balance(transactor.clone());
		let recipient_balance_before = Balances::free_balance(recipient.clone());
		assert_err!(
			VAnchor1::transact(
				RuntimeOrigin::signed(transactor.clone()),
				tree_id,
				proof_data,
				ext_data
			),
			Error::<Test, Instance1>::InvalidWithdrawAmount
		);

		// Fee is not paid out
		let relayer_balance_after = Balances::free_balance(relayer);
		assert_eq!(relayer_balance_after, relayer_balance_before);

		// Recipient is not paid
		let recipient_balance_after = Balances::free_balance(recipient);
		assert_eq!(recipient_balance_after, recipient_balance_before);

		let transactor_balance_after = Balances::free_balance(transactor);
		assert_eq!(transactor_balance_after, transactor_balance_before);
	});
}

#[test]
fn set_get_max_deposit_amount() {
	new_test_ext().execute_with(|| {
		assert_ok!(VAnchor1::set_max_deposit_amount(RuntimeOrigin::root(), 1, 1));
		assert_eq!(MaxDepositAmount::<Test, Instance1>::get(), 1);

		assert_ok!(VAnchor1::set_max_deposit_amount(RuntimeOrigin::root(), 5, 2));
		assert_eq!(MaxDepositAmount::<Test, Instance1>::get(), 5);
	})
}

#[test]
fn set_get_min_withdraw_amount() {
	new_test_ext().execute_with(|| {
		assert_ok!(VAnchor1::set_min_withdraw_amount(RuntimeOrigin::root(), 2, 1));
		assert_eq!(MinWithdrawAmount::<Test, Instance1>::get(), 2);

		assert_ok!(VAnchor1::set_min_withdraw_amount(RuntimeOrigin::root(), 5, 2));
		assert_eq!(MinWithdrawAmount::<Test, Instance1>::get(), 5);
	})
}

#[test]
fn should_fail_to_set_amounts_with_invalid_nonces() {
	new_test_ext().execute_with(|| {
		assert_ok!(VAnchor1::set_min_withdraw_amount(RuntimeOrigin::root(), 2, 1));
		assert_eq!(MinWithdrawAmount::<Test, Instance1>::get(), 2);

		assert_err!(
			VAnchor1::set_min_withdraw_amount(RuntimeOrigin::root(), 5, 1),
			Error::<Test, Instance1>::InvalidNonce
		);
	})
}
