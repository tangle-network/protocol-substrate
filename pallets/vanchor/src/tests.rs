use std::convert::TryInto;

use ark_ff::{BigInteger, PrimeField};
use arkworks_gadgets::setup::common::Curve;

use darkwebb_primitives::{merkle_tree::TreeInspector, AccountId, ElementTrait};

use codec::Encode;

use frame_benchmarking::account;
use frame_support::{assert_err, assert_ok, error::BadOrigin, traits::OnInitialize};
use pallet_asset_registry::AssetType;

use crate::{mock::*, test_utils::*};

const SEED: u32 = 0;
const TREE_DEPTH: usize = 30;
const M: usize = 2;

fn setup_environment(curve: Curve) -> Vec<u8> {
	let params = match curve {
		Curve::Bn254 => get_hash_params::<ark_bn254::Fr>(curve),
		Curve::Bls381 => {
			todo!("Setup hash params for bls381")
		}
	};
	// 1. Setup The Hasher Pallet.
	assert_ok!(HasherPallet::force_set_parameters(Origin::root(), params.0));
	// 2. Initialize MerkleTree pallet.
	<MerkleTree as OnInitialize<u64>>::on_initialize(1);
	// 3. Setup the VerifierPallet
	//    but to do so, we need to have a VerifyingKey
	let mut verifier_key_bytes = Vec::new();
	let mut proving_key_bytes = Vec::new();

	get_keys(curve, &mut proving_key_bytes, &mut verifier_key_bytes);

	assert_ok!(VerifierPallet::force_set_parameters(Origin::root(), verifier_key_bytes));
	// 4. and top-up some accounts with some balance
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
	proving_key_bytes
}

#[test]
fn should_create_new_vanchor() {
	new_test_ext().execute_with(|| {
		setup_environment(Curve::Bn254);
		let max_edges = M as _;
		let depth = TREE_DEPTH as u8;
		let asset_id = 0;

		assert_ok!(VAnchor::create(Origin::root(), max_edges, depth, asset_id));

		let tree_id = MerkleTree::next_tree_id() - 1;
		crate::mock::assert_last_event::<Test>(crate::Event::<Test>::VAnchorCreation { tree_id }.into());
	});
}

#[test]
fn should_fail_to_create_new_vanchor_if_not_root() {
	new_test_ext().execute_with(|| {
		setup_environment(Curve::Bn254);
		let max_edges = M as _;
		let depth = TREE_DEPTH as u8;
		let asset_id = 0;
		assert_err!(
			VAnchor::create(
				Origin::signed(account::<AccountId>("", 1, SEED)),
				max_edges,
				depth,
				asset_id
			),
			BadOrigin,
		);
	});
}

#[test]
fn should_be_able_to_deposit() {
	new_test_ext().execute_with(|| {
		setup_environment(Curve::Bn254);
	});
}

#[test]
fn should_fail_to_deposit_if_vanchor_not_found() {
	new_test_ext().execute_with(|| {
		setup_environment(Curve::Bn254);
	});
}

fn create_vanchor(asset_id: u32) -> u32 {
	let max_edges = 2;
	let depth = TREE_DEPTH as u8;
	assert_ok!(VAnchor::create(Origin::root(), max_edges, depth, asset_id));
	MerkleTree::next_tree_id() - 1
}

#[test]
fn vanchor_works() {
	new_test_ext().execute_with(|| {
		let curve = Curve::Bn254;
		let pk_bytes = setup_environment(curve);

		// inputs
		let tree_id = create_vanchor(0);
		let src_chain_id = 1;
		let sender_account_id = account::<AccountId>("", 1, SEED);
		let recipient_account_id = account::<AccountId>("", 2, SEED);
		let relayer_account_id = account::<AccountId>("", 0, SEED);
		let fee_value = 0;
		let refund_value = 0;

		let recipient_bytes = crate::truncate_and_pad(&recipient_account_id.encode()[..]);
		let relayer_bytes = crate::truncate_and_pad(&relayer_account_id.encode()[..]);

		let (proof_bytes, roots_element, nullifier_hash_element, leaf_element) = setup_zk_circuit(
			curve,
			recipient_bytes,
			relayer_bytes,
			pk_bytes,
			src_chain_id,
			fee_value,
			refund_value,
		);
	});
}

