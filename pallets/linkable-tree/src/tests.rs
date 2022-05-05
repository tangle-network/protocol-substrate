use arkworks_setups::{common::setup_params, Curve};
use frame_benchmarking::account;
use frame_support::{assert_err, assert_ok, error::BadOrigin};
use webb_primitives::{AccountId, ElementTrait};

use super::*;
use crate::mock::*;

const SEED: u32 = 0;
const TREE_DEPTH: usize = 30;
const M: usize = 2;

#[test]
fn should_create_new_linkable_tree() {
	new_test_ext().execute_with(|| {
		let curve = Curve::Bn254;
		let params = setup_params::<ark_bn254::Fr>(curve, 5, 3);
		let res = HasherPallet::force_set_parameters(Origin::root(), params.to_bytes());

		let max_edges = M as _;
		let depth = TREE_DEPTH as u8;
		assert_ok!(LinkableTree::create(Origin::root(), max_edges, depth));
		let id = MerkleTree::next_tree_id() - 1;
		let root = <LinkableTree as LinkableTreeInspector<_>>::get_root(id);
		assert_eq!(
			root.unwrap(),
			Element::from_bytes(&[
				160, 138, 218, 95, 86, 180, 30, 11, 21, 87, 76, 148, 219, 172, 9, 169, 157, 121,
				22, 135, 145, 189, 248, 188, 120, 227, 71, 137, 95, 88, 21, 31
			])
		);
	});
}

#[test]
fn should_fail_to_create_new_linkable_tree_if_not_root() {
	new_test_ext().execute_with(|| {
		let max_edges = M as _;
		let depth = TREE_DEPTH as u8;
		assert_err!(
			LinkableTree::create(
				Origin::signed(account::<AccountId>("", 1, SEED)),
				max_edges,
				depth,
			),
			BadOrigin,
		);
	});
}

#[test]
fn should_be_able_to_add_neighbors_and_check_history() {
	use rand::prelude::*;

	new_test_ext().execute_with(|| {
		let curve = Curve::Bn254;
		let params = setup_params::<ark_bn254::Fr>(curve, 5, 3);
		let res = HasherPallet::force_set_parameters(Origin::root(), params.to_bytes());

		let max_edges = M as _;
		let depth = TREE_DEPTH as u8;
		assert_ok!(LinkableTree::create(Origin::root(), max_edges, depth));
		let tree_id = MerkleTree::next_tree_id() - 1;
		let src_chain_id = 1;
		let root = Element::from_bytes(&[1u8; 32]);
		let height = 1;
		let neighbor_index_before: u32 =
			LinkableTree::curr_neighbor_root_index((tree_id, src_chain_id));
		let target = Element::from_bytes(&tree_id.to_le_bytes());
		assert_ok!(<LinkableTree as LinkableTreeInterface<_>>::add_edge(
			tree_id,
			src_chain_id,
			root,
			height,
			target,
		));
		let neighbor_index_after: u32 =
			LinkableTree::curr_neighbor_root_index((tree_id, src_chain_id));
		assert_eq!(neighbor_index_after, neighbor_index_before + 1);

		for _ in 0..(HistoryLength::get() - 1) {
			assert!(LinkableTree::is_known_neighbor_root(tree_id, src_chain_id, root).unwrap(),);

			let val = thread_rng().gen::<[u8; 32]>();
			let elt = Element::from_bytes(&val);
			let target = Element::from_bytes(&tree_id.to_le_bytes());
			assert_ok!(<LinkableTree as LinkableTreeInterface<_>>::update_edge(
				tree_id,
				src_chain_id,
				elt,
				height,
				target
			));

			assert!(LinkableTree::is_known_neighbor_root(tree_id, src_chain_id, elt).unwrap(),);
		}

		assert!(!LinkableTree::is_known_neighbor_root(tree_id, src_chain_id, root).unwrap(),);
	});
}
