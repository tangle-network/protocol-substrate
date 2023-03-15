use crate::{
	mock_signature_bridge::{new_test_ext_initialized, *},
	AnchorList,
};

use arkworks_setups::{common::setup_params, Curve};
use codec::Encode;
use frame_support::{assert_err, assert_ok};
use hex_literal::hex;
use pallet_linkable_tree::types::EdgeMetadata;
use sp_core::{
	ecdsa::{self, Signature},
	keccak_256, Pair,
};
use sp_std::convert::TryInto;

use webb_proposals::{
	FunctionSignature, ResourceId, SubstrateTargetSystem, TargetSystem, TypedChainId,
};

const TEST_MAX_EDGES: u32 = 100;
const TEST_TREE_DEPTH: u8 = 32;

const ANCHOR_CREATE_FUNCTION_SIG: FunctionSignature = FunctionSignature::new(0u32.to_be_bytes());
const ANCHOR_UPDATE_FUNCTION_SIG: FunctionSignature = FunctionSignature::new(1u32.to_be_bytes());
const SET_RESOURCE_FUNCTION_SIG: FunctionSignature = FunctionSignature::new(2u32.to_be_bytes());

fn make_set_resource_proposal(
	header: webb_proposals::ProposalHeader,
	new_resource: webb_proposals::ResourceId,
) -> Vec<u8> {
	let set_resource_proposal = webb_proposals::substrate::ResourceIdUpdateProposal::builder()
		.header(header)
		.new_resource_id(new_resource)
		.build();
	set_resource_proposal.to_bytes()
}

fn get_edsca_account() -> ecdsa::Pair {
	let seed = "0x9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60";
	ecdsa::Pair::from_string(seed, None).unwrap()
}

fn get_public_uncompressed_key() -> [u8; 64] {
	hex!("8db55b05db86c0b1786ca49f095d76344c9e6056b2f02701a7e7f3c20aabfd913ebbe148dd17c56551a52952371071a6c604b3f3abe8f2c8fa742158ea6dd7d4")
}

fn make_proposal_header(
	resource_id: webb_proposals::ResourceId,
	function_signature: webb_proposals::FunctionSignature,
	nonce: webb_proposals::Nonce,
) -> webb_proposals::ProposalHeader {
	webb_proposals::ProposalHeader::new(resource_id, function_signature, nonce)
}

// helper function to create anchor using Anchor pallet call
fn mock_vanchor_creation_using_pallet_call(resource_id: &ResourceId) {
	// upon successful anchor creation, Tree(with id=0) will be created in
	// `pallet_mt`, make sure Tree(with id=0) doesn't exist in `pallet_mt` storage
	assert!(!<pallet_mt::Trees<Test>>::contains_key(0));
	assert_ok!(VAnchor::create(RuntimeOrigin::root(), TEST_MAX_EDGES, TEST_TREE_DEPTH, 0));
	// hack: insert an entry in AnchorsList with tree-id=0
	AnchorList::<Test>::insert(resource_id, 0);
	// make sure Tree(with id=0) exists in `pallet_mt` storage
	assert!(<pallet_mt::Trees<Test>>::contains_key(0));
	// check that anchor has stored `TEST_MAX_EDGES` correctly
	assert_eq!(TEST_MAX_EDGES, <pallet_linkable_tree::MaxEdges<Test>>::get(0));
}

fn make_vanchor_create_proposal(
	src_chain_id: ChainId,
	resource_id: &ResourceId,
	nonce: u32,
) -> RuntimeCall {
	RuntimeCall::VAnchorHandler(crate::Call::execute_vanchor_create_proposal {
		src_chain_id,
		r_id: *resource_id,
		max_edges: TEST_MAX_EDGES,
		tree_depth: TEST_TREE_DEPTH,
		asset: NativeCurrencyId::get(),
		nonce,
	})
}