#[test]
fn double_spending_should_fail() {
	new_test_ext().execute_with(|| {
		let curve = Curve::Bn254;
		let pk_bytes = setup_environment(curve);

		// inputs
		let tree_id = create_vanchor(0);
		let src_chain_id = 1;
		let sender_account_id = account::<AccountId>("", 1, SEED);
		let recipient_account_id = account::<AccountId>("", 2, SEED);
		let relayer_account_id = account::<AccountId>("", 0, SEED);
		let fee_value = 0;
		let refund_value = 0;

		let recipient_bytes = crate::truncate_and_pad(&recipient_account_id.encode()[..]);
		let relayer_bytes = crate::truncate_and_pad(&relayer_account_id.encode()[..]);

		let (proof_bytes, roots_element, nullifier_hash_element, leaf_element) = setup_zk_circuit(
			curve,
			recipient_bytes,
			relayer_bytes,
			pk_bytes,
			src_chain_id,
			fee_value,
			refund_value,
		);
	});
}

#[test]
fn should_fail_when_invalid_merkle_roots() {
	new_test_ext().execute_with(|| {
		let curve = Curve::Bn254;
		let pk_bytes = setup_environment(curve);

		// inputs
		let tree_id = create_vanchor(0);
		let src_chain_id = 1;
		let sender_account_id = account::<AccountId>("", 1, SEED);
		let recipient_account_id = account::<AccountId>("", 2, SEED);
		let relayer_account_id = account::<AccountId>("", 0, SEED);
		let fee_value = 0;
		let refund_value = 0;

		let recipient_bytes = crate::truncate_and_pad(&recipient_account_id.encode()[..]);
		let relayer_bytes = crate::truncate_and_pad(&relayer_account_id.encode()[..]);

		let (proof_bytes, mut roots_element, nullifier_hash_element, leaf_element) = setup_zk_circuit(
			curve,
			recipient_bytes,
			relayer_bytes,
			pk_bytes,
			src_chain_id,
			fee_value,
			refund_value,
		);
	});
}

#[test]
fn should_fail_with_when_any_byte_is_changed_in_proof() {
	new_test_ext().execute_with(|| {
		let curve = Curve::Bn254;
		let pk_bytes = setup_environment(curve);

		// inputs
		let tree_id = create_vanchor(0);
		let src_chain_id = 1;
		let sender_account_id = account::<AccountId>("", 1, SEED);
		let recipient_account_id = account::<AccountId>("", 2, SEED);
		let relayer_account_id = account::<AccountId>("", 0, SEED);
		let fee_value = 0;
		let refund_value = 0;

		let recipient_bytes = crate::truncate_and_pad(&recipient_account_id.encode()[..]);
		let relayer_bytes = crate::truncate_and_pad(&relayer_account_id.encode()[..]);

		let (mut proof_bytes, roots_element, nullifier_hash_element, leaf_element) = setup_zk_circuit(
			curve,
			recipient_bytes,
			relayer_bytes,
			pk_bytes,
			src_chain_id,
			fee_value,
			refund_value,
		);
	});
}

#[test]
fn should_fail_when_relayer_id_is_different_from_that_in_proof_generation() {
	new_test_ext().execute_with(|| {
		let curve = Curve::Bn254;
		let pk_bytes = setup_environment(curve);

		// inputs
		let tree_id = create_vanchor(0);
		let src_chain_id = 1;
		let sender_account_id = account::<AccountId>("", 1, SEED);
		let recipient_account_id = account::<AccountId>("", 2, SEED);
		let relayer_account_id = account::<AccountId>("", 0, SEED);
		let fee_value = 0;
		let refund_value = 0;

		let recipient_bytes = crate::truncate_and_pad(&recipient_account_id.encode()[..]);
		let relayer_bytes = crate::truncate_and_pad(&relayer_account_id.encode()[..]);

		let (proof_bytes, roots_element, nullifier_hash_element, leaf_element) = setup_zk_circuit(
			curve,
			recipient_bytes,
			relayer_bytes,
			pk_bytes,
			src_chain_id,
			fee_value,
			refund_value,
		);
	});
}

#[test]
fn should_fail_with_when_fee_submitted_is_changed() {
	new_test_ext().execute_with(|| {
		let curve = Curve::Bn254;
		let pk_bytes = setup_environment(curve);

		// inputs
		let tree_id = create_vanchor(0);
		let src_chain_id = 1;
		let sender_account_id = account::<AccountId>("", 1, SEED);
		let recipient_account_id = account::<AccountId>("", 2, SEED);
		let relayer_account_id = account::<AccountId>("", 0, SEED);
		let fee_value = 0;
		let refund_value = 0;

		let recipient_bytes = crate::truncate_and_pad(&recipient_account_id.encode()[..]);
		let relayer_bytes = crate::truncate_and_pad(&relayer_account_id.encode()[..]);

		let (proof_bytes, roots_element, nullifier_hash_element, leaf_element) = setup_zk_circuit(
			curve,
			recipient_bytes,
			relayer_bytes,
			pk_bytes,
			src_chain_id,
			fee_value,
			refund_value,
		);
	});
}

