use crate::{
	mock_signature_bridge::{new_test_ext_initialized, *},
	types::UpdateRecord,
	AnchorList, Counts, UpdateRecords,
};

use codec::{Decode, Encode, EncodeLike};
use frame_support::{assert_err, assert_ok};
use hex_literal::hex;
use pallet_linkable_tree::types::EdgeMetadata;
use sp_core::{
	ecdsa::{self, Signature},
	keccak_256, Pair, Public,
};

use pallet_signature_bridge::utils::derive_resource_id;
use webb_primitives::{signing::SigningSystem, ResourceId};

const TEST_THRESHOLD: u32 = 2;
const TEST_MAX_EDGES: u32 = 100;
const TEST_TREE_DEPTH: u8 = 32;

// helper function to create anchor using Anchor pallet call
fn mock_anchor_creation_using_pallet_call(src_chain_id: u32, resource_id: &[u8; 32]) {
	// upon successful anchor creation, Tree(with id=0) will be created in
	// `pallet_mt`, make sure Tree(with id=0) doesn't exist in `pallet_mt` storage
	assert!(!<pallet_mt::Trees<Test>>::contains_key(0));

	let deposit_size = 100;
	assert_ok!(Anchor::create(
		Origin::root(),
		deposit_size,
		TEST_MAX_EDGES,
		TEST_TREE_DEPTH,
		0
	));
	// hack: insert an entry in AnchorsList with tree-id=0
	AnchorList::<Test>::insert(resource_id, 0);
	Counts::<Test>::insert(src_chain_id, 0);
	// make sure Tree(with id=0) exists in `pallet_mt` storage
	assert!(<pallet_mt::Trees<Test>>::contains_key(0));
	// check that anchor has stored `TEST_MAX_EDGES` correctly
	assert_eq!(TEST_MAX_EDGES, <pallet_linkable_tree::MaxEdges<Test>>::get(0));
}

fn make_anchor_create_proposal(deposit_size: Balance, src_chain_id: u32, resource_id: &[u8; 32]) -> Call {
	Call::AnchorHandler(crate::Call::execute_anchor_create_proposal {
		deposit_size,
		src_chain_id,
		r_id: *resource_id,
		max_edges: TEST_MAX_EDGES,
		tree_depth: TEST_TREE_DEPTH,
		asset: NativeCurrencyId::get(),
	})
}

fn make_anchor_update_proposal(
	resource_id: &[u8; 32],
	anchor_metadata: EdgeMetadata<
		ChainId,
		<Test as pallet_mt::Config>::Element,
		<Test as pallet_mt::Config>::LeafIndex,
	>,
) -> Call {
	Call::AnchorHandler(crate::Call::execute_anchor_update_proposal {
		r_id: *resource_id,
		anchor_metadata,
	})
}

// Signature Bridge Tests

#[test]
fn should_create_anchor_with_sig_succeed() {
	let src_id = 1u32;
	let r_id = derive_resource_id(src_id, b"execute_anchor_create_proposal");
	let public_uncompressed = hex!("8db55b05db86c0b1786ca49f095d76344c9e6056b2f02701a7e7f3c20aabfd913ebbe148dd17c56551a52952371071a6c604b3f3abe8f2c8fa742158ea6dd7d4");
	let pair = ecdsa::Pair::from_string(
		"0x9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60",
		None,
	)
	.unwrap();

	new_test_ext_initialized(src_id, r_id, b"AnchorHandler.execute_anchor_create_proposal".to_vec()).execute_with(
		|| {
			let prop_id = 1;
			let deposit_size = 100;
			let proposal = make_anchor_create_proposal(deposit_size, src_id, &r_id);
			let msg = keccak_256(&proposal.encode());
			let sig: Signature = pair.sign_prehashed(&msg).into();
			// should fail to execute proposal as non-maintainer
			assert_err!(
				SignatureBridge::execute_proposal(
					Origin::signed(RELAYER_A),
					prop_id,
					src_id,
					r_id,
					Box::new(proposal.clone()),
					sig.0.to_vec(),
				),
				pallet_signature_bridge::Error::<Test, _>::InvalidPermissions
			);

			// set the maintainer
			assert_ok!(SignatureBridge::force_set_maintainer(
				Origin::root(),
				public_uncompressed.to_vec()
			));
			assert!(!<pallet_mt::Trees<Test>>::contains_key(0));

			assert_ok!(SignatureBridge::execute_proposal(
				Origin::signed(RELAYER_A),
				prop_id,
				src_id,
				r_id,
				Box::new(proposal.clone()),
				sig.0.to_vec(),
			));

			assert!(<pallet_mt::Trees<Test>>::contains_key(0));
			event_exists(crate::Event::AnchorCreated);
		},
	)
}

