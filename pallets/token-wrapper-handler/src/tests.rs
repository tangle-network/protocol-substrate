use std::convert::TryInto;

use crate::mock::*;

use asset_registry::AssetType;
use frame_support::{assert_err, assert_ok, error::BadOrigin};
use pallet_bridge::types::{ProposalStatus, ProposalVotes};

const TEST_THRESHOLD: u32 = 2;

fn make_wrapping_fee_proposal(resource_id: &[u8; 32], wrapping_fee_percent: u128) -> Call {
	Call::TokenWrapperHandler(crate::Call::execute_wrapping_fee_proposal {
		r_id: *resource_id,
		wrapping_fee_percent,
	})
}

fn make_add_token_proposal(resource_id: &[u8; 32], name: Vec<u8>, asset_id: u32) -> Call {
	Call::TokenWrapperHandler(crate::Call::execute_add_token_to_pool_share {
		r_id: *resource_id,
		name,
		asset_id,
	})
}

fn make_remove_token_proposal(resource_id: &[u8; 32], name: Vec<u8>, asset_id: u32) -> Call {
	Call::TokenWrapperHandler(crate::Call::execute_remove_token_from_pool_share {
		r_id: *resource_id,
		name,
		asset_id,
	})
}

fn setup_relayers(src_id: u32) {
	// set anchors threshold
	assert_ok!(Bridge::set_threshold(Origin::root(), TEST_THRESHOLD,));
	// add relayers
	assert_eq!(Bridge::relayer_count(), 0);
	assert_ok!(Bridge::add_relayer(Origin::root(), RELAYER_A));
	assert_ok!(Bridge::add_relayer(Origin::root(), RELAYER_B));
	assert_eq!(Bridge::relayer_count(), 2);
	// whitelist chain
	assert_ok!(Bridge::whitelist_chain(Origin::root(), src_id));
}

fn relay_fee_update_proposal(src_chain_id: u32, resource_id: &[u8; 32], prop_id: u64, wrapping_fee_percent: u128) {
	// create fee update proposal
	let resource = b"TokenWrapperHandler.execute_wrapping_fee_proposal".to_vec();
	let update_proposal = make_wrapping_fee_proposal(resource_id, wrapping_fee_percent);
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

fn relay_add_token_proposal(src_chain_id: u32, resource_id: &[u8; 32], prop_id: u64, name: Vec<u8>, asset_id: u32) {
	// create fee update proposal
	let resource = b"TokenWrapperHandler.execute_add_token_to_pool_share".to_vec();
	let update_proposal = make_add_token_proposal(resource_id, name, asset_id);
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

fn relay_remove_token_proposal(src_chain_id: u32, resource_id: &[u8; 32], prop_id: u64, name: Vec<u8>, asset_id: u32) {
	// create fee update proposal
	let resource = b"TokenWrapperHandler.execute_remove_token_from_pool_share".to_vec();
	let update_proposal = make_remove_token_proposal(resource_id, name, asset_id);
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
fn should_update_fee() {
	new_test_ext().execute_with(|| {
		let src_chain_id = 1;
		let resource_id = pallet_bridge::utils::derive_resource_id(src_chain_id, b"hash");
		let prop_id = 1;
		// create fee update proposal
		setup_relayers(src_chain_id);
		relay_fee_update_proposal(src_chain_id, &resource_id, prop_id, 5);
		assert_eq!(TokenWrapper::get_wrapping_fee(1000_u128), 52);
	})
}

#[test]
fn should_add_token() {
	new_test_ext().execute_with(|| {
		// Create tokens
		let existential_balance: u32 = 1000;
		let first_token_id = AssetRegistry::register_asset(
			b"btcs".to_vec().try_into().unwrap(),
			AssetType::Token,
			existential_balance.into(),
		)
		.unwrap();

		let pool_share_id = AssetRegistry::register_asset(
			b"meme".to_vec().try_into().unwrap(),
			AssetType::PoolShare(vec![]),
			existential_balance.into(),
		)
		.unwrap();

		//Setup necessary relayers/bridge functionality
		let src_chain_id = 1;
		let resource_id = pallet_bridge::utils::derive_resource_id(src_chain_id, b"hash");
		let prop_id = 1;

		setup_relayers(src_chain_id);
		relay_add_token_proposal(
			src_chain_id,
			&resource_id,
			prop_id,
			b"meme".to_vec().try_into().unwrap(),
			first_token_id,
		);

		// Check that first_token_id is part of pool
		assert_eq!(AssetRegistry::contains_asset(pool_share_id, first_token_id), true);
	})
}

#[test]
fn should_remove_token() {
	new_test_ext().execute_with(|| {
		// Create tokens
		let existential_balance: u32 = 1000;
		let first_token_id = AssetRegistry::register_asset(
			b"btcs".to_vec().try_into().unwrap(),
			AssetType::Token,
			existential_balance.into(),
		)
		.unwrap();

		let pool_share_id = AssetRegistry::register_asset(
			b"meme".to_vec().try_into().unwrap(),
			AssetType::PoolShare(vec![]),
			existential_balance.into(),
		)
		.unwrap();

		//Setup necessary relayers/bridge functionality
		let src_chain_id = 1;
		let resource_id = pallet_bridge::utils::derive_resource_id(src_chain_id, b"hash");
		let prop_id = 1;

		setup_relayers(src_chain_id);
		relay_add_token_proposal(
			src_chain_id,
			&resource_id,
			prop_id,
			b"meme".to_vec().try_into().unwrap(),
			first_token_id,
		);

		// Check that first_token_id is part of pool
		assert_eq!(AssetRegistry::contains_asset(pool_share_id, first_token_id), true);

		relay_remove_token_proposal(
			src_chain_id,
			&resource_id,
			prop_id,
			b"meme".to_vec().try_into().unwrap(),
			first_token_id,
		);

		assert_eq!(AssetRegistry::contains_asset(pool_share_id, first_token_id), false);
	})
}
