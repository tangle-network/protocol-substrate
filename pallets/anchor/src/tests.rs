use ark_ff::{BigInteger, FromBytes, PrimeField};
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use arkworks_gadgets::{
	poseidon::PoseidonParameters,
	prelude::ark_groth16::ProvingKey,
	setup::{
		bridge::{
			prove_groth16_circuit_circomx5, setup_arbitrary_data, setup_groth16_random_circuit_circomx5,
			setup_leaf_circomx5, setup_set, Circuit_Circomx5,
		},
		common::{setup_circom_params_x5_3, setup_circom_params_x5_5, setup_tree_and_create_path_tree_circomx5, Curve},
	},
	utils::{get_mds_poseidon_circom_bn254_x5_3, get_rounds_poseidon_circom_bn254_x5_3},
};

use darkwebb_primitives::{
	anchor::{AnchorInspector, AnchorInterface},
	merkle_tree::TreeInspector,
	AccountId, ElementTrait,
};

use codec::Encode;

use frame_benchmarking::account;
use frame_support::{assert_err, assert_ok, error::BadOrigin, traits::OnInitialize};

use crate::{mock::*, test_utils::*};

const SEED: u32 = 0;
const TREE_DEPTH: usize = 30;
const M: usize = 2;
const DEPOSIT_SIZE: u128 = 10_000;