fn make_vanchor_update_proposal(
	resource_id: &ResourceId,
	merkle_root: Element,
	src_resource_id: ResourceId,
	nonce: u32,
) -> RuntimeCall {
	RuntimeCall::VAnchorHandler(crate::Call::execute_vanchor_update_proposal {
		r_id: *resource_id,
		merkle_root,
		src_resource_id,
		nonce,
	})
}

fn make_proposal_data(
	encoded_r_id: Vec<u8>,
	function_signature: FunctionSignature,
	nonce: [u8; 4],
	encoded_call: Vec<u8>,
) -> Vec<u8> {
	let mut prop_data = encoded_r_id;
	prop_data.extend_from_slice(&function_signature.0);
	prop_data.extend_from_slice(&nonce);
	prop_data.extend_from_slice(&encoded_call[..]);
	prop_data
}

// Signature Bridge Tests
#[test]
fn should_create_vanchor_with_sig_succeed() {
	let src_id = TypedChainId::Substrate(1);
	let target_id = TypedChainId::Substrate(5);
	let target_system =
		TargetSystem::Substrate(SubstrateTargetSystem { pallet_index: 11, tree_id: 5 });
	let r_id: ResourceId = ResourceId::new(target_system, target_id);

	let public_uncompressed = get_public_uncompressed_key();
	let pair = get_edsca_account();

	new_test_ext_initialized(
		src_id.chain_id(),
		r_id,
		b"VAnchorHandler.execute_vanchor_create_proposal".to_vec(),
	)
	.execute_with(|| {
		let curve = Curve::Bn254;
		let params = setup_params::<ark_bn254::Fr>(curve, 5, 3);
		let _ = HasherPallet::force_set_parameters(
			RuntimeOrigin::root(),
			params.to_bytes().try_into().unwrap(),
		);
		let nonce = 1;
		let anchor_create_call = make_vanchor_create_proposal(src_id.chain_id(), &r_id, nonce);
		let anchor_create_call_encoded = anchor_create_call.encode();
		let nonce = [0u8, 0u8, 0u8, 1u8];
		let prop_data = make_proposal_data(
			r_id.encode(),
			ANCHOR_CREATE_FUNCTION_SIG,
			nonce,
			anchor_create_call_encoded,
		);
		let msg = keccak_256(&prop_data);
		let sig: Signature = pair.sign_prehashed(&msg);
		// should fail to execute proposal as non-maintainer
		assert_err!(
			SignatureBridge::execute_proposal(
				RuntimeOrigin::signed(RELAYER_A),
				src_id.chain_id(),
				prop_data.clone().try_into().unwrap(),
				sig.0.to_vec().try_into().unwrap(),
			),
			pallet_signature_bridge::Error::<Test, _>::InvalidPermissions
		);
		// set the maintainer
		assert_ok!(SignatureBridge::force_set_maintainer(
			RuntimeOrigin::root(),
			1u32,
			public_uncompressed.to_vec().try_into().unwrap()
		));
		assert!(!<pallet_mt::Trees<Test>>::contains_key(0));

		assert_ok!(SignatureBridge::execute_proposal(
			RuntimeOrigin::signed(RELAYER_A),
			src_id.chain_id(),
			prop_data.try_into().unwrap(),
			sig.0.to_vec().try_into().unwrap(),
		));

		assert!(<pallet_mt::Trees<Test>>::contains_key(0));
		event_exists(crate::Event::AnchorCreated);
	})
}

