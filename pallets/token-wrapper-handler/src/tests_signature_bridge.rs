use std::convert::TryInto;

use sp_core::{
	ecdsa::{self, Signature},
	keccak_256, Pair, Public,
};

use super::{
	mock_signature_bridge::{
		assert_events, new_test_ext, Balances, Call, ChainIdentifier, Event, Origin, ProposalLifetime, SignatureBridge,
		System, Test, ENDOWED_BALANCE, RELAYER_A, RELAYER_B, RELAYER_C,
	},
	*,
};

use crate::mock_signature_bridge::new_test_ext_initialized;

use codec::{Decode, Encode, EncodeLike};

use hex_literal::hex;
use pallet_signature_bridge::utils::derive_resource_id;
use webb_primitives::{signing::SigningSystem, ResourceId};

use crate::mock_signature_bridge::*;

use asset_registry::AssetType;
use frame_support::{assert_err, assert_ok, dispatch::DispatchResultWithPostInfo, error::BadOrigin};

const TEST_THRESHOLD: u32 = 2;

fn get_add_token_resource() -> Vec<u8> {
	b"TokenWrapperHandler.execute_add_token_to_pool_share".to_vec()
}

fn get_remove_token_resource() -> Vec<u8> {
	b"TokenWrapperHandler.execute_remove_token_to_pool_share".to_vec()
}

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

// ----Signature Bridge Tests----

#[test]

fn should_update_fee_with_sig() {
	let src_id = 1u32;
	let r_id = derive_resource_id(src_id, b"execute_wrapping_fee_proposal");
	let public_uncompressed = hex!("8db55b05db86c0b1786ca49f095d76344c9e6056b2f02701a7e7f3c20aabfd913ebbe148dd17c56551a52952371071a6c604b3f3abe8f2c8fa742158ea6dd7d4");
	let pair = ecdsa::Pair::from_string(
		"0x9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60",
		None,
	)
	.unwrap();

	new_test_ext_initialized(src_id, r_id, b"System.remark".to_vec()).execute_with(|| {
		let prop_id = 1;
		let proposal = make_wrapping_fee_proposal(&r_id, 5);
		let msg = keccak_256(&proposal.encode());
		let sig: Signature = pair.sign_prehashed(&msg).into();
		// should fail to execute proposal as non-maintainer
		// assert_err!(
		// 	SignatureBridge::execute_proposal(
		// 		Origin::signed(RELAYER_A),
		// 		prop_id,
		// 		src_id,
		// 		r_id,
		// 		Box::new(proposal.clone()),
		// 		sig.0.to_vec(),
		// 	),
		// 	Error::<Test>::InvalidPermissions
		// );

		// set the new maintainer
		assert_ok!(SignatureBridge::force_set_maintainer(
			Origin::root(),
			public_uncompressed.to_vec()
		));
		// Create proposal (& vote)
		assert_ok!(SignatureBridge::execute_proposal(
			Origin::signed(RELAYER_A),
			prop_id,
			src_id,
			r_id,
			Box::new(proposal.clone()),
			sig.0.to_vec(),
		));
		assert_eq!(TokenWrapper::get_wrapping_fee(1000_u128), 52);
		// assert_events(vec![
		// 	Event::SignatureBridge(pallet_signature_bridge::Event::
		// ProposalApproved { 		chain_id: src_id,
		// 		proposal_nonce: prop_id,
		// 	}),
		// 	Event::SignatureBridge(pallet_signature_bridge::Event::
		// ProposalSucceeded { 		chain_id: src_id,
		// 		proposal_nonce: prop_id,
		// 	}),
		// ]);
	})
}
