use arkworks_gadgets::prelude::ark_ff::{BigInteger, Field, PrimeField};
use frame_support::{assert_err, assert_ok, traits::OnInitialize};
use sp_std::vec;

use super::*;
use crate::mock::*;

#[test]
fn shout_create_an_empty_tree() {
	new_test_ext().execute_with(|| {
		assert_ok!(MerkleTree::create(Origin::signed(1), 32));
	});
}

#[test]
fn should_fail_in_case_of_larger_depth() {
	new_test_ext().execute_with(|| {
		let max_depth = <Test as Config>::MaxTreeDepth::get();
		assert_err!(
			MerkleTree::create(Origin::signed(1), max_depth + 1),
			DispatchError::Module {
				index: 3,
				error: 1, // InvalidTreeDepth,
				message: None,
			}
		);
	});
}

#[test]
fn should_fail_in_case_when_max_default_hashes_is_exceeded() {
	new_test_ext().execute_with(|| {
		let max_default_hashes = <Test as Config>::MaxTreeDepth::get();
		assert_err!(
			MerkleTree::force_set_default_hashes(Origin::root(), vec![
				<Test as Config>::DefaultZeroElement::get();
				(max_default_hashes + 1) as usize
			]),
			crate::Error::<Test, _>::ExceedsMaxDefaultHashes
		);
	});
}

#[test]
fn should_be_able_to_insert_leaves() {
	new_test_ext().execute_with(|| {
		// init hasher pallet first.
		assert_ok!(HasherPallet::force_set_parameters(Origin::root(), hasher_params()));
		let depth = 3;
		assert_ok!(MerkleTree::create(Origin::signed(1), depth));
		let tree_id = MerkleTree::next_tree_id() - 1;
		let total_leaves_count = 2u32.pow(depth as _);
		let leaf = Element::from_bytes(&[1u8; 32]);
		(0..total_leaves_count).for_each(|_| {
			assert_ok!(MerkleTree::insert(Origin::signed(1), tree_id, leaf));
		});
	});
}

#[test]
fn should_fail_if_the_tree_is_full() {
	new_test_ext().execute_with(|| {
		// init hasher pallet first.
		assert_ok!(HasherPallet::force_set_parameters(Origin::root(), hasher_params()));
		let depth = 3;
		assert_ok!(MerkleTree::create(Origin::signed(1), depth));
		let tree_id = MerkleTree::next_tree_id() - 1;
		let total_leaves_count = 2u32.pow(depth as _);
		let leaf = Element::from_bytes(&[1u8; 32]);
		(0..total_leaves_count).for_each(|_| {
			assert_ok!(MerkleTree::insert(Origin::signed(1), tree_id, leaf));
		});
		assert_err!(
			MerkleTree::insert(Origin::signed(1), tree_id, leaf),
			DispatchError::Module {
				index: 3,
				error: 3, // ExceedsMaxLeaves
				message: None,
			}
		);
	});
}

#[test]
fn should_reach_same_root_as_js() {
	new_test_ext().execute_with(|| {
		// ```js
		// const tree = new MerkleTree(3, [1, 2]);
		// const root = await tree.root();
		// console.log({ root });
		// ```
		use ark_bn254::Fr;
		// init hasher pallet first.
		assert_ok!(HasherPallet::force_set_parameters(Origin::root(), hasher_params()));
		// init zero hashes.
		<MerkleTree as OnInitialize<u64>>::on_initialize(1);
		let depth = 3;
		assert_ok!(MerkleTree::create(Origin::signed(1), depth as _));
		let tree_id = MerkleTree::next_tree_id() - 1;
		let one = Fr::one();
		let two = one.double();
		let leaf_one = Element::from_bytes(&one.into_repr().to_bytes_le());
		let leaf_two = Element::from_bytes(&two.into_repr().to_bytes_le());
		assert_ok!(MerkleTree::insert(Origin::signed(1), tree_id, leaf_one));
		assert_ok!(MerkleTree::insert(Origin::signed(1), tree_id, leaf_two));
		let root = MerkleTree::get_root(tree_id).unwrap();
		let root = Fr::from_le_bytes_mod_order(root.to_bytes());
		let expected_root = ark_ff::field_new!(
			Fr,
			"4699987802398398943634862747629228556213954342379652513425782003585571710410"
		);
		assert_eq!(root, expected_root);
		assert_ok!(MerkleTree::is_known_root(
			tree_id,
			Element::from_bytes(&expected_root.into_repr().to_bytes_be())
		));
	});
}

#[test]
fn genesis_config_works() {
	ExtBuilder::default().with_crate_gen_config().execute_with(|| {
		assert!(!MerkleTree::is_default_hashes_empty());
	})
}

#[ignore = "used only for debugging"]
#[test]
pub fn shout_print_zero_element() {
	use ark_bn254::Fr;
	let f = ark_ff::field_new!(
		Fr,
		"21663839004416932945382355908790599225266501822907911457504978515578255421292"
	);
	let f_bytes = f.into_repr().to_bytes_le();
	dbg!(Element::from_bytes(&f_bytes));
}