// Test
// 1. Create an anchor using `pallet-vanchor` intrinsic call
// 2. Add an edge to the anchor using `pallet-vanchor-handler` proposal through
// `pallet-signature-bridge`
#[test]
fn should_add_vanchor_edge_with_sig_succeed() {
	let src_id = TypedChainId::Substrate(1);
	let target_id = TypedChainId::Substrate(5);
	let target_system =
		TargetSystem::Substrate(SubstrateTargetSystem { pallet_index: 11, tree_id: 0 });
	let r_id: ResourceId = ResourceId::new(target_system, target_id);
	let public_uncompressed = get_public_uncompressed_key();
	let pair = get_edsca_account();

	new_test_ext_initialized(
		src_id.chain_id(),
		r_id,
		b"VAnchorHandler.execute_vanchor_update_proposal".to_vec().try_into().unwrap(),
	)
	.execute_with(|| {
		let curve = Curve::Bn254;
		let params = setup_params::<ark_bn254::Fr>(curve, 5, 3);
		let _ = HasherPallet::force_set_parameters(
			RuntimeOrigin::root(),
			params.to_bytes().try_into().unwrap(),
		);

		mock_vanchor_creation_using_pallet_call(&r_id);

		let root = Element::from_bytes(&[1; 32]);
		let latest_leaf_index = 5;
		let src_target_system = target_system;
		let src_resource_id = ResourceId::new(src_target_system, src_id);
		let anchor_update_call =
			make_vanchor_update_proposal(&r_id, root, src_resource_id, latest_leaf_index);
		let anchor_update_call_encoded = anchor_update_call.encode();
		let prop_data = make_proposal_data(
			r_id.encode(),
			ANCHOR_UPDATE_FUNCTION_SIG,
			latest_leaf_index.to_be_bytes(),
			anchor_update_call_encoded,
		);
		let msg = keccak_256(&prop_data);
		let sig: Signature = pair.sign_prehashed(&msg);
		// set the maintainer
		assert_ok!(SignatureBridge::force_set_maintainer(
			RuntimeOrigin::root(),
			1u32,
			public_uncompressed.to_vec().try_into().unwrap()
		));

		assert_ok!(SignatureBridge::execute_proposal(
			RuntimeOrigin::signed(RELAYER_A),
			src_id.chain_id(),
			prop_data.try_into().unwrap(),
			sig.0.to_vec().try_into().unwrap(),
		));
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
			EdgeMetadata {
				src_chain_id: src_id.chain_id(),
				root,
				latest_leaf_index,
				src_resource_id
			},
			<pallet_linkable_tree::EdgeList<Test>>::get(expected_tree_id, src_id.chain_id())
		);
	})
}