fn setup_environment(curve: Curve) -> Vec<u8> {
	let rng = &mut ark_std::test_rng();
	let params = match curve {
		Curve::Bn254 => get_hash_params::<ark_bn254::Fr>(curve),
		Curve::Bls381 => {
			todo!("Setup hash params for bls381")
		}
	};
	// 1. Setup The Hasher Pallet.
	assert_ok!(HasherPallet::force_set_parameters(Origin::root(), params.to_bytes()));
	// 2. Initialize MerkleTree pallet.
	<MerkleTree as OnInitialize<u64>>::on_initialize(1);
	// 3. Setup the VerifierPallet
	//    but to do so, we need to have a VerifyingKey
	let mut verifier_key_bytes = Vec::new();
	let mut proving_key_bytes = Vec::new();

	get_keys(curve.clone(), &mut proving_key_bytes, &mut verifier_key_bytes);

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
fn should_create_new_anchor() {
	new_test_ext().execute_with(|| {
		setup_environment(Curve::Bn254);
		let max_edges = M as _;
		let depth = TREE_DEPTH as u8;
		let asset_id = 0;
		assert_ok!(Anchor::create(Origin::root(), DEPOSIT_SIZE, max_edges, depth, asset_id));
	});
}

#[test]
fn should_fail_to_create_new_anchor_if_not_root() {
	new_test_ext().execute_with(|| {
		setup_environment(Curve::Bn254);
		let max_edges = M as _;
		let depth = TREE_DEPTH as u8;
		let asset_id = 0;
		assert_err!(
			Anchor::create(
				Origin::signed(account::<AccountId>("", 1, SEED)),
				DEPOSIT_SIZE,
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

		let max_edges = M as _;
		let depth = TREE_DEPTH as _;
		let asset_id = 0;
		assert_ok!(Anchor::create(Origin::root(), DEPOSIT_SIZE, max_edges, depth, asset_id));

		let tree_id = MerkleTree::next_tree_id() - 1;
		let account_id = account::<AccountId>("", 1, SEED);
		let leaf = Element::from_bytes(&[1u8; 32]);
		// check the balance before the deposit.
		let balance_before = Balances::free_balance(account_id.clone());
		println!("Balance before: {}", balance_before);
		// and we do the deposit
		assert_ok!(Anchor::deposit(Origin::signed(account_id.clone()), tree_id, leaf));
		// now we check the balance after the deposit.
		let balance_after = Balances::free_balance(account_id);
		// the balance should be less now with `deposit_size`
		assert_eq!(balance_after, balance_before - DEPOSIT_SIZE);
		// now we need also to check if the state got updated.
		let tree = MerkleTree::trees(tree_id);
		assert_eq!(tree.leaf_count, 1);
	});
}

#[test]
fn should_fail_to_deposit_if_mixer_not_found() {
	new_test_ext().execute_with(|| {
		setup_environment(Curve::Bn254);
		assert_err!(
			Anchor::deposit(type Bn254Fr = ark_bn254::Fr;
				Origin::signed(account::<AccountId>("", 1, SEED)),
				2,
				Element::from_bytes(&[1u8; 32])
			),
			pallet_mixer::Error::<Test, _>::NoMixerFound,
		);
	});
}

#[test]
fn should_be_able_to_change_the_maintainer() {
	new_test_ext().execute_with(|| {
		setup_environment(Curve::Bn254);
		let max_edges = M as _;
		let depth = TREE_DEPTH as u8;
		let asset_id = 0;
		assert_ok!(Anchor::create(Origin::root(), DEPOSIT_SIZE, max_edges, depth, asset_id));

		let default_maintainer_account_id = AccountId::default();
		let current_maintainer_account_id = Anchor::maintainer();
		assert_eq!(current_maintainer_account_id, default_maintainer_account_id);

		let new_maintainer_account_id = account::<AccountId>("", 1, SEED);
		assert_ok!(Anchor::force_set_maintainer(
			Origin::root(),
			new_maintainer_account_id.clone()
		));
		let current_maintainer_account_id = Anchor::maintainer();
		assert_eq!(current_maintainer_account_id, new_maintainer_account_id);
	});
}

#[test]
fn should_fail_to_change_the_maintainer_if_not_the_current_maintainer() {
	new_test_ext().execute_with(|| {
		setup_environment(Curve::Bn254);
		let max_edges = M as _;
		let depth = TREE_DEPTH as u8;
		let asset_id = 0;
		assert_ok!(Anchor::create(Origin::root(), DEPOSIT_SIZE, max_edges, depth, asset_id));
		let current_maintainer_account_id = Anchor::maintainer();
		let new_maintainer_account_id = account::<AccountId>("", 1, SEED);
		assert_err!(
			Anchor::set_maintainer(
				Origin::signed(account::<AccountId>("", 2, SEED)),
				new_maintainer_account_id
			),
			crate::Error::<Test, _>::InvalidPermissions,
		);
		// maintainer should never be changed.
		assert_eq!(current_maintainer_account_id, Anchor::maintainer());
	});
}

#[test]
fn should_be_able_to_add_neighbors_and_check_history() {
	use rand::prelude::*;

	new_test_ext().execute_with(|| {
		setup_environment(Curve::Bn254);

		let max_edges = M as _;
		let depth = TREE_DEPTH as u8;
		let asset_id = 0;
		assert_ok!(Anchor::create(Origin::root(), DEPOSIT_SIZE, max_edges, depth, asset_id));
		let tree_id = MerkleTree::next_tree_id() - 1;
		let src_chain_id = 1;
		let root = Element::from_bytes(&[1u8; 32]);
		let height = 1;
		let neighbor_index_before: u32 = Anchor::curr_neighbor_root_index((tree_id, src_chain_id));
		assert_ok!(<Anchor as AnchorInterface<_>>::add_edge(
			tree_id,
			src_chain_id,
			root,
			height,
		));
		let neighbor_index_after: u32 = Anchor::curr_neighbor_root_index((tree_id, src_chain_id));
		assert_eq!(neighbor_index_after, neighbor_index_before + 1);

		for _ in 0..(HistoryLength::get() - 1) {
			assert_eq!(
				Anchor::is_known_neighbor_root(tree_id, src_chain_id, root).unwrap(),
				true
			);

			let val = thread_rng().gen::<[u8; 32]>();
			let elt = Element::from_bytes(&val);
			assert_ok!(<Anchor as AnchorInterface<_>>::update_edge(
				tree_id,
				src_chain_id,
				elt,
				height,
			));

			assert_eq!(
				Anchor::is_known_neighbor_root(tree_id, src_chain_id, elt).unwrap(),
				true
			);
		}

		assert_eq!(
			Anchor::is_known_neighbor_root(tree_id, src_chain_id, root).unwrap(),
			false
		);
	});
}

fn create_anchor() -> u32 {
	let max_edges = 2;
	let depth = TREE_DEPTH as u8;
	let asset_id = 0;
	assert_ok!(Anchor::create(Origin::root(), DEPOSIT_SIZE, max_edges, depth, asset_id));
	MerkleTree::next_tree_id() - 1
}

#[test]
fn anchor_works() {
	type Bn254Fr = ark_bn254::Fr;
	new_test_ext().execute_with(|| {
		let curve = Curve::Bn254;
		let pk_bytes = setup_environment(curve);

		let rng = &mut ark_std::test_rng();

		// inputs
		let tree_id = create_anchor();
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

		assert_ok!(Anchor::deposit(
			Origin::signed(sender_account_id.clone()),
			tree_id,
			leaf_element.clone(),
		));

		let tree_root = MerkleTree::get_root(tree_id).unwrap();
		// sanity check.
		assert_eq!(roots_element[0], tree_root);

		let balance_before = Balances::free_balance(recipient_account_id.clone());
		// fire the call.
		assert_ok!(Anchor::withdraw(
			Origin::signed(sender_account_id),
			tree_id,
			proof_bytes,
			src_chain_id,
			roots_element,
			nullifier_hash_element,
			recipient_account_id.clone(),
			relayer_account_id,
			fee_value.into(),
			refund_value.into(),
		));
		// now we check the recipient balance again.
		let balance_after = Balances::free_balance(recipient_account_id);
		assert_eq!(balance_after, balance_before + DEPOSIT_SIZE);
		// perfect
	});
}

#[test]
fn double_spending_should_fail() {
	type Bn254Fr = ark_bn254::Fr;
	new_test_ext().execute_with(|| {
		let curve = Curve::Bn254;
		let pk_bytes = setup_environment(curve);
		let rng = &mut ark_std::test_rng();

		// inputs
		let tree_id = create_anchor();
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

		assert_ok!(Anchor::deposit(
			Origin::signed(sender_account_id.clone()),
			tree_id,
			leaf_element.clone(),
		));

		let tree_root = MerkleTree::get_root(tree_id).unwrap();
		assert_eq!(roots_element[0], tree_root);
		// all ready, call withdraw.
		// but first check the balance before that.

		let balance_before = Balances::free_balance(recipient_account_id.clone());
		// fire the call.
		assert_ok!(Anchor::withdraw(
			Origin::signed(sender_account_id.clone()),
			tree_id,
			proof_bytes.clone(),
			src_chain_id,
			roots_element.clone(),
			nullifier_hash_element,
			recipient_account_id.clone(),
			relayer_account_id.clone(),
			fee_value.into(),
			refund_value.into(),
		));
		// now we check the recipient balance again.
		let balance_after = Balances::free_balance(recipient_account_id.clone());
		assert_eq!(balance_after, balance_before + DEPOSIT_SIZE);
		// perfect

		// now double spending should fail.
		assert_err!(
			Anchor::withdraw(
				Origin::signed(sender_account_id),
				tree_id,
				proof_bytes,
				src_chain_id,
				roots_element,
				nullifier_hash_element,
				recipient_account_id,
				relayer_account_id,
				fee_value.into(),
				refund_value.into(),
			),
			pallet_mixer::Error::<Test, _>::AlreadyRevealedNullifier
		);
	});
}

#[test]
fn should_fail_when_invalid_merkle_roots() {
	type Bn254Fr = ark_bn254::Fr;
	new_test_ext().execute_with(|| {
		let curve = Curve::Bn254;
		let pk_bytes = setup_environment(curve);

		let rng = &mut ark_std::test_rng();

		// inputs
		let tree_id = create_anchor();
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

		assert_ok!(Anchor::deposit(
			Origin::signed(sender_account_id.clone()),
			tree_id,
			leaf_element.clone(),
		));

		let tree_root = MerkleTree::get_root(tree_id).unwrap();
		assert_eq!(roots_element[0], tree_root);

		// invalid root length
		roots_element.push(Element::from_bytes(&Bn254Fr::default().into_repr().to_bytes_le()));
		// all ready, call withdraw.

		// fire the call.
		assert_err!(
			Anchor::withdraw(
				Origin::signed(sender_account_id),
				tree_id,
				proof_bytes,
				src_chain_id,
				roots_element,
				nullifier_hash_element,
				recipient_account_id,
				relayer_account_id,
				fee_value.into(),
				refund_value.into(),
			),
			crate::Error::<Test, _>::InvalidMerkleRoots,
		);
	});
}

#[test]
fn mixer_should_fail_with_when_proof_when_any_byte_is_changed_in_proof() {}

#[test]
fn mixer_should_fail_when_relayer_id_is_different_from_that_in_proof_generation() {}

#[test]
fn mixer_should_fail_with_when_fee_submitted_is_changed() {}

#[test]
fn mixer_should_fail_with_invalid_proof_when_account_ids_are_truncated_in_reverse() {}
