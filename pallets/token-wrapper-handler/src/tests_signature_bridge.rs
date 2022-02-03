use std::convert::TryInto;

use sp_core::{
	ecdsa::{self, Signature},
	keccak_256, Pair, Public,
};

use super::{
	mock_signature_bridge::{
		assert_events, new_test_ext, Balances, Call, ChainIdentifier, Event, Origin,
		ProposalLifetime, SignatureBridge, System, Test, ENDOWED_BALANCE, RELAYER_A, RELAYER_B,
		RELAYER_C,
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
use frame_support::{
	assert_err, assert_ok, dispatch::DispatchResultWithPostInfo, error::BadOrigin,
};

const TEST_THRESHOLD: u32 = 2;

fn get_add_token_resource() -> Vec<u8> {
	b"TokenWrapperHandler.execute_add_token_to_pool_share".to_vec()
}

fn get_remove_token_resource() -> Vec<u8> {
	b"TokenWrapperHandler.execute_remove_token_to_pool_share".to_vec()
}

fn make_wrapping_fee_proposal(
	resource_id: &[u8; 32],
	wrapping_fee_percent: u128,
	into_pool_share_id: u32,
) -> Call {
	Call::TokenWrapperHandler(crate::Call::execute_wrapping_fee_proposal {
		r_id: *resource_id,
		wrapping_fee_percent,
		into_pool_share_id,
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
fn should_update_fee_with_sig_succeed() {
	let src_id = 1u32;
	let r_id = derive_resource_id(src_id, b"execute_wrapping_fee_proposal");
	let public_uncompressed = hex!("8db55b05db86c0b1786ca49f095d76344c9e6056b2f02701a7e7f3c20aabfd913ebbe148dd17c56551a52952371071a6c604b3f3abe8f2c8fa742158ea6dd7d4");
	let pair = ecdsa::Pair::from_string(
		"0x9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60",
		None,
	)
	.unwrap();

	new_test_ext_initialized(
		src_id,
		r_id,
		b"TokenWrapperHandler.execute_wrapping_fee_proposal".to_vec(),
	)
	.execute_with(|| {
		let prop_id = 1;
		let existential_balance: u32 = 1000;
		let pool_share_id = AssetRegistry::register_asset(
			b"meme".to_vec().try_into().unwrap(),
			AssetType::PoolShare(vec![]),
			existential_balance.into(),
		)
		.unwrap();

		let proposal = make_wrapping_fee_proposal(&r_id, 5, pool_share_id);
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
		// Create proposal (& vote)
		assert_ok!(SignatureBridge::execute_proposal(
			Origin::signed(RELAYER_A),
			prop_id,
			src_id,
			r_id,
			Box::new(proposal.clone()),
			sig.0.to_vec(),
		));

		assert_eq!(TokenWrapper::get_wrapping_fee(1000_u128, pool_share_id).unwrap(), 52);
	})
}

#[test]
fn should_add_token_with_sig_succeed() {
	let src_id = 1u32;
	let r_id = derive_resource_id(src_id, b"execute_add_token_to_pool_share");
	let public_uncompressed = hex!("8db55b05db86c0b1786ca49f095d76344c9e6056b2f02701a7e7f3c20aabfd913ebbe148dd17c56551a52952371071a6c604b3f3abe8f2c8fa742158ea6dd7d4");
	let pair = ecdsa::Pair::from_string(
		"0x9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60",
		None,
	)
	.unwrap();

	new_test_ext_initialized(
		src_id,
		r_id,
		b"TokenWrapperHandler.execute_add_token_to_pool_share".to_vec(),
	)
	.execute_with(|| {
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

		let prop_id = 1;
		let proposal = make_add_token_proposal(&r_id, b"meme".to_vec(), first_token_id);
		let msg = keccak_256(&proposal.encode());
		let sig: Signature = pair.sign_prehashed(&msg).into();

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
		// Check that first_token_id is part of pool
		assert_eq!(AssetRegistry::contains_asset(pool_share_id, first_token_id), true);
	})
}

#[test]
fn should_remove_token_with_sig_succeed() {
	let src_id = 1u32;
	let r_id = derive_resource_id(src_id, b"remark");
	let r_id_add_token = derive_resource_id(src_id, b"execute_add_token_to_pool_share");
	let r_id_remove_token = derive_resource_id(src_id, b"execute_remove_token_from_pool_share");
	let public_uncompressed = hex!("8db55b05db86c0b1786ca49f095d76344c9e6056b2f02701a7e7f3c20aabfd913ebbe148dd17c56551a52952371071a6c604b3f3abe8f2c8fa742158ea6dd7d4");
	let pair = ecdsa::Pair::from_string(
		"0x9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60",
		None,
	)
	.unwrap();

	new_test_ext_initialized(src_id, r_id, b"System.remark".to_vec()).execute_with(|| {
		assert_ok!(SignatureBridge::set_resource(
			Origin::root(),
			r_id_add_token,
			b"TokenWrapperHandler.execute_add_token_to_pool_share".to_vec()
		));
		assert_ok!(SignatureBridge::set_resource(
			Origin::root(),
			r_id_remove_token,
			b"TokenWrapperHandler.execute_remove_token_from_pool_share".to_vec()
		));

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

		let prop_id = 1;
		let proposal = make_add_token_proposal(&r_id_add_token, b"meme".to_vec(), first_token_id);
		let msg = keccak_256(&proposal.encode());
		let sig: Signature = pair.sign_prehashed(&msg).into();

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
			r_id_add_token,
			Box::new(proposal.clone()),
			sig.0.to_vec(),
		));
		// Check that first_token_id is part of pool
		assert_eq!(AssetRegistry::contains_asset(pool_share_id, first_token_id), true);

		let prop_id = 2;
		let proposal =
			make_remove_token_proposal(&r_id_remove_token, b"meme".to_vec(), first_token_id);
		let msg = keccak_256(&proposal.encode());
		let sig: Signature = pair.sign_prehashed(&msg).into();

		assert_ok!(SignatureBridge::execute_proposal(
			Origin::signed(RELAYER_A),
			prop_id,
			src_id,
			r_id_remove_token,
			Box::new(proposal.clone()),
			sig.0.to_vec(),
		));

		assert_eq!(AssetRegistry::contains_asset(pool_share_id, first_token_id), false);
	})
}

#[test]
fn should_fail_to_remove_token_not_in_pool_with_sig() {
	let src_id = 1u32;
	let r_id = derive_resource_id(src_id, b"execute_remove_token_from_pool_share");
	let public_uncompressed = hex!("8db55b05db86c0b1786ca49f095d76344c9e6056b2f02701a7e7f3c20aabfd913ebbe148dd17c56551a52952371071a6c604b3f3abe8f2c8fa742158ea6dd7d4");
	let pair = ecdsa::Pair::from_string(
		"0x9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60",
		None,
	)
	.unwrap();

	new_test_ext_initialized(
		src_id,
		r_id,
		b"TokenWrapperHandler.execute_remove_token_from_pool_share".to_vec(),
	)
	.execute_with(|| {
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

		// set the new maintainer
		assert_ok!(SignatureBridge::force_set_maintainer(
			Origin::root(),
			public_uncompressed.to_vec()
		));

		let prop_id = 1;
		let proposal = make_remove_token_proposal(&r_id, b"meme".to_vec(), first_token_id);
		let msg = keccak_256(&proposal.encode());
		let sig: Signature = pair.sign_prehashed(&msg).into();

		assert_err!(
			SignatureBridge::execute_proposal(
				Origin::signed(RELAYER_A),
				prop_id,
				src_id,
				r_id,
				Box::new(proposal.clone()),
				sig.0.to_vec(),
			),
			asset_registry::Error::<Test>::AssetNotFoundInPool
		);
	})
}

#[test]
fn should_add_many_tokens_with_sig_succeed() {
	let src_id = 1u32;
	let r_id = derive_resource_id(src_id, b"execute_add_token_to_pool_share");
	let public_uncompressed = hex!("8db55b05db86c0b1786ca49f095d76344c9e6056b2f02701a7e7f3c20aabfd913ebbe148dd17c56551a52952371071a6c604b3f3abe8f2c8fa742158ea6dd7d4");
	let pair = ecdsa::Pair::from_string(
		"0x9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60",
		None,
	)
	.unwrap();

	new_test_ext_initialized(
		src_id,
		r_id,
		b"TokenWrapperHandler.execute_add_token_to_pool_share".to_vec(),
	)
	.execute_with(|| {
		let existential_balance: u32 = 1000;

		let first_token_id = AssetRegistry::register_asset(
			b"btcs".to_vec().try_into().unwrap(),
			AssetType::Token,
			existential_balance.into(),
		)
		.unwrap();

		let second_token_id = AssetRegistry::register_asset(
			b"doge".to_vec().try_into().unwrap(),
			AssetType::Token,
			existential_balance.into(),
		)
		.unwrap();

		let third_token_id = AssetRegistry::register_asset(
			b"shib".to_vec().try_into().unwrap(),
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
		// set the new maintainer
		assert_ok!(SignatureBridge::force_set_maintainer(
			Origin::root(),
			public_uncompressed.to_vec()
		));
		let prop_id = 1;
		let proposal = make_add_token_proposal(&r_id, b"meme".to_vec(), first_token_id);
		let msg = keccak_256(&proposal.encode());
		let sig: Signature = pair.sign_prehashed(&msg).into();

		// Create proposal (& vote)
		assert_ok!(SignatureBridge::execute_proposal(
			Origin::signed(RELAYER_A),
			prop_id,
			src_id,
			r_id,
			Box::new(proposal.clone()),
			sig.0.to_vec(),
		));

		let prop_id = 2;
		let proposal = make_add_token_proposal(&r_id, b"meme".to_vec(), second_token_id);
		let msg = keccak_256(&proposal.encode());
		let sig: Signature = pair.sign_prehashed(&msg).into();

		// Create proposal (& vote)
		assert_ok!(SignatureBridge::execute_proposal(
			Origin::signed(RELAYER_A),
			prop_id,
			src_id,
			r_id,
			Box::new(proposal.clone()),
			sig.0.to_vec(),
		));

		let prop_id = 3;
		let proposal = make_add_token_proposal(&r_id, b"meme".to_vec(), third_token_id);
		let msg = keccak_256(&proposal.encode());
		let sig: Signature = pair.sign_prehashed(&msg).into();

		// Create proposal (& vote)
		assert_ok!(SignatureBridge::execute_proposal(
			Origin::signed(RELAYER_A),
			prop_id,
			src_id,
			r_id,
			Box::new(proposal.clone()),
			sig.0.to_vec(),
		));

		// Check that first_token_id is part of pool
		assert_eq!(AssetRegistry::contains_asset(pool_share_id, first_token_id), true);

		// Check that second_token_id is part of pool
		assert_eq!(AssetRegistry::contains_asset(pool_share_id, second_token_id), true);

		// Check that third_token_id is part of pool
		assert_eq!(AssetRegistry::contains_asset(pool_share_id, third_token_id), true);
	})
}

#[test]
fn should_fail_to_add_same_token_with_sig() {
	let src_id = 1u32;
	let r_id = derive_resource_id(src_id, b"execute_add_token_to_pool_share");
	let public_uncompressed = hex!("8db55b05db86c0b1786ca49f095d76344c9e6056b2f02701a7e7f3c20aabfd913ebbe148dd17c56551a52952371071a6c604b3f3abe8f2c8fa742158ea6dd7d4");
	let pair = ecdsa::Pair::from_string(
		"0x9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60",
		None,
	)
	.unwrap();

	new_test_ext_initialized(
		src_id,
		r_id,
		b"TokenWrapperHandler.execute_add_token_to_pool_share".to_vec(),
	)
	.execute_with(|| {
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

		let prop_id = 1;
		let proposal = make_add_token_proposal(&r_id, b"meme".to_vec(), first_token_id);
		let msg = keccak_256(&proposal.encode());
		let sig: Signature = pair.sign_prehashed(&msg).into();

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
		// Check that first_token_id is part of pool
		assert_eq!(AssetRegistry::contains_asset(pool_share_id, first_token_id), true);

		let prop_id = 2;

		assert_err!(
			SignatureBridge::execute_proposal(
				Origin::signed(RELAYER_A),
				prop_id,
				src_id,
				r_id,
				Box::new(proposal.clone()),
				sig.0.to_vec(),
			),
			asset_registry::Error::<Test>::AssetExistsInPool
		);
	})
}

#[test]
fn should_fail_to_add_non_existent_token_with_sig() {
	let src_id = 1u32;
	let r_id = derive_resource_id(src_id, b"execute_add_token_to_pool_share");
	let public_uncompressed = hex!("8db55b05db86c0b1786ca49f095d76344c9e6056b2f02701a7e7f3c20aabfd913ebbe148dd17c56551a52952371071a6c604b3f3abe8f2c8fa742158ea6dd7d4");
	let pair = ecdsa::Pair::from_string(
		"0x9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60",
		None,
	)
	.unwrap();

	new_test_ext_initialized(
		src_id,
		r_id,
		b"TokenWrapperHandler.execute_add_token_to_pool_shares".to_vec(),
	)
	.execute_with(|| {
		let existential_balance: u32 = 1000;

		let first_token_id = 100;

		let pool_share_id = AssetRegistry::register_asset(
			b"meme".to_vec().try_into().unwrap(),
			AssetType::PoolShare(vec![]),
			existential_balance.into(),
		)
		.unwrap();

		let prop_id = 1;
		let proposal = make_add_token_proposal(&r_id, b"meme".to_vec(), first_token_id);
		let msg = keccak_256(&proposal.encode());
		let sig: Signature = pair.sign_prehashed(&msg).into();

		// set the new maintainer
		assert_ok!(SignatureBridge::force_set_maintainer(
			Origin::root(),
			public_uncompressed.to_vec()
		));
		// Create proposal (& vote)
		assert_err!(
			SignatureBridge::execute_proposal(
				Origin::signed(RELAYER_A),
				prop_id,
				src_id,
				r_id,
				Box::new(proposal.clone()),
				sig.0.to_vec(),
			),
			asset_registry::Error::<Test>::AssetNotRegistered
		);
	})
}