// Test
// 1. Create an anchor using `pallet-vanchor` intrinsic call
// 2. Add an edge to the anchor using `pallet-vanchor-handler` proposal through
// `pallet-signature-bridge`
// 3. Update the edge of the anchor using
// `pallet-vanchor-handler` proposal through `pallet-signature-bridge`
#[test]
fn should_update_vanchor_edge_with_sig_succeed() {
	let src_id = TypedChainId::Substrate(1);
	let target_id = TypedChainId::Substrate(5);
	let target_system =
		TargetSystem::Substrate(SubstrateTargetSystem { pallet_index: 11, tree_id: 0 });
	let r_id: ResourceId = ResourceId::new(target_system, target_id);
	let public_uncompressed = get_public_uncompressed_key();
	let pair = get_edsca_account();

	new_test_ext_initialized(
		src_id.chain_id(),
		r_id,
		b"VAnchorHandler.execute_vanchor_update_proposal".to_vec(),
	)
	.execute_with(|| {
		let curve = Curve::Bn254;
		let params = setup_params::<ark_bn254::Fr>(curve, 5, 3);
		let _ = HasherPallet::force_set_parameters(
			RuntimeOrigin::root(),
			params.to_bytes().try_into().unwrap(),
		);
		println!("here");
		mock_vanchor_creation_using_pallet_call(&r_id);
		println!("there");
		let root = Element::from_bytes(&[1; 32]);
		let latest_leaf_index = 5;
		let src_target_system = target_system;
		let src_resource_id = ResourceId::new(src_target_system, src_id);
		let anchor_update_call =
			make_vanchor_update_proposal(&r_id, root, src_resource_id, latest_leaf_index);
		let anchor_update_call_encoded = anchor_update_call.encode();
		let prop_data = make_proposal_data(
			r_id.encode(),
			ANCHOR_UPDATE_FUNCTION_SIG,
			latest_leaf_index.to_be_bytes(),
			anchor_update_call_encoded,
		);
		let msg = keccak_256(&prop_data);
		let sig: Signature = pair.sign_prehashed(&msg);

		// set the maintainer
		assert_ok!(SignatureBridge::force_set_maintainer(
			RuntimeOrigin::root(),
			1u32,
			public_uncompressed.to_vec().try_into().unwrap()
		));

		assert_ok!(SignatureBridge::execute_proposal(
			RuntimeOrigin::signed(RELAYER_A),
			src_id.chain_id(),
			prop_data.try_into().unwrap(),
			sig.0.to_vec().try_into().unwrap(),
		));
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
			EdgeMetadata {
				src_chain_id: src_id.chain_id(),
				root,
				latest_leaf_index,
				src_resource_id
			},
			<pallet_linkable_tree::EdgeList<Test>>::get(expected_tree_id, src_id.chain_id())
		);

		// Update Edge
		let root = Element::from_bytes(&[2; 32]);
		let latest_leaf_index = 10;
		let src_target_system = target_system;
		let src_resource_id = ResourceId::new(src_target_system, src_id);
		let anchor_update_call =
			make_vanchor_update_proposal(&r_id, root, src_resource_id, latest_leaf_index);
		let anchor_update_call_encoded = anchor_update_call.encode();
		let prop_data = make_proposal_data(
			r_id.encode(),
			ANCHOR_UPDATE_FUNCTION_SIG,
			latest_leaf_index.to_be_bytes(),
			anchor_update_call_encoded,
		);
		let msg = keccak_256(&prop_data);
		let sig: Signature = pair.sign_prehashed(&msg);

		assert_ok!(SignatureBridge::execute_proposal(
			RuntimeOrigin::signed(RELAYER_A),
			src_id.chain_id(),
			prop_data.try_into().unwrap(),
			sig.0.to_vec().try_into().unwrap(),
		));
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
			EdgeMetadata {
				src_chain_id: src_id.chain_id(),
				root,
				latest_leaf_index,
				src_resource_id
			},
			<pallet_linkable_tree::EdgeList<Test>>::get(expected_tree_id, src_id.chain_id())
		);
	})
}

#[test]
fn should_fail_to_whitelist_chain_already_whitelisted() {
	let src_id = TypedChainId::Substrate(1);
	let target_id = TypedChainId::Substrate(5);
	let target_system =
		TargetSystem::Substrate(SubstrateTargetSystem { pallet_index: 11, tree_id: 5 });
	let r_id: ResourceId = ResourceId::new(target_system, target_id);

	new_test_ext_initialized(
		src_id.chain_id(),
		r_id,
		b"VAnchorHandler.execute_vanchor_create_proposal".to_vec(),
	)
	.execute_with(|| {
		assert_err!(
			SignatureBridge::whitelist_chain(RuntimeOrigin::root(), src_id.chain_id()),
			pallet_signature_bridge::Error::<Test, _>::ChainAlreadyWhitelisted
		);
	})
}

