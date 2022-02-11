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

use webb_primitives::{
	signing::SigningSystem,
	utils::{compute_chain_id_type, derive_resource_id},
	ResourceId,
};

const TEST_MAX_EDGES: u32 = 100;
const TEST_TREE_DEPTH: u8 = 32;

// helper function to create anchor using Anchor pallet call
fn mock_anchor_creation_using_pallet_call(src_chain_id: ChainId, resource_id: &[u8; 32]) {
	// upon successful anchor creation, Tree(with id=0) will be created in
	// `pallet_mt`, make sure Tree(with id=0) doesn't exist in `pallet_mt` storage
	assert!(!<pallet_mt::Trees<Test>>::contains_key(0));
	let deposit_size = 100;
	assert_ok!(Anchor::create(Origin::root(), deposit_size, TEST_MAX_EDGES, TEST_TREE_DEPTH, 0));
	// hack: insert an entry in AnchorsList with tree-id=0
	AnchorList::<Test>::insert(resource_id, 0);
	Counts::<Test>::insert(src_chain_id, 0);
	// make sure Tree(with id=0) exists in `pallet_mt` storage
	assert!(<pallet_mt::Trees<Test>>::contains_key(0));
	// check that anchor has stored `TEST_MAX_EDGES` correctly
	assert_eq!(TEST_MAX_EDGES, <pallet_linkable_tree::MaxEdges<Test>>::get(0));
}

