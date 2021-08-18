use arkworks_gadgets::{
	poseidon::PoseidonParameters,
	utils::{get_mds_poseidon_circom_bn254_x5_3, get_rounds_poseidon_circom_bn254_x5_3},
};
use frame_support::{assert_err, assert_ok};

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