#[test]
fn should_fail_to_execute_proposal_from_non_whitelisted_chain() {
	let src_id = TypedChainId::Substrate(1);
	let target_id = TypedChainId::Substrate(5);
	let target_system =
		TargetSystem::Substrate(SubstrateTargetSystem { pallet_index: 11, tree_id: 5 });
	let r_id: ResourceId = ResourceId::new(target_system, target_id);

	let public_uncompressed = get_public_uncompressed_key();
	let pair = get_edsca_account();
	new_test_ext_initialized(
		src_id.chain_id(),
		r_id,
		b"VAnchorHandler.execute_vanchor_create_proposal".to_vec(),
	)
	.execute_with(|| {
		let latest_leaf_index = 1;
		let anchor_create_call =
			make_vanchor_create_proposal(src_id.chain_id(), &r_id, latest_leaf_index);
		let anchor_create_call_encoded = anchor_create_call.encode();
		let prop_data = make_proposal_data(
			r_id.encode(),
			ANCHOR_CREATE_FUNCTION_SIG,
			latest_leaf_index.to_be_bytes(),
			anchor_create_call_encoded,
		);
		let msg = keccak_256(&prop_data);
		let sig: Signature = pair.sign_prehashed(&msg);
		// set the maintainer
		assert_ok!(SignatureBridge::force_set_maintainer(
			RuntimeOrigin::root(),
			1u32,
			public_uncompressed.to_vec().try_into().unwrap()
		));
		assert!(!<pallet_mt::Trees<Test>>::contains_key(0));

		assert_err!(
			SignatureBridge::execute_proposal(
				RuntimeOrigin::signed(RELAYER_A),
				src_id.chain_id() + 1,
				prop_data.try_into().unwrap(),
				sig.0.to_vec().try_into().unwrap(),
			),
			pallet_signature_bridge::Error::<Test, _>::ChainNotWhitelisted
		);
	})
}

#[test]
fn should_fail_to_execute_proposal_with_non_existent_resource_id() {
	let src_id = TypedChainId::Substrate(1);
	let target_id = TypedChainId::Substrate(5);
	let target_system =
		TargetSystem::Substrate(SubstrateTargetSystem { pallet_index: 11, tree_id: 5 });
	let r_id: ResourceId = ResourceId::new(target_system, target_id);

	let public_uncompressed = get_public_uncompressed_key();
	let pair = get_edsca_account();

	new_test_ext_initialized(
		src_id.chain_id(),
		r_id,
		b"VAnchorHandler.execute_vanchor_create_proposal".to_vec(),
	)
	.execute_with(|| {
		let nonce = 1;

		let non_existent_r_id = ResourceId::new(
			TargetSystem::Substrate(SubstrateTargetSystem { pallet_index: 11, tree_id: 5 }),
			TypedChainId::Substrate(500),
		);
		let anchor_create_call =
			make_vanchor_create_proposal(src_id.chain_id(), &non_existent_r_id, nonce);
		let anchor_create_call_encoded = anchor_create_call.encode();
		let nonce = [0u8, 0u8, 0u8, 1u8];
		let prop_data = make_proposal_data(
			non_existent_r_id.encode(),
			ANCHOR_CREATE_FUNCTION_SIG,
			nonce,
			anchor_create_call_encoded,
		);
		let msg = keccak_256(&prop_data);
		let sig: Signature = pair.sign_prehashed(&msg);
		// set the maintainer
		assert_ok!(SignatureBridge::force_set_maintainer(
			RuntimeOrigin::root(),
			1u32,
			public_uncompressed.to_vec().try_into().unwrap()
		));
		assert!(!<pallet_mt::Trees<Test>>::contains_key(0));

		assert_err!(
			SignatureBridge::execute_proposal(
				RuntimeOrigin::signed(RELAYER_A),
				src_id.chain_id(),
				prop_data.try_into().unwrap(),
				sig.0.to_vec().try_into().unwrap(),
			),
			pallet_signature_bridge::Error::<Test, _>::ResourceDoesNotExist
		);
	})
}

