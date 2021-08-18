use arkworks_gadgets::{
	poseidon::PoseidonParameters,
	prelude::ark_ff::{BigInteger, Field, PrimeField},
	utils::{get_mds_poseidon_circom_bn254_x5_3, get_rounds_poseidon_circom_bn254_x5_3},
};
use frame_support::{assert_err, assert_ok, traits::OnInitialize};

use super::*;
use crate::mock::*;

fn hasher_params() -> Vec<u8> {
	let rounds = get_rounds_poseidon_circom_bn254_x5_3::<ark_bn254::Fr>();
	let mds = get_mds_poseidon_circom_bn254_x5_3::<ark_bn254::Fr>();
	let params = PoseidonParameters::new(rounds, mds);
	params.to_bytes()
}

#[test]
fn shout_create_an_empty_tree() {
	new_test_ext().execute_with(|| {
		assert_ok!(Smt::create(Origin::signed(1), 32));
	});
}

#[test]
fn should_fail_in_case_of_larger_depth() {
	new_test_ext().execute_with(|| {
		let max_depth = <Test as Config>::MaxTreeDepth::get();
		assert_err!(Smt::create(Origin::signed(1), max_depth + 1), DispatchError::Module {
			index: 3,
			error: 1, // InvalidTreeDepth,
			message: None,
		});
	});
}

#[test]
fn should_be_able_to_insert_leaves() {
	new_test_ext().execute_with(|| {
		// init hasher pallet first.
		assert_ok!(HasherPallet::force_set_parameters(Origin::root(), hasher_params()));
		let depth = 3;
		assert_ok!(Smt::create(Origin::signed(1), depth));
		let tree_id = Smt::next_tree_id() - 1;
		let total_leaves_count = 2u32.pow(depth as _);
		let leaf = Element::from_bytes(&[1u8; 32]);
		(0..total_leaves_count).for_each(|_| {
			assert_ok!(Smt::insert(Origin::signed(1), tree_id, leaf));
		});
	});
}

#[test]
fn should_fail_if_the_tree_is_full() {
	new_test_ext().execute_with(|| {
		// init hasher pallet first.
		assert_ok!(HasherPallet::force_set_parameters(Origin::root(), hasher_params()));
		let depth = 3;
		assert_ok!(Smt::create(Origin::signed(1), depth));
		let tree_id = Smt::next_tree_id() - 1;
		let total_leaves_count = 2u32.pow(depth as _);
		let leaf = Element::from_bytes(&[1u8; 32]);
		(0..total_leaves_count).for_each(|_| {
			assert_ok!(Smt::insert(Origin::signed(1), tree_id, leaf));
		});
		assert_err!(Smt::insert(Origin::signed(1), tree_id, leaf), DispatchError::Module {
			index: 3,
			error: 3, // ExceedsMaxLeaves
			message: None,
		});
	});
}

#[test]
fn should_reach_same_root_as_js() {
	new_test_ext().execute_with(|| {
		use ark_bn254::Fr;
		// init hasher pallet first.
		assert_ok!(HasherPallet::force_set_parameters(Origin::root(), hasher_params()));
		// init zero hashes.
		<Smt as OnInitialize<u64>>::on_initialize(1);
		let depth = 32;
		assert_ok!(Smt::create(Origin::signed(1), depth));
		let tree_id = Smt::next_tree_id() - 1;
		let one = Fr::one();
		let two = one.double();
		println!("{}\n{}", one, two);
		let leaf_one = Element::from_bytes(&one.into_repr().to_bytes_be());
		let leaf_two = Element::from_bytes(&two.into_repr().to_bytes_be());
		assert_ok!(Smt::insert(Origin::signed(1), tree_id, leaf_one));
		assert_ok!(Smt::insert(Origin::signed(1), tree_id, leaf_two));
		let root = Smt::get_root(tree_id).unwrap();
		let root = Fr::from_be_bytes_mod_order(root.to_bytes());
		let expected_root = ark_ff::field_new!(
			Fr,
			"12221176053742666810995205403876748559952118879983760689257065293017064981901"
		);
		println!("root: {} and expected: {}", root, expected_root);
		assert_eq!(root, expected_root);
	});
}
