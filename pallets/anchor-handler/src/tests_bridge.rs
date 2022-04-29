use crate::{mock_bridge::*, types::UpdateRecord, AnchorList, Counts, UpdateRecords};
use frame_support::{assert_ok, traits::OnInitialize};
use pallet_bridge::types::{ProposalStatus, ProposalVotes};
use pallet_linkable_tree::types::EdgeMetadata;
use webb_primitives::utils::{
	compute_chain_id_type, derive_resource_id, get_typed_chain_id_in_u64,
};

const TEST_THRESHOLD: u32 = 2;
const TEST_MAX_EDGES: u32 = 100;
const TEST_TREE_DEPTH: u8 = 32;
const SUBSTRATE_CHAIN_TYPE: [u8; 2] = [2, 0];

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

fn setup_relayers(src_id: ChainId) {
	// set anchors threshold
	assert_ok!(Bridge::set_threshold(Origin::root(), TEST_THRESHOLD,));
	// add relayers
	assert_eq!(Bridge::relayer_count(), 0);
	assert_ok!(Bridge::add_relayer(Origin::root(), RELAYER_A));
	assert_ok!(Bridge::add_relayer(Origin::root(), RELAYER_B));
	assert_eq!(Bridge::relayer_count(), 2);
	// whitelish chain
	assert_ok!(Bridge::whitelist_chain(Origin::root(), src_id));
}
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

// helper function to add relayers and then make a proposal
fn relay_anchor_update_proposal(
	src_chain_id: ChainId,
	resource_id: &[u8; 32],
	prop_id: u64,
	edge_metadata: EdgeMetadata<ChainId, Element, <Test as pallet_mt::Config>::LeafIndex>,
) {
	// create anchor update proposal
	let resource = b"AnchorHandler.execute_anchor_update_proposal".to_vec();
	let update_proposal = make_anchor_update_proposal(resource_id, edge_metadata);
	// set resource id
	assert_ok!(Bridge::set_resource(Origin::root(), *resource_id, resource));
	// make proposals
	assert_ok!(Bridge::acknowledge_proposal(
		Origin::signed(RELAYER_A),
		prop_id,
		src_chain_id,
		*resource_id,
		Box::new(update_proposal.clone())
	));
	assert_ok!(Bridge::acknowledge_proposal(
		Origin::signed(RELAYER_B),
		prop_id,
		src_chain_id,
		*resource_id,
		Box::new(update_proposal)
	));
}

#[test]
// Test
// 1. Create an anchor using `pallet-anchor-handler` proposal through
// `pallet-bridge`
fn anchor_create_proposal() {
	new_test_ext().execute_with(|| {
		let params3 = arkworks_setups::common::setup_params::<ark_bn254::Fr>(arkworks_setups::Curve::Bn254, 5, 3);
		assert_ok!(HasherPallet::force_set_parameters(Origin::root(), params3.to_bytes()));

		let src_chain_id_u32 = 1u32;
		let resource_id = derive_resource_id(src_chain_id_u32, 1u32).into();
		let src_chain_id = get_typed_chain_id_in_u64(src_chain_id_u32);
		let prop_id = 1;
		setup_relayers(src_chain_id);
		// make anchor create proposal
		let deposit_size = 100;
		let create_proposal = make_anchor_create_proposal(deposit_size, src_chain_id, &resource_id);
		let resource = b"AnchorHandler.execute_anchor_create_proposal".to_vec();
		// set resource id
		assert_ok!(Bridge::set_resource(Origin::root(), resource_id, resource.clone()));
		assert_eq!(Some(resource), Bridge::resources(resource_id));

		// upon successful execution, Tree(with id=0) will be created in `pallet_mt`,
		// test Tree(with id=0) doesn't exist as of now
		assert!(!<pallet_mt::Trees<Test>>::contains_key(0));
		// make proposals
		assert_ok!(Bridge::acknowledge_proposal(
			Origin::signed(RELAYER_A),
			prop_id,
			src_chain_id,
			resource_id,
			Box::new(create_proposal.clone())
		));
		assert_ok!(Bridge::acknowledge_proposal(
			Origin::signed(RELAYER_B),
			prop_id,
			src_chain_id,
			resource_id,
			Box::new(create_proposal.clone())
		));
		// make sure `bridge_pallet` storage is expected
		let prop = Bridge::votes(src_chain_id, (prop_id, create_proposal)).unwrap();
		let expected = ProposalVotes {
			votes_for: vec![RELAYER_A, RELAYER_B],
			votes_against: vec![],
			status: ProposalStatus::Approved,
			expiry: ProposalLifetime::get() + 1,
		};
		assert_eq!(prop, expected);
		// make sureTest Tree(with id=0) exists in `pallet_mt` storage
		assert!(<pallet_mt::Trees<Test>>::contains_key(0));
		// proposal should be voted successfully
		// the anchor-handler callback must have been called by bridge
		// event must be emitted in callback should exist
		event_exists(crate::Event::AnchorCreated);
	})
}