#[test]
fn should_fail_with_invalid_proof_when_account_ids_are_truncated_in_reverse() {
	new_test_ext().execute_with(|| {
		let curve = Curve::Bn254;
		let pk_bytes = setup_environment(curve);

		// inputs
		let tree_id = create_vanchor(0);
		let src_chain_id = 1;
		let sender_account_id = account::<AccountId>("", 1, SEED);
		let recipient_account_id = account::<AccountId>("", 2, SEED);
		let relayer_account_id = account::<AccountId>("", 0, SEED);
		let fee_value = 0;
		let refund_value = 0;

		let recipient_bytes = truncate_and_pad_reverse(&recipient_account_id.encode()[..]);
		let relayer_bytes = truncate_and_pad_reverse(&relayer_account_id.encode()[..]);

		let (proof_bytes, roots_element, nullifier_hash_element, leaf_element) = setup_zk_circuit(
			curve,
			recipient_bytes,
			relayer_bytes,
			pk_bytes,
			src_chain_id,
			fee_value,
			refund_value,
		);
	});
}

#[test]
fn vanchor_works_for_pool_tokens() {
	new_test_ext().execute_with(|| {
		let existential_balance: u32 = 1000;
		let first_token_id = AssetRegistry::register_asset(
			b"shib".to_vec().try_into().unwrap(),
			AssetType::Token,
			existential_balance.into(),
		)
		.unwrap();
		let second_token_id = AssetRegistry::register_asset(
			b"doge".to_vec().try_into().unwrap(),
			AssetType::Token,
			existential_balance.into(),
		)
		.unwrap();

		let pool_share_id = AssetRegistry::register_asset(
			b"meme".to_vec().try_into().unwrap(),
			AssetType::PoolShare(vec![second_token_id, first_token_id]),
			existential_balance.into(),
		)
		.unwrap();

		let curve = Curve::Bn254;
		let pk_bytes = setup_environment(curve);

		// inputs
		let tree_id = create_vanchor(pool_share_id);
		let src_chain_id = 1;
		let sender_account_id = account::<AccountId>("", 1, SEED);
		let recipient_account_id = account::<AccountId>("", 2, SEED);
		let relayer_account_id = account::<AccountId>("", 0, SEED);
		let fee_value = 0;
		let refund_value = 0;
		let balance = 30_000u32;

		assert_ok!(Currencies::update_balance(
			Origin::root(),
			sender_account_id.clone(),
			first_token_id,
			balance.into()
		));

		assert_ok!(Currencies::update_balance(
			Origin::root(),
			sender_account_id.clone(),
			second_token_id,
			balance.into()
		));

		assert_ok!(TokenWrapper::set_wrapping_fee(Origin::root(), 0));

		assert_ok!(TokenWrapper::wrap(
			Origin::signed(sender_account_id.clone()),
			first_token_id,
			pool_share_id,
			10000 as u128,
			sender_account_id.clone()
		));

		assert_ok!(TokenWrapper::wrap(
			Origin::signed(sender_account_id.clone()),
			second_token_id,
			pool_share_id,
			10000 as u128,
			sender_account_id.clone()
		));

		assert_eq!(Tokens::total_issuance(pool_share_id), 20_000u32.into());

		let recipient_bytes = crate::truncate_and_pad(&recipient_account_id.encode()[..]);
		let relayer_bytes = crate::truncate_and_pad(&relayer_account_id.encode()[..]);

		let (proof_bytes, roots_element, nullifier_hash_element, leaf_element) = setup_zk_circuit(
			curve,
			recipient_bytes,
			relayer_bytes,
			pk_bytes,
			src_chain_id,
			fee_value,
			refund_value,
		);
	});
}

#[test]
fn should_run_post_deposit_hook_sucessfully() {
	new_test_ext().execute_with(|| {
		setup_environment(Curve::Bn254);

		let max_edges = M as _;
		let depth = TREE_DEPTH as _;
		let asset_id = 0;
		assert_ok!(VAnchor::create(Origin::root(), max_edges, depth, asset_id));

		let tree_id = MerkleTree::next_tree_id() - 1;
		let account_id = account::<AccountId>("", 1, SEED);
		let leaf = Element::from_bytes(&[1u8; 32]);
		// check the balance before the deposit.
		let balance_before = Balances::free_balance(account_id.clone());
		println!("Balance before: {}", balance_before);
	});
}