fn make_anchor_create_proposal(
	deposit_size: Balance,
	src_chain_id: ChainId,
	resource_id: &[u8; 32],
) -> Call {
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

fn make_proposal_data(encoded_r_id: Vec<u8>, nonce: [u8; 4], encoded_call: Vec<u8>) -> Vec<u8> {
	let mut prop_data = encoded_r_id;
	prop_data.extend_from_slice(&[0u8; 4]);
	prop_data.extend_from_slice(&nonce);
	prop_data.extend_from_slice(&encoded_call[..]);
	prop_data
}

// Signature Bridge Tests

#[test]
fn should_create_anchor_with_sig_succeed() {
	let chain_type = [2, 0];
	let src_id = compute_chain_id_type(1u32, chain_type);
	let this_chain_id = compute_chain_id_type(5u32, chain_type);
	let r_id = derive_resource_id(this_chain_id, b"execute_anchor_create_proposal");
	let public_uncompressed = hex!("8db55b05db86c0b1786ca49f095d76344c9e6056b2f02701a7e7f3c20aabfd913ebbe148dd17c56551a52952371071a6c604b3f3abe8f2c8fa742158ea6dd7d4");
	let pair = ecdsa::Pair::from_string(
		"0x9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60",
		None,
	)
	.unwrap();

	new_test_ext_initialized(src_id, r_id, b"AnchorHandler.execute_anchor_create_proposal".to_vec())
		.execute_with(|| {
			let deposit_size = 100;
			let anchor_create_call = make_anchor_create_proposal(deposit_size, src_id, &r_id);
			let anchor_create_call_encoded = anchor_create_call.encode();
			let nonce = [0u8, 0u8, 0u8, 1u8];
			let prop_data = make_proposal_data(r_id.encode(), nonce, anchor_create_call_encoded);
			let msg = keccak_256(&prop_data);
			let sig: Signature = pair.sign_prehashed(&msg).into();
			// should fail to execute proposal as non-maintainer
			assert_err!(
				SignatureBridge::execute_proposal(
					Origin::signed(RELAYER_A),
					src_id,
					Box::new(anchor_create_call.clone()),
					prop_data.clone(),
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
				src_id,
				Box::new(anchor_create_call.clone()),
				prop_data.clone(),
				sig.0.to_vec(),
			));

			assert!(<pallet_mt::Trees<Test>>::contains_key(0));
			event_exists(crate::Event::AnchorCreated);
		})
}

// Test
// 1. Create an anchor using `pallet-anchor` intrinsic call
// 2. Add an edge to the anchor using `pallet-anchor-handler` proposal through
// `pallet-signature-bridge`
#[test]
fn should_add_anchor_edge_with_sig_succeed() {
	let chain_type = [2, 0];
	let src_id = compute_chain_id_type(1u32, chain_type);
	let this_chain_id = compute_chain_id_type(5u32, chain_type);
	let r_id = derive_resource_id(this_chain_id, b"execute_anchor_update_proposal");
	let public_uncompressed =
hex!("8db55b05db86c0b1786ca49f095d76344c9e6056b2f02701a7e7f3c20aabfd913ebbe148dd17c56551a52952371071a6c604b3f3abe8f2c8fa742158ea6dd7d4"
);
	let pair = ecdsa::Pair::from_string(
		"0x9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60",
		None,
	)
	.unwrap();

	new_test_ext_initialized(src_id, r_id, b"AnchorHandler.execute_anchor_update_proposal".to_vec())
		.execute_with(|| {
			let prop_id = 1;
			mock_anchor_creation_using_pallet_call(src_id, &r_id);

			let root = Element::from_bytes(&[1; 32]);
			let latest_leaf_index = 5;
			let edge_metadata = EdgeMetadata { src_chain_id: src_id, root, latest_leaf_index };
			assert_eq!(0, Counts::<Test>::get(src_id));

			let anchor_update_call = make_anchor_update_proposal(&r_id, edge_metadata.clone());
			let anchor_update_call_encoded = anchor_update_call.encode();
			let nonce = [0u8, 0u8, 0u8, 1u8];
			let prop_data = make_proposal_data(r_id.encode(), nonce, anchor_update_call_encoded);
			let msg = keccak_256(&prop_data);
			let sig: Signature = pair.sign_prehashed(&msg).into();
			// set the maintainer
			assert_ok!(SignatureBridge::force_set_maintainer(
				Origin::root(),
				public_uncompressed.to_vec()
			));

			assert_ok!(SignatureBridge::execute_proposal(
				Origin::signed(RELAYER_A),
				src_id,
				Box::new(anchor_update_call.clone()),
				prop_data,
				sig.0.to_vec(),
			));
			assert_eq!(1, Counts::<Test>::get(src_id));

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

			let expected_update_record =
				UpdateRecord { tree_id: expected_tree_id, resource_id: r_id, edge_metadata };
			assert_eq!(expected_update_record, UpdateRecords::<Test>::get(src_id, 0));
		})
}

// Test
// 1. Create an anchor using `pallet-anchor` intrinsic call
// 2. Add an edge to the anchor using `pallet-anchor-handler` proposal through
// `pallet-signature-bridge`
// 3. Update the edge of the anchor using
// `pallet-anchor-handler` proposal through `pallet-signature-bridge`
#[test]
fn should_update_anchor_edge_with_sig_succeed() {
	let chain_type = [2, 0];
	let src_id = compute_chain_id_type(1u32, chain_type);
	let this_chain_id = compute_chain_id_type(5u32, chain_type);
	let r_id = derive_resource_id(this_chain_id, b"execute_anchor_update_proposal");
	let public_uncompressed =
hex!("8db55b05db86c0b1786ca49f095d76344c9e6056b2f02701a7e7f3c20aabfd913ebbe148dd17c56551a52952371071a6c604b3f3abe8f2c8fa742158ea6dd7d4"
);
	let pair = ecdsa::Pair::from_string(
		"0x9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60",
		None,
	)
	.unwrap();

	new_test_ext_initialized(src_id, r_id, b"AnchorHandler.execute_anchor_update_proposal".to_vec())
		.execute_with(|| {
			mock_anchor_creation_using_pallet_call(src_id, &r_id);

			let root = Element::from_bytes(&[1; 32]);
			let latest_leaf_index = 5;
			let edge_metadata = EdgeMetadata { src_chain_id: src_id, root, latest_leaf_index };
			assert_eq!(0, Counts::<Test>::get(src_id));

			let anchor_update_call = make_anchor_update_proposal(&r_id, edge_metadata.clone());
			let anchor_update_call_encoded = anchor_update_call.encode();
			let nonce = [0u8, 0u8, 0u8, 1u8];
			let prop_data = make_proposal_data(r_id.encode(), nonce, anchor_update_call_encoded);
			let msg = keccak_256(&prop_data);
			let sig: Signature = pair.sign_prehashed(&msg).into();

			// set the maintainer
			assert_ok!(SignatureBridge::force_set_maintainer(
				Origin::root(),
				public_uncompressed.to_vec()
			));

			assert_ok!(SignatureBridge::execute_proposal(
				Origin::signed(RELAYER_A),
				src_id,
				Box::new(anchor_update_call.clone()),
				prop_data,
				sig.0.to_vec(),
			));
			assert_eq!(1, Counts::<Test>::get(src_id));

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

			let expected_update_record =
				UpdateRecord { tree_id: expected_tree_id, resource_id: r_id, edge_metadata };
			assert_eq!(expected_update_record, UpdateRecords::<Test>::get(src_id, 0));

			// Update Edge
			let root = Element::from_bytes(&[2; 32]);
			let latest_leaf_index = 10;
			let edge_metadata = EdgeMetadata { src_chain_id: src_id, root, latest_leaf_index };

			let anchor_update_call = make_anchor_update_proposal(&r_id, edge_metadata.clone());
			let anchor_update_call_encoded = anchor_update_call.encode();
			let nonce = [0u8, 0u8, 0u8, 2u8];
			let prop_data = make_proposal_data(r_id.encode(), nonce, anchor_update_call_encoded);
			let msg = keccak_256(&prop_data);
			let sig: Signature = pair.sign_prehashed(&msg).into();

			assert_ok!(SignatureBridge::execute_proposal(
				Origin::signed(RELAYER_A),
				src_id,
				Box::new(anchor_update_call.clone()),
				prop_data,
				sig.0.to_vec(),
			));

			assert_eq!(2, Counts::<Test>::get(src_id));

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
			let expected_update_record =
				UpdateRecord { tree_id: expected_tree_id, resource_id: r_id, edge_metadata };
			assert_eq!(expected_update_record, UpdateRecords::<Test>::get(src_id, 1));
		})
}

#[test]
fn should_fail_to_whitelist_chain_already_whitelisted() {
	let chain_type = [2, 0];
	let src_id = compute_chain_id_type(1u32, chain_type);
	let this_chain_id = compute_chain_id_type(1u32, chain_type);
	let r_id = derive_resource_id(this_chain_id, b"execute_anchor_create_proposal");
	let public_uncompressed =
hex!("8db55b05db86c0b1786ca49f095d76344c9e6056b2f02701a7e7f3c20aabfd913ebbe148dd17c56551a52952371071a6c604b3f3abe8f2c8fa742158ea6dd7d4"
);
	let pair = ecdsa::Pair::from_string(
		"0x9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60",
		None,
	)
	.unwrap();

	new_test_ext_initialized(src_id, r_id, b"AnchorHandler.execute_anchor_create_proposal".to_vec())
		.execute_with(|| {
			assert_err!(
				SignatureBridge::whitelist_chain(Origin::root(), src_id),
				pallet_signature_bridge::Error::<Test, _>::ChainAlreadyWhitelisted
			);
		})
}

#[test]
fn should_fail_to_whitelist_this_chain() {
	let chain_type = [2, 0];
	let src_id = compute_chain_id_type(1u32, chain_type);
	let this_chain_id = compute_chain_id_type(5u32, chain_type);
	let r_id = derive_resource_id(this_chain_id, b"execute_anchor_create_proposal");
	let public_uncompressed =
hex!("8db55b05db86c0b1786ca49f095d76344c9e6056b2f02701a7e7f3c20aabfd913ebbe148dd17c56551a52952371071a6c604b3f3abe8f2c8fa742158ea6dd7d4"
);
	let pair = ecdsa::Pair::from_string(
		"0x9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60",
		None,
	)
	.unwrap();

	new_test_ext_initialized(src_id, r_id, b"AnchorHandler.execute_anchor_create_proposal".to_vec())
		.execute_with(|| {
			assert_err!(
				SignatureBridge::whitelist_chain(
					Origin::root(),
					compute_chain_id_type(ChainIdentifier::get(), chain_type)
				),
				pallet_signature_bridge::Error::<Test, _>::InvalidChainId
			);
		})
}

#[test]
fn should_fail_to_execute_proposal_from_non_whitelisted_chain() {
	let chain_type = [2, 0];
	let src_id = compute_chain_id_type(1u32, chain_type);
	let this_chain_id = compute_chain_id_type(5u32, chain_type);
	let r_id = derive_resource_id(this_chain_id, b"execute_anchor_create_proposal");
	let public_uncompressed =
hex!("8db55b05db86c0b1786ca49f095d76344c9e6056b2f02701a7e7f3c20aabfd913ebbe148dd17c56551a52952371071a6c604b3f3abe8f2c8fa742158ea6dd7d4"
);
	let pair = ecdsa::Pair::from_string(
		"0x9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60",
		None,
	)
	.unwrap();

	new_test_ext_initialized(src_id, r_id, b"AnchorHandler.execute_anchor_create_proposal".to_vec())
		.execute_with(|| {
			let deposit_size = 100;
			let anchor_create_call = make_anchor_create_proposal(deposit_size, src_id, &r_id);
			let anchor_create_call_encoded = anchor_create_call.encode();
			let nonce = [0u8, 0u8, 0u8, 1u8];
			let prop_data = make_proposal_data(r_id.encode(), nonce, anchor_create_call_encoded);
			let msg = keccak_256(&prop_data);
			let sig: Signature = pair.sign_prehashed(&msg).into();
			// set the maintainer
			assert_ok!(SignatureBridge::force_set_maintainer(
				Origin::root(),
				public_uncompressed.to_vec()
			));
			assert!(!<pallet_mt::Trees<Test>>::contains_key(0));

			assert_err!(
				SignatureBridge::execute_proposal(
					Origin::signed(RELAYER_A),
					src_id + 1,
					Box::new(anchor_create_call.clone()),
					prop_data,
					sig.0.to_vec(),
				),
				pallet_signature_bridge::Error::<Test, _>::ChainNotWhitelisted
			);
		})
}

#[test]
fn should_fail_to_execute_proposal_with_non_existent_resource_id() {
	let chain_type = [2, 0];
	let src_id = compute_chain_id_type(1u32, chain_type);
	let this_chain_id = compute_chain_id_type(5u32, chain_type);
	let r_id = derive_resource_id(this_chain_id, b"execute_anchor_create_proposal");
	let public_uncompressed =
hex!("8db55b05db86c0b1786ca49f095d76344c9e6056b2f02701a7e7f3c20aabfd913ebbe148dd17c56551a52952371071a6c604b3f3abe8f2c8fa742158ea6dd7d4"
);
	let pair = ecdsa::Pair::from_string(
		"0x9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60",
		None,
	)
	.unwrap();

	new_test_ext_initialized(src_id, r_id, b"AnchorHandler.execute_anchor_create_proposal".to_vec())
		.execute_with(|| {
			let deposit_size = 100;
			let non_existent_r_id =
				derive_resource_id(this_chain_id, b"execute_anchor_crate_proposal");
			let anchor_create_call =
				make_anchor_create_proposal(deposit_size, src_id, &non_existent_r_id);
			let anchor_create_call_encoded = anchor_create_call.encode();
			let nonce = [0u8, 0u8, 0u8, 1u8];
			let prop_data =
				make_proposal_data(non_existent_r_id.encode(), nonce, anchor_create_call_encoded);
			let msg = keccak_256(&prop_data);
			let sig: Signature = pair.sign_prehashed(&msg).into();
			// set the maintainer
			assert_ok!(SignatureBridge::force_set_maintainer(
				Origin::root(),
				public_uncompressed.to_vec()
			));
			assert!(!<pallet_mt::Trees<Test>>::contains_key(0));

			assert_err!(
				SignatureBridge::execute_proposal(
					Origin::signed(RELAYER_A),
					src_id,
					Box::new(anchor_create_call.clone()),
					prop_data,
					sig.0.to_vec(),
				),
				pallet_signature_bridge::Error::<Test, _>::ResourceDoesNotExist
			);
		})
}

#[test]
fn should_fail_to_verify_proposal_with_tampered_signature() {
	let chain_type = [2, 0];
	let src_id = compute_chain_id_type(1u32, chain_type);
	let this_chain_id = compute_chain_id_type(5u32, chain_type);
	let r_id = derive_resource_id(this_chain_id, b"execute_anchor_create_proposal");
	let public_uncompressed =
hex!("8db55b05db86c0b1786ca49f095d76344c9e6056b2f02701a7e7f3c20aabfd913ebbe148dd17c56551a52952371071a6c604b3f3abe8f2c8fa742158ea6dd7d4"
);
	let pair = ecdsa::Pair::from_string(
		"0x9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60",
		None,
	)
	.unwrap();

	new_test_ext_initialized(src_id, r_id, b"AnchorHandler.execute_anchor_create_proposal".to_vec())
		.execute_with(|| {
			let deposit_size = 100;
			let anchor_create_call = make_anchor_create_proposal(deposit_size, src_id, &r_id);
			let anchor_create_call_encoded = anchor_create_call.encode();
			let nonce = [0u8, 0u8, 0u8, 1u8];
			let prop_data = make_proposal_data(r_id.encode(), nonce, anchor_create_call_encoded);
			let msg = keccak_256(&prop_data);
			let sig: Signature = pair.sign_prehashed(&msg).into();
			// set the maintainer
			assert_ok!(SignatureBridge::force_set_maintainer(
				Origin::root(),
				public_uncompressed.to_vec()
			));
			assert!(!<pallet_mt::Trees<Test>>::contains_key(0));
			let mut tampered_sig = sig.0.to_vec().clone();
			for x in &mut tampered_sig {
				*x += 1;
			}

			assert_err!(
				SignatureBridge::execute_proposal(
					Origin::signed(RELAYER_A),
					src_id,
					Box::new(anchor_create_call.clone()),
					prop_data,
					tampered_sig.clone(),
				),
				pallet_signature_bridge::Error::<Test, _>::InvalidPermissions
			);
		})
}