#[test]
fn should_fail_to_verify_proposal_with_tampered_signature() {
	let src_id = TypedChainId::Substrate(1);
	let target_id = TypedChainId::Substrate(5);
	let target_system =
		TargetSystem::Substrate(SubstrateTargetSystem { pallet_index: 11, tree_id: 5 });
	let r_id: ResourceId = ResourceId::new(target_system, target_id);

	let public_uncompressed = get_public_uncompressed_key();
	let pair = get_edsca_account();

	new_test_ext_initialized(
		src_id.chain_id(),
		r_id,
		b"VAnchorHandler.execute_vanchor_create_proposal".to_vec(),
	)
	.execute_with(|| {
		let nonce = 1;
		let anchor_create_call = make_vanchor_create_proposal(src_id.chain_id(), &r_id, nonce);
		let anchor_create_call_encoded = anchor_create_call.encode();
		let nonce = [0u8, 0u8, 0u8, 1u8];
		let prop_data = make_proposal_data(
			r_id.encode(),
			ANCHOR_CREATE_FUNCTION_SIG,
			nonce,
			anchor_create_call_encoded,
		);
		let msg = keccak_256(&prop_data);
		let sig: Signature = pair.sign_prehashed(&msg);
		// set the maintainer
		assert_ok!(SignatureBridge::force_set_maintainer(
			RuntimeOrigin::root(),
			1u32,
			public_uncompressed.to_vec().try_into().unwrap()
		));
		assert!(!<pallet_mt::Trees<Test>>::contains_key(0));
		let mut tampered_sig = sig.0.to_vec();
		for x in &mut tampered_sig[2..5] {
			*x += 1;
		}

		assert_err!(
			SignatureBridge::execute_proposal(
				RuntimeOrigin::signed(RELAYER_A),
				src_id.chain_id(),
				prop_data.try_into().unwrap(),
				tampered_sig.clone().try_into().unwrap(),
			),
			pallet_signature_bridge::Error::<Test, _>::InvalidPermissions
		);
	})
}

// Test ResourceIdProposal
#[test]
fn should_add_resource_sig_succeed_using_webb_proposals() {
	let target_system = webb_proposals::TargetSystem::Substrate(SubstrateTargetSystem {
		pallet_index: 10,
		tree_id: 5,
	});
	let this_chain_id = webb_proposals::TypedChainId::Substrate(5);
	let resource = webb_proposals::ResourceId::new(target_system, this_chain_id);
	let src_id = webb_proposals::TypedChainId::Substrate(1);
	let public_uncompressed = get_public_uncompressed_key();
	let pair = get_edsca_account();

	new_test_ext_for_set_resource_proposal_initialized(src_id.chain_id()).execute_with(|| {
		let curve = Curve::Bn254;
		let params = setup_params::<ark_bn254::Fr>(curve, 5, 3);
		let _ = HasherPallet::force_set_parameters(
			RuntimeOrigin::root(),
			params.to_bytes().try_into().unwrap(),
		);
		let nonce = webb_proposals::Nonce::from(0x0001);
		let header = make_proposal_header(resource, SET_RESOURCE_FUNCTION_SIG, nonce);
		//create anchor
		assert_ok!(VAnchor::create(RuntimeOrigin::root(), TEST_MAX_EDGES, TEST_TREE_DEPTH, 0));
		// Anchorlist should be 0 and will be updated after exectuing set resource proposal
		assert_eq!(0, AnchorList::<Test>::iter_keys().count());

		// make set resource proposal
		let set_resource_proposal_bytes = make_set_resource_proposal(header, resource);

		let msg = keccak_256(&set_resource_proposal_bytes);
		let sig: Signature = pair.sign_prehashed(&msg);

		// set the maintainer
		assert_ok!(SignatureBridge::force_set_maintainer(
			RuntimeOrigin::root(),
			1u32,
			public_uncompressed.to_vec().try_into().unwrap(),
		));

		assert_ok!(SignatureBridge::set_resource_with_signature(
			RuntimeOrigin::signed(RELAYER_A),
			src_id.chain_id(),
			set_resource_proposal_bytes.try_into().unwrap(),
			sig.0.to_vec().try_into().unwrap(),
		));

		// the anchor-handler callback must have been called by bridge
		// event must be emitted in callback should exist
		event_exists(crate::Event::ResourceAnchored);
		// edge count should be 1
		assert_eq!(1, AnchorList::<Test>::iter_keys().count());
	})
}
