use webb_primitives::{AccountId, ElementTrait};

use frame_benchmarking::account;
use frame_support::{assert_err, assert_ok, error::BadOrigin};

use super::*;
use crate::mock::*;

const SEED: u32 = 0;
const TREE_DEPTH: usize = 30;
const M: usize = 2;

#[test]
fn should_create_new_linkable_tree() {
	new_test_ext().execute_with(|| {
		let max_edges = M as _;
		let depth = TREE_DEPTH as u8;
		assert_ok!(LinkableTree::create(Origin::root(), max_edges, depth));
		let id = MerkleTree::next_tree_id() - 1;
		let root = <LinkableTree as LinkableTreeInspector<_>>::get_root(id);
		assert_eq!(root.unwrap(), Element::from_bytes(&[0; 32]));
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
		let max_edges = M as _;
		let depth = TREE_DEPTH as u8;
		assert_ok!(LinkableTree::create(Origin::root(), max_edges, depth));
		let tree_id = MerkleTree::next_tree_id() - 1;
		let src_chain_id = 1;
		let root = Element::from_bytes(&[1u8; 32]);
		let height = 1;
		let neighbor_index_before: u32 =
			LinkableTree::curr_neighbor_root_index((tree_id, src_chain_id));
		assert_ok!(<LinkableTree as LinkableTreeInterface<_>>::add_edge(
			tree_id,
			src_chain_id,
			root,
			height,
		));
		let neighbor_index_after: u32 =
			LinkableTree::curr_neighbor_root_index((tree_id, src_chain_id));
		assert_eq!(neighbor_index_after, neighbor_index_before + 1);

		for _ in 0..(HistoryLength::get() - 1) {
			assert!(LinkableTree::is_known_neighbor_root(tree_id, src_chain_id, root).unwrap(),);

			let val = thread_rng().gen::<[u8; 32]>();
			let elt = Element::from_bytes(&val);
			assert_ok!(<LinkableTree as LinkableTreeInterface<_>>::update_edge(
				tree_id,
				src_chain_id,
				elt,
				height,
			));

			assert!(LinkableTree::is_known_neighbor_root(tree_id, src_chain_id, elt).unwrap(),);
		}

		assert!(!LinkableTree::is_known_neighbor_root(tree_id, src_chain_id, root).unwrap(),);
	});
}
