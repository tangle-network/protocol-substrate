use crate::test_utils::*;
use arkworks_gadgets::{
	poseidon::PoseidonParameters,
	setup::common::Curve,
	utils::{get_mds_poseidon_circom_bn254_x5_3, get_rounds_poseidon_circom_bn254_x5_3},
};
use codec::Encode;
use darkwebb_primitives::{merkle_tree::TreeInspector, AccountId, ElementTrait};
use frame_benchmarking::account;
use frame_support::{assert_err, assert_ok, traits::OnInitialize};
use sp_runtime::traits::One;

use crate::mock::*;

const SEED: u32 = 0;

fn hasher_params() -> Vec<u8> {
	let rounds = get_rounds_poseidon_circom_bn254_x5_3::<ark_bn254::Fr>();
	let mds = get_mds_poseidon_circom_bn254_x5_3::<ark_bn254::Fr>();
	let params = PoseidonParameters::new(rounds, mds);
	params.to_bytes()
}

fn setup_environment(curve: Curve) -> Vec<u8> {
	let params = match curve {
		Curve::Bn254 => get_hash_params::<ark_bn254::Fr>(curve),
		Curve::Bls381 => get_hash_params::<ark_bls12_381::Fr>(curve),
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

	// finally return the provingkey bytes
	proving_key_bytes
}

#[test]
fn should_create_new_mixer() {
	new_test_ext().execute_with(|| {
		// init hasher pallet first.
		assert_ok!(HasherPallet::force_set_parameters(Origin::root(), hasher_params()));
		// then the merkle tree.
		<MerkleTree as OnInitialize<u64>>::on_initialize(1);
		assert_ok!(Mixer::create(Origin::root(), One::one(), 3, 0));
	});
}
#[test]
fn should_be_able_to_deposit() {
	new_test_ext().execute_with(|| {
		// init hasher pallet first.
		assert_ok!(HasherPallet::force_set_parameters(Origin::root(), hasher_params()));
		// then the merkle tree.
		<MerkleTree as OnInitialize<u64>>::on_initialize(1);
		let deposit_size = One::one();
		assert_ok!(Mixer::create(Origin::root(), deposit_size, 3, 0));
		let tree_id = MerkleTree::next_tree_id() - 1;
		let account_id = account::<AccountId>("", 1, SEED);
		let leaf = Element::from_bytes(&[1u8; 32]);
		// check the balance before the deposit.
		let balance_before = Balances::free_balance(account_id.clone());
		// and we do the deposit
		assert_ok!(Mixer::deposit(Origin::signed(account_id.clone()), tree_id, leaf));
		// now we check the balance after the deposit.
		let balance_after = Balances::free_balance(account_id);
		// the balance should be less now with `deposit_size`
		assert_eq!(balance_after, balance_before - deposit_size);
		// now we need also to check if the state got updated.
		let tree = MerkleTree::trees(tree_id);
		assert_eq!(tree.leaf_count, 1);
	});
}
#[test]
fn should_be_able_to_change_the_maintainer() {
	new_test_ext().execute_with(|| {
		assert_ok!(Mixer::create(Origin::root(), One::one(), 3, 0));
		let default_maintainer_account_id = AccountId::default();
		let current_maintainer_account_id = Mixer::maintainer();
		assert_eq!(current_maintainer_account_id, default_maintainer_account_id);
		let new_maintainer_account_id = account::<AccountId>("", 1, SEED);
		assert_ok!(Mixer::force_set_maintainer(
			Origin::root(),
			new_maintainer_account_id.clone()
		));
		let current_maintainer_account_id = Mixer::maintainer();
		assert_eq!(current_maintainer_account_id, new_maintainer_account_id);
	});
}

#[test]
fn mixer_works() {
	new_test_ext().execute_with(|| {
		let curve = Curve::Bn254;
		let pk_bytes = setup_environment(curve);
		// now let's create the mixer.
		let deposit_size = One::one();
		assert_ok!(Mixer::create(Origin::root(), deposit_size, 30, 0));
		// now with mixer created, we should setup the circuit.
		let tree_id = MerkleTree::next_tree_id() - 1;
		let sender_account_id = account::<AccountId>("", 1, SEED);
		let recipient_account_id = account::<AccountId>("", 2, SEED);
		let relayer_account_id = account::<AccountId>("", 0, SEED);
		let fee_value = 0;
		let refund_value = 0;

		// inputs
		let recipient_bytes = crate::truncate_and_pad(&recipient_account_id.encode()[..]);
		let relayer_bytes = crate::truncate_and_pad(&relayer_account_id.encode()[..]);

		let (proof_bytes, roots_element, nullifier_hash_element, leaf_element) =
			setup_zk_circuit(curve, recipient_bytes, relayer_bytes, pk_bytes, fee_value, refund_value);

		assert_ok!(Mixer::deposit(
			Origin::signed(sender_account_id.clone()),
			tree_id,
			leaf_element,
		));
		// check the balance before the withdraw.
		let balance_before = Balances::free_balance(recipient_account_id.clone());

		let mixer_tree_root = MerkleTree::get_root(tree_id).unwrap();
		assert_eq!(roots_element[0], mixer_tree_root);

		assert_ok!(Mixer::withdraw(
			Origin::signed(sender_account_id),
			tree_id,
			proof_bytes,
			roots_element[0],
			nullifier_hash_element,
			recipient_account_id.clone(),
			relayer_account_id,
			fee_value.into(),
			refund_value.into(),
		));
		// now we check the recipient balance again.
		let balance_after = Balances::free_balance(recipient_account_id);
		assert_eq!(balance_after, balance_before + deposit_size);
		// perfect
	});
}

#[test]
fn mixer_should_fail_with_when_proof_when_any_byte_is_changed_in_proof() {
	new_test_ext().execute_with(|| {
		let curve = Curve::Bn254;
		let pk_bytes = setup_environment(curve);

		let deposit_size = One::one();
		assert_ok!(Mixer::create(Origin::root(), deposit_size, 30, 0));
		// now with mixer created, we should setup the circuit.
		let tree_id = MerkleTree::next_tree_id() - 1;
		let sender_account_id = account::<AccountId>("", 1, SEED);
		let recipient_account_id = account::<AccountId>("", 2, SEED);
		let relayer_account_id = account::<AccountId>("", 0, SEED);
		let fee_value = 0;
		let refund_value = 0;

		// inputs
		let recipient_bytes = crate::truncate_and_pad(&recipient_account_id.encode()[..]);
		let relayer_bytes = crate::truncate_and_pad(&relayer_account_id.encode()[..]);

		let (mut proof_bytes, roots_element, nullifier_hash_element, leaf_element) =
			setup_zk_circuit(curve, recipient_bytes, relayer_bytes, pk_bytes, fee_value, refund_value);

		assert_ok!(Mixer::deposit(
			Origin::signed(sender_account_id.clone()),
			tree_id,
			leaf_element,
		));

		let root_element = roots_element[0];
		let mixer_tree_root = MerkleTree::get_root(tree_id).unwrap();
		assert_eq!(root_element, mixer_tree_root);

		let a = proof_bytes[0];
		let b = proof_bytes[1];
		proof_bytes[0] = b;
		proof_bytes[1] = a;

		assert_err!(
			Mixer::withdraw(
				Origin::signed(sender_account_id),
				tree_id,
				proof_bytes,
				root_element,
				nullifier_hash_element,
				recipient_account_id,
				relayer_account_id,
				fee_value.into(),
				refund_value.into(),
			),
			crate::Error::<Test>::InvalidWithdrawProof
		);
	});
}

#[test]
fn mixer_should_fail_when_invalid_merkle_roots() {
	new_test_ext().execute_with(|| {
		let curve = Curve::Bn254;

		let pk_bytes = setup_environment(curve);

		let deposit_size = One::one();
		assert_ok!(Mixer::create(Origin::root(), deposit_size, 30, 0));
		// now with mixer created, we should setup the circuit.
		let tree_id = MerkleTree::next_tree_id() - 1;
		let sender_account_id = account::<AccountId>("", 1, SEED);
		let recipient_account_id = account::<AccountId>("", 2, SEED);
		let relayer_account_id = account::<AccountId>("", 0, SEED);
		let fee_value = 0;
		let refund_value = 0;

		// inputs
		let recipient_bytes = crate::truncate_and_pad(&recipient_account_id.encode()[..]);
		let relayer_bytes = crate::truncate_and_pad(&relayer_account_id.encode()[..]);

		let (proof_bytes, roots_element, nullifier_hash_element, leaf_element) =
			setup_zk_circuit(curve, recipient_bytes, relayer_bytes, pk_bytes, fee_value, refund_value);

		assert_ok!(Mixer::deposit(
			Origin::signed(sender_account_id.clone()),
			tree_id,
			leaf_element,
		));

		let mut root_element_bytes = roots_element[0].to_bytes().to_vec();
		let a = root_element_bytes[0];
		let b = root_element_bytes[1];
		root_element_bytes[0] = b;
		root_element_bytes[1] = a;
		let root_element = Element::from_bytes(&root_element_bytes[..]);

		// now we start to generate the proof.
		assert_err!(
			Mixer::withdraw(
				Origin::signed(sender_account_id),
				tree_id,
				proof_bytes,
				root_element,
				nullifier_hash_element,
				recipient_account_id,
				relayer_account_id,
				fee_value.into(),
				refund_value.into(),
			),
			crate::Error::<Test>::UnknownRoot
		);
	});
}

#[test]
fn mixer_should_fail_when_relayer_id_is_different_from_that_in_proof_generation() {
	new_test_ext().execute_with(|| {
		let curve = Curve::Bn254;
		let pk_bytes = setup_environment(curve);

		let deposit_size = One::one();
		assert_ok!(Mixer::create(Origin::root(), deposit_size, 30, 0));
		// now with mixer created, we should setup the circuit.
		let tree_id = MerkleTree::next_tree_id() - 1;
		let sender_account_id = account::<AccountId>("", 1, SEED);
		let recipient_account_id = account::<AccountId>("", 2, SEED);
		let relayer_account_id = account::<AccountId>("", 0, SEED);
		let fee_value = 0;
		let refund_value = 0;

		// inputs
		let recipient_bytes = crate::truncate_and_pad(&recipient_account_id.encode()[..]);
		let relayer_bytes = crate::truncate_and_pad(&relayer_account_id.encode()[..]);

		let (proof_bytes, roots_element, nullifier_hash_element, leaf_element) =
			setup_zk_circuit(curve, recipient_bytes, relayer_bytes, pk_bytes, fee_value, refund_value);

		assert_ok!(Mixer::deposit(
			Origin::signed(sender_account_id.clone()),
			tree_id,
			leaf_element,
		));

		let root_element = roots_element[0];
		let mixer_tree_root = MerkleTree::get_root(tree_id).unwrap();
		assert_eq!(root_element, mixer_tree_root);

		assert_err!(
			Mixer::withdraw(
				Origin::signed(sender_account_id.clone()),
				tree_id,
				proof_bytes,
				root_element,
				nullifier_hash_element,
				recipient_account_id,
				sender_account_id,
				fee_value.into(),
				refund_value.into(),
			),
			crate::Error::<Test>::InvalidWithdrawProof
		);
	});
}

#[test]
fn mixer_should_fail_with_when_fee_submitted_is_changed() {
	new_test_ext().execute_with(|| {
		let curve = Curve::Bn254;
		let pk_bytes = setup_environment(curve);

		let deposit_size = One::one();
		assert_ok!(Mixer::create(Origin::root(), deposit_size, 30, 0));
		// now with mixer created, we should setup the circuit.
		let tree_id = MerkleTree::next_tree_id() - 1;
		let sender_account_id = account::<AccountId>("", 1, SEED);
		let recipient_account_id = account::<AccountId>("", 2, SEED);
		let relayer_account_id = account::<AccountId>("", 0, SEED);
		let fee_value = 0;
		let refund_value = 0;

		// inputs
		let recipient_bytes = crate::truncate_and_pad(&recipient_account_id.encode()[..]);
		let relayer_bytes = crate::truncate_and_pad(&relayer_account_id.encode()[..]);

		let (proof_bytes, roots_element, nullifier_hash_element, leaf_element) =
			setup_zk_circuit(curve, recipient_bytes, relayer_bytes, pk_bytes, fee_value, refund_value);

		assert_ok!(Mixer::deposit(
			Origin::signed(sender_account_id.clone()),
			tree_id,
			leaf_element,
		));

		let root_element = roots_element[0];
		let mixer_tree_root = MerkleTree::get_root(tree_id).unwrap();
		assert_eq!(root_element, mixer_tree_root);

		assert_err!(
			Mixer::withdraw(
				Origin::signed(sender_account_id),
				tree_id,
				proof_bytes,
				root_element,
				nullifier_hash_element,
				recipient_account_id,
				relayer_account_id,
				100u128,
				refund_value.into(),
			),
			crate::Error::<Test>::InvalidWithdrawProof
		);
	});
}

#[test]
fn mixer_should_fail_with_invalid_proof_when_account_ids_are_truncated_in_reverse() {
	new_test_ext().execute_with(|| {
		let curve = Curve::Bn254;
		let pk_bytes = setup_environment(curve);

		let deposit_size = One::one();
		assert_ok!(Mixer::create(Origin::root(), deposit_size, 30, 0));
		// now with mixer created, we should setup the circuit.
		let tree_id = MerkleTree::next_tree_id() - 1;
		let sender_account_id = account::<AccountId>("", 1, SEED);
		let recipient_account_id = account::<AccountId>("", 2, SEED);
		let relayer_account_id = account::<AccountId>("", 0, SEED);
		let fee_value = 0;
		let refund_value = 0;

		// inputs
		let recipient_bytes = truncate_and_pad_reverse(&recipient_account_id.encode()[..]);
		let relayer_bytes = truncate_and_pad_reverse(&relayer_account_id.encode()[..]);

		let (proof_bytes, roots_element, nullifier_hash_element, leaf_element) =
			setup_zk_circuit(curve, recipient_bytes, relayer_bytes, pk_bytes, fee_value, refund_value);

		assert_ok!(Mixer::deposit(
			Origin::signed(sender_account_id.clone()),
			tree_id,
			leaf_element,
		));

		let root_element = roots_element[0];
		let mixer_tree_root = MerkleTree::get_root(tree_id).unwrap();
		assert_eq!(root_element, mixer_tree_root);

		assert_err!(
			Mixer::withdraw(
				Origin::signed(sender_account_id),
				tree_id,
				proof_bytes,
				root_element,
				nullifier_hash_element,
				recipient_account_id,
				relayer_account_id,
				fee_value.into(),
				refund_value.into(),
			),
			crate::Error::<Test>::InvalidWithdrawProof
		);
	});
}

#[test]
fn double_spending_should_fail() {
	new_test_ext().execute_with(|| {
		let curve = Curve::Bn254;
		let pk_bytes = setup_environment(curve);

		let deposit_size = One::one();
		assert_ok!(Mixer::create(Origin::root(), deposit_size, 30, 0));
		// now with mixer created, we should setup the circuit.
		let tree_id = MerkleTree::next_tree_id() - 1;
		let sender_account_id = account::<AccountId>("", 1, SEED);
		let recipient_account_id = account::<AccountId>("", 2, SEED);
		let relayer_account_id = account::<AccountId>("", 0, SEED);
		let fee_value = 0;
		let refund_value = 0;

		// inputs
		let recipient_bytes = crate::truncate_and_pad(&recipient_account_id.encode()[..]);
		let relayer_bytes = crate::truncate_and_pad(&relayer_account_id.encode()[..]);

		let (proof_bytes, roots_element, nullifier_hash_element, leaf_element) =
			setup_zk_circuit(curve, recipient_bytes, relayer_bytes, pk_bytes, fee_value, refund_value);

		assert_ok!(Mixer::deposit(
			Origin::signed(sender_account_id.clone()),
			tree_id,
			leaf_element,
		));

		let root_element = roots_element[0];
		let mixer_tree_root = MerkleTree::get_root(tree_id).unwrap();
		assert_eq!(root_element, mixer_tree_root);

		let balance_before = Balances::free_balance(recipient_account_id.clone());

		assert_ok!(Mixer::withdraw(
			Origin::signed(sender_account_id.clone()),
			tree_id,
			proof_bytes.clone(),
			root_element,
			nullifier_hash_element,
			recipient_account_id.clone(),
			relayer_account_id.clone(),
			fee_value.into(),
			refund_value.into(),
		));
		// now we check the recipient balance again.
		let balance_after = Balances::free_balance(recipient_account_id.clone());
		assert_eq!(balance_after, balance_before + deposit_size);
		// perfect

		assert_err!(
			Mixer::withdraw(
				Origin::signed(sender_account_id),
				tree_id,
				proof_bytes,
				root_element,
				nullifier_hash_element,
				recipient_account_id,
				relayer_account_id,
				fee_value.into(),
				refund_value.into(),
			),
			crate::Error::<Test>::AlreadyRevealedNullifier
		);
	});
}