// Test
// 1. Create an anchor using `pallet-anchor` intrinsic call
// 2. Add an edge to the anchor using `pallet-anchor-handler` proposal through
// `pallet-signature-bridge`
#[test]
fn should_add_anchor_edge_succeed() {
	let src_id = 1u32;
	let r_id = derive_resource_id(src_id, b"execute_anchor_update_proposal");
	let public_uncompressed = hex!("8db55b05db86c0b1786ca49f095d76344c9e6056b2f02701a7e7f3c20aabfd913ebbe148dd17c56551a52952371071a6c604b3f3abe8f2c8fa742158ea6dd7d4");
	let pair = ecdsa::Pair::from_string(
		"0x9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60",
		None,
	)
	.unwrap();

	new_test_ext_initialized(src_id, r_id, b"AnchorHandler.execute_anchor_update_proposal".to_vec()).execute_with(
		|| {
			let prop_id = 1;
			mock_anchor_creation_using_pallet_call(src_id, &r_id);

			let root = Element::from_bytes(&[1; 32]);
			let latest_leaf_index = 5;
			let edge_metadata = EdgeMetadata {
				src_chain_id: src_id,
				root,
				latest_leaf_index,
			};
			assert_eq!(0, Counts::<Test>::get(src_id));

			let proposal = make_anchor_update_proposal(&r_id, edge_metadata.clone());
			let msg = keccak_256(&proposal.encode());
			let sig: Signature = pair.sign_prehashed(&msg).into();

			// set the maintainer
			assert_ok!(SignatureBridge::force_set_maintainer(
				Origin::root(),
				public_uncompressed.to_vec()
			));

			assert_ok!(SignatureBridge::execute_proposal(
				Origin::signed(RELAYER_A),
				prop_id,
				src_id,
				r_id,
				Box::new(proposal.clone()),
				sig.0.to_vec(),
			));
			assert_eq!(1, Counts::<Test>::get(src_id));

			// proposal should have been voted successfully
			// the anchor-handler callback must have been called by bridge
			// event must be emitted in callback should exist
			event_exists(crate::Event::AnchorEdgeAdded);
			// edge count should be 1
			assert_eq!(
				1,
				<pallet_linkable_tree::EdgeList<Test>>::iter_prefix_values(0)
					.into_iter()
					.count()
			);

			let expected_tree_id = 0;
			assert_eq!(
				edge_metadata,
				<pallet_linkable_tree::EdgeList<Test>>::get(expected_tree_id, src_id)
			);

			let expected_update_record = UpdateRecord {
				tree_id: expected_tree_id,
				resource_id: r_id,
				edge_metadata,
			};
			assert_eq!(expected_update_record, UpdateRecords::<Test>::get(src_id, 0));
		},
	)
}