#[test]
// Test
// 1. Create an anchor using `pallet-anchor` intrinsic call
// 2. Add an edge to the anchor using `pallet-anchor-handler` proposal through
// `pallet-bridge`
fn anchor_update_proposal_edge_add_success() {
	new_test_ext().execute_with(|| {
		let params3 = arkworks_setups::common::setup_params::<ark_bn254::Fr>(arkworks_setups::Curve::Bn254, 5, 3);
		assert_ok!(HasherPallet::force_set_parameters(Origin::root(), params3.to_bytes()));

		let src_chain_id_u32 = 1u32;
		let resource_id = derive_resource_id(src_chain_id_u32, 1).into();
		let src_chain_id = get_typed_chain_id_in_u64(src_chain_id_u32);
		let prop_id = 1;
		// create anchor update proposal
		setup_relayers(src_chain_id);
		mock_anchor_creation_using_pallet_call(src_chain_id, &resource_id);
		let root = Element::from_bytes(&[1; 32]);
		let latest_leaf_index = 5;
		let expected_tree_id = 0u32;
		let target = Element::from_bytes(&expected_tree_id.to_le_bytes());
		let edge_metadata = EdgeMetadata { src_chain_id, root, latest_leaf_index, target };
		assert_eq!(0, Counts::<Test>::get(src_chain_id));
		relay_anchor_update_proposal(src_chain_id, &resource_id, prop_id, edge_metadata.clone());
		assert_eq!(1, Counts::<Test>::get(src_chain_id));

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
			<pallet_linkable_tree::EdgeList<Test>>::get(expected_tree_id, src_chain_id)
		);

		let expected_update_record =
			UpdateRecord { tree_id: expected_tree_id, resource_id, edge_metadata };
		assert_eq!(expected_update_record, UpdateRecords::<Test>::get(src_chain_id, 0));
	})
}

#[test]
// Test
// 1. Create an anchor using `pallet-anchor` intrinsic call
// 2. Add an edge to the anchor using `pallet-anchor-handler` proposal through
// `pallet-bridge`
// 3. Update the edge of the anchor using
// `pallet-anchor-handler` proposal through `pallet-bridge`
fn anchor_update_proposal_edge_update_success() {
	new_test_ext().execute_with(|| {
		let params3 = arkworks_setups::common::setup_params::<ark_bn254::Fr>(arkworks_setups::Curve::Bn254, 5, 3);
		assert_ok!(HasherPallet::force_set_parameters(Origin::root(), params3.to_bytes()));

		let src_chain_id_u32 = 1u32;
		let resource_id = derive_resource_id(src_chain_id_u32, 1u32).into();
		let src_chain_id = get_typed_chain_id_in_u64(src_chain_id_u32);
		let prop_id = 1;
		setup_relayers(src_chain_id);
		mock_anchor_creation_using_pallet_call(src_chain_id, &resource_id);
		let root = Element::from_bytes(&[1; 32]);
		let latest_leaf_index = 5;
		let expected_tree_id = 0u32;
		let target = Element::from_bytes(&expected_tree_id.to_le_bytes());
		let edge_metadata = EdgeMetadata { src_chain_id, root, latest_leaf_index, target };
		assert_eq!(0, Counts::<Test>::get(src_chain_id));
		relay_anchor_update_proposal(src_chain_id, &resource_id, prop_id, edge_metadata.clone());
		assert_eq!(1, Counts::<Test>::get(src_chain_id));
		// proposal should have been voted successfully
		// the anchor-handler callback must have been called by bridge
		// event must be emitted in callback should exist
		event_exists(crate::Event::AnchorEdgeAdded);
		// follow
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
			<pallet_linkable_tree::EdgeList<Test>>::get(expected_tree_id, src_chain_id)
		);
		let expected_update_record =
			UpdateRecord { tree_id: expected_tree_id, resource_id, edge_metadata };
		assert_eq!(expected_update_record, UpdateRecords::<Test>::get(src_chain_id, 0));

		let root = Element::from_bytes(&[2; 32]);
		let latest_leaf_index = 10;
		let edge_metadata = EdgeMetadata { src_chain_id, root, latest_leaf_index, target };
		relay_anchor_update_proposal(
			src_chain_id,
			&resource_id,
			prop_id + 1,
			edge_metadata.clone(),
		);
		assert_eq!(2, Counts::<Test>::get(src_chain_id));
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
			<pallet_linkable_tree::EdgeList<Test>>::get(expected_tree_id, src_chain_id)
		);
		let expected_update_record =
			UpdateRecord { tree_id: expected_tree_id, resource_id, edge_metadata };
		assert_eq!(expected_update_record, UpdateRecords::<Test>::get(src_chain_id, 1));
	})
}
