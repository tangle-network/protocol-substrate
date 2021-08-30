use crate::{mock::*, ChainId};
use frame_support::{assert_err, assert_noop, assert_ok, dispatch::DispatchError};
use pallet_anchor::types::EdgeMetadata;
use pallet_bridge::types::{ProposalStatus, ProposalVotes};
use pallet_mt::types::ElementTrait;

const TEST_THRESHOLD: u32 = 2;
const TEST_MAX_EDGES: u32 = 100;
const TEST_TREE_DEPTH: u8 = 32;

fn make_anchor_create_proposal() -> Call {
	let resource_id = [1; 32];
	Call::AnchorHandler(crate::Call::execute_anchor_create_proposal(
		resource_id,
		TEST_MAX_EDGES,
		TEST_TREE_DEPTH,
	))
}

fn make_anchor_update_proposal(
	anchor_metadata: EdgeMetadata<
		ChainId<Test>,
		<Test as pallet_mt::Config>::Element,
		<Test as frame_system::Config>::BlockNumber,
	>,
) -> Call {
	let resource_id = [1; 32];
	Call::AnchorHandler(crate::Call::execute_anchor_update_proposal(
		resource_id,
		anchor_metadata,
	))
}

fn setup_relayers(src_id: u32) {
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
fn create_anchor_using_pallet_call() {
	// upon successful anchor creation, Tree(with id=0) will be created in
	// `pallet_mt`, make sure Tree(with id=0) doesn't exist in `pallet_mt` storage
	assert_eq!(false, <pallet_mt::Trees<Test>>::contains_key(0));
	assert_ok!(Anchor::create(Origin::root(), TEST_MAX_EDGES, TEST_TREE_DEPTH));
	// make sure Tree(with id=0) exists in `pallet_mt` storage
	assert_eq!(true, <pallet_mt::Trees<Test>>::contains_key(0));
	// check that anchor has stored `TEST_MAX_EDGES` correctly
	assert_eq!(TEST_MAX_EDGES, <pallet_anchor::MaxEdges<Test>>::get(0));
}

// helper function to add relayers and then make a proposal
fn relay_anchor_update_proposal(
	src_id: u32,
	prop_id: u64,
	edge_metadata: EdgeMetadata<ChainId<Test>, Element, <Test as frame_system::Config>::BlockNumber>,
) {
	let r_id = pallet_bridge::utils::derive_resource_id(src_id, b"hash");
	// create anchor update proposal
	let resource = b"AnchorHandler.execute_anchor_update_proposal".to_vec();
	let update_proposal = make_anchor_update_proposal(edge_metadata.clone());
	// set resource id
	assert_ok!(Bridge::set_resource(Origin::root(), r_id, resource));
	// make proposals
	assert_ok!(Bridge::acknowledge_proposal(
		Origin::signed(RELAYER_A),
		prop_id,
		src_id,
		r_id,
		Box::new(update_proposal.clone())
	));
	assert_ok!(Bridge::acknowledge_proposal(
		Origin::signed(RELAYER_B),
		prop_id,
		src_id,
		r_id,
		Box::new(update_proposal.clone())
	));
}

#[test]
// Test
// 1. Create an anchor using `pallet-anchor-handler` proposal through
// `pallet-bridge`
fn anchor_create_proposal() {
	new_test_ext().execute_with(|| {
		let src_id = 1;
		setup_relayers(src_id);
		// make anchor create proposal
		let create_proposal = make_anchor_create_proposal();
		let prop_id = 1;
		let r_id = pallet_bridge::utils::derive_resource_id(src_id, b"hash");
		let resource = b"AnchorHandler.execute_anchor_create_proposal".to_vec();
		// set resource id
		assert_ok!(Bridge::set_resource(Origin::root(), r_id, resource.clone()));
		assert_eq!(Some(resource), Bridge::resources(r_id));

		// upon successful execution, Tree(with id=0) will be created in `pallet_mt`,
		// test Tree(with id=0) doesn't exist as of now
		assert_eq!(false, <pallet_mt::Trees<Test>>::contains_key(0));
		// make proposals
		assert_ok!(Bridge::acknowledge_proposal(
			Origin::signed(RELAYER_A),
			prop_id,
			src_id,
			r_id,
			Box::new(create_proposal.clone())
		));
		assert_ok!(Bridge::acknowledge_proposal(
			Origin::signed(RELAYER_B),
			prop_id,
			src_id,
			r_id,
			Box::new(create_proposal.clone())
		));
		// make sure `bridge_pallet` storage is expected
		let prop = Bridge::votes(src_id, (prop_id.clone(), create_proposal.clone())).unwrap();
		let expected = ProposalVotes {
			votes_for: vec![RELAYER_A, RELAYER_B],
			votes_against: vec![],
			status: ProposalStatus::Approved,
			expiry: ProposalLifetime::get() + 1,
		};
		assert_eq!(prop, expected);
		// make sureTest Tree(with id=0) exists in `pallet_mt` storage
		assert_eq!(true, <pallet_mt::Trees<Test>>::contains_key(0));
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
		let src_chain_id = 1;
		setup_relayers(src_chain_id);
		create_anchor_using_pallet_call();
		let root = Element::from_bytes(&[1; 32]);
		let height: u64 = 5;
		let edge_metadata = EdgeMetadata {
			src_chain_id,
			root,
			height,
		};
		relay_anchor_update_proposal(1, 1, edge_metadata.clone());

		// proposal should have been voted successfully
		// the anchor-handler callback must have been called by bridge
		// event must be emitted in callback should exist
		event_exists(crate::Event::AnchorEdgeAdded);
		// edge count should be 1
		assert_eq!(
			1,
			<pallet_anchor::EdgeList<Test>>::iter_prefix_values(0)
				.into_iter()
				.count()
		);
		assert_eq!(edge_metadata, <pallet_anchor::EdgeList<Test>>::get(0, src_chain_id));
	})
}

#[test]
// Test
// 1. Create an anchor using `pallet-anchor` intrinsic call
// 2. Add an edge to the anchor using `pallet-anchor-handler` proposal through
// `pallet-bridge`
// 3. Update the edge of the anchor using
// `pallet-anchor-handler` proposal through `pallet-bridge`
fn anchor_update_proposal_edge_update() {
	new_test_ext().execute_with(|| {
		let src_chain_id = 1;
		setup_relayers(src_chain_id);
		create_anchor_using_pallet_call();
		let prop_id = 1;
		let root = Element::from_bytes(&[1; 32]);
		let height: u64 = 5;
		let edge_metadata = EdgeMetadata {
			src_chain_id,
			root,
			height,
		};
		relay_anchor_update_proposal(src_chain_id, prop_id, edge_metadata.clone());

		// proposal should have been voted successfully
		// the anchor-handler callback must have been called by bridge
		// event must be emitted in callback should exist
		event_exists(crate::Event::AnchorEdgeAdded);
		// follow
		// edge count should be 1
		assert_eq!(
			1,
			<pallet_anchor::EdgeList<Test>>::iter_prefix_values(0)
				.into_iter()
				.count()
		);
		assert_eq!(edge_metadata, <pallet_anchor::EdgeList<Test>>::get(0, src_chain_id));

		let root = Element::from_bytes(&[2; 32]);
		let height: u64 = 10;
		let edge_metadata = EdgeMetadata {
			src_chain_id,
			root,
			height,
		};
		relay_anchor_update_proposal(src_chain_id, prop_id + 1, edge_metadata.clone());

		// proposal should have been voted successfully
		// the anchor-handler callback must have been called by bridge
		// event must be emitted in callback should exist
		event_exists(crate::Event::AnchorEdgeUpdated);
		// edge count should be 1
		assert_eq!(
			1,
			<pallet_anchor::EdgeList<Test>>::iter_prefix_values(0)
				.into_iter()
				.count()
		);
		assert_eq!(edge_metadata, <pallet_anchor::EdgeList<Test>>::get(0, src_chain_id));
	})
}