// Test
// 1. Create an anchor using `pallet-anchor` intrinsic call
// 2. Add an edge to the anchor using `pallet-anchor-handler` proposal through
// `pallet-signature-bridge`
// 3. Update the edge of the anchor using
// `pallet-anchor-handler` proposal through `pallet-signature-bridge`
#[test]
fn should_update_anchor_edge_succeed() {
	let src_id = 1u32;
	let r_id = derive_resource_id(src_id, b"execute_anchor_update_proposal");
	let public_uncompressed = hex!("8db55b05db86c0b1786ca49f095d76344c9e6056b2f02701a7e7f3c20aabfd913ebbe148dd17c56551a52952371071a6c604b3f3abe8f2c8fa742158ea6dd7d4");
	let pair = ecdsa::Pair::from_string(
		"0x9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60",
		None,
	)
	.unwrap();

	new_test_ext_initialized(src_id, r_id, b"AnchorHandler.execute_anchor_update_proposal".to_vec()).execute_with(
		|| {
			let prop_id = 1;
			mock_anchor_creation_using_pallet_call(src_id, &r_id);

			let root = Element::from_bytes(&[1; 32]);
			let latest_leaf_index = 5;
			let edge_metadata = EdgeMetadata {
				src_chain_id: src_id,
				root,
				latest_leaf_index,
			};
			assert_eq!(0, Counts::<Test>::get(src_id));

			let proposal = make_anchor_update_proposal(&r_id, edge_metadata.clone());
			let msg = keccak_256(&proposal.encode());
			let sig: Signature = pair.sign_prehashed(&msg).into();

			// set the maintainer
			assert_ok!(SignatureBridge::force_set_maintainer(
				Origin::root(),
				public_uncompressed.to_vec()
			));

			assert_ok!(SignatureBridge::execute_proposal(
				Origin::signed(RELAYER_A),
				prop_id,
				src_id,
				r_id,
				Box::new(proposal.clone()),
				sig.0.to_vec(),
			));
			assert_eq!(1, Counts::<Test>::get(src_id));

			// proposal should have been voted successfully
			// the anchor-handler callback must have been called by bridge
			// event must be emitted in callback should exist
			event_exists(crate::Event::AnchorEdgeAdded);
			// edge count should be 1
			assert_eq!(
				1,
				<pallet_linkable_tree::EdgeList<Test>>::iter_prefix_values(0)
					.into_iter()
					.count()
			);

			let expected_tree_id = 0;
			assert_eq!(
				edge_metadata,
				<pallet_linkable_tree::EdgeList<Test>>::get(expected_tree_id, src_id)
			);

			let expected_update_record = UpdateRecord {
				tree_id: expected_tree_id,
				resource_id: r_id,
				edge_metadata,
			};
			assert_eq!(expected_update_record, UpdateRecords::<Test>::get(src_id, 0));

			// Update Edge
			let root = Element::from_bytes(&[2; 32]);
			let latest_leaf_index = 10;
			let edge_metadata = EdgeMetadata {
				src_chain_id: src_id,
				root,
				latest_leaf_index,
			};

			let proposal = make_anchor_update_proposal(&r_id, edge_metadata.clone());
			let msg = keccak_256(&proposal.encode());
			let sig: Signature = pair.sign_prehashed(&msg).into();

			assert_ok!(SignatureBridge::execute_proposal(
				Origin::signed(RELAYER_A),
				prop_id + 1,
				src_id,
				r_id,
				Box::new(proposal.clone()),
				sig.0.to_vec(),
			));

			assert_eq!(2, Counts::<Test>::get(src_id));
			// proposal should have been voted successfully
			// the anchor-handler callback must have been called by bridge
			// event must be emitted in callback should exist
			event_exists(crate::Event::AnchorEdgeUpdated);
			// edge count should be 1
			assert_eq!(
				1,
				<pallet_linkable_tree::EdgeList<Test>>::iter_prefix_values(0)
					.into_iter()
					.count()
			);
			assert_eq!(
				edge_metadata,
				<pallet_linkable_tree::EdgeList<Test>>::get(expected_tree_id, src_id)
			);
			let expected_update_record = UpdateRecord {
				tree_id: expected_tree_id,
				resource_id: r_id,
				edge_metadata,
			};
			assert_eq!(expected_update_record, UpdateRecords::<Test>::get(src_id, 1));
		},
	)
}
