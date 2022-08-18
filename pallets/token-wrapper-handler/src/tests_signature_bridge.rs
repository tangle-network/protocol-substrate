use std::convert::TryInto;

use sp_core::{
	ecdsa::{self, Signature},
	keccak_256, Pair,
};

use super::mock_signature_bridge::{Call, Origin, SignatureBridge, Test, RELAYER_A};

use crate::mock_signature_bridge::new_test_ext_initialized;

use hex_literal::hex;

use crate::mock_signature_bridge::*;
use asset_registry::AssetType;
use frame_support::{assert_err, assert_ok};
use webb_proposals::{
	substrate::{TokenAddProposal, TokenRemoveProposal, WrappingFeeUpdateProposal},
	FunctionSignature, SubstrateTargetSystem,
};

const WRAPPING_FEE_FUNCTION_SIG: FunctionSignature = FunctionSignature(0u32.to_be_bytes());
const ADD_TOKEN_FUNCTION_SIG: FunctionSignature = FunctionSignature(1u32.to_be_bytes());
const REMOVE_TOKEN_FUNCTION_SIG: FunctionSignature = FunctionSignature(2u32.to_be_bytes());

fn get_add_token_resource() -> Vec<u8> {
	b"TokenWrapperHandler.execute_add_token_to_pool_share".to_vec()
}

fn get_remove_token_resource() -> Vec<u8> {
	b"TokenWrapperHandler.execute_remove_token_from_pool_share".to_vec()
}

fn get_edsca_account() -> ecdsa::Pair {
	let seed = "0x9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60";
	ecdsa::Pair::from_string(seed, None).unwrap()
}

fn get_public_uncompressed_key() -> [u8; 64] {
	hex!("8db55b05db86c0b1786ca49f095d76344c9e6056b2f02701a7e7f3c20aabfd913ebbe148dd17c56551a52952371071a6c604b3f3abe8f2c8fa742158ea6dd7d4")
}

fn make_wrapping_fee_proposal(
	header: webb_proposals::ProposalHeader,
	wrapping_fee_percent: u128,
	into_pool_share_id: u32,
) -> Vec<u8> {
	let wrapping_fee_proposal = WrappingFeeUpdateProposal::builder()
		.header(header)
		.wrapping_fee_percent(wrapping_fee_percent)
		.into_pool_share_id(into_pool_share_id)
		.build();
	wrapping_fee_proposal.to_bytes()
}

fn make_add_token_proposal(
	header: webb_proposals::ProposalHeader,
	name: String,
	asset_id: u32,
) -> Vec<u8> {
	let add_token_proposal =
		TokenAddProposal::builder().header(header).name(name).asset_id(asset_id).build();
	add_token_proposal.to_bytes()
}

fn make_remove_token_proposal(
	header: webb_proposals::ProposalHeader,
	name: String,
	asset_id: u32,
) -> Vec<u8> {
	let remove_token_proposal = TokenRemoveProposal::builder()
		.header(header)
		.name(name)
		.asset_id(asset_id)
		.build();

	remove_token_proposal.to_bytes()
}

fn make_proposal_header(
	resource_id: webb_proposals::ResourceId,
	function_signature: webb_proposals::FunctionSignature,
	nonce: webb_proposals::Nonce,
) -> webb_proposals::ProposalHeader {
	let header = webb_proposals::ProposalHeader::new(resource_id, function_signature, nonce);
	header
}

// ----Signature Bridge Tests----

#[test]
fn should_update_fee_with_sig_succeed() {
	let src_chain = webb_proposals::TypedChainId::Substrate(1);
	let this_chain_id = webb_proposals::TypedChainId::Substrate(5);
	let target_system = webb_proposals::TargetSystem::Substrate(SubstrateTargetSystem {
		pallet_index: 7,
		tree_id: 0,
	});
	let r_id = webb_proposals::ResourceId::new(target_system, this_chain_id);

	let src_id = src_chain.chain_id();

	let public_uncompressed = get_public_uncompressed_key();
	let pair = get_edsca_account();

	new_test_ext_initialized(
		src_id,
		r_id,
		b"TokenWrapperHandler.execute_wrapping_fee_proposal".to_vec(),
	)
	.execute_with(|| {
		let existential_balance: u32 = 1000;
		let pool_share_id = AssetRegistry::register_asset(
			b"meme".to_vec().try_into().unwrap(),
			AssetType::PoolShare(vec![]),
			existential_balance.into(),
		)
		.unwrap();
		let nonce = webb_proposals::Nonce::from(0x0001);
		let header = make_proposal_header(r_id, WRAPPING_FEE_FUNCTION_SIG, nonce);
		let wrapping_fee_proposal_bytes = make_wrapping_fee_proposal(header, 5, pool_share_id);
		let msg = keccak_256(&wrapping_fee_proposal_bytes);
		let sig: Signature = pair.sign_prehashed(&msg).into();
		let fee_call: Call =
			codec::Decode::decode(&mut &wrapping_fee_proposal_bytes[40..]).unwrap();
		// should fail to execute proposal as non-maintainer
		assert_err!(
			SignatureBridge::execute_proposal(
				Origin::signed(RELAYER_A),
				src_id,
				Box::new(fee_call.clone()),
				wrapping_fee_proposal_bytes.clone(),
				sig.0.to_vec(),
			),
			pallet_signature_bridge::Error::<Test, _>::InvalidPermissions
		);

		// set the maintainer
		assert_ok!(SignatureBridge::force_set_maintainer(
			Origin::root(),
			public_uncompressed.to_vec()
		));

		assert_ok!(SignatureBridge::execute_proposal(
			Origin::signed(RELAYER_A),
			src_id,
			Box::new(fee_call.clone()),
			wrapping_fee_proposal_bytes.clone(),
			sig.0.to_vec(),
		));

		assert_eq!(TokenWrapper::get_wrapping_fee(1000_u128, pool_share_id).unwrap(), 52);
	})
}

#[test]
fn should_add_token_with_sig_succeed() {
	let src_chain = webb_proposals::TypedChainId::Substrate(1);
	let this_chain_id = webb_proposals::TypedChainId::Substrate(5);
	let target_system = webb_proposals::TargetSystem::Substrate(SubstrateTargetSystem {
		pallet_index: 7,
		tree_id: 0,
	});
	let r_id = webb_proposals::ResourceId::new(target_system, this_chain_id);

	let src_id = src_chain.chain_id();

	let public_uncompressed = get_public_uncompressed_key();
	let pair = get_edsca_account();
	let add_token_resource_bytes = get_add_token_resource();
	new_test_ext_initialized(src_id, r_id, add_token_resource_bytes).execute_with(|| {
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
		// create add token proposal bytes
		let nonce = webb_proposals::Nonce::from(0x0001);
		let header = make_proposal_header(r_id, ADD_TOKEN_FUNCTION_SIG, nonce);
		let add_token_proposal_bytes =
			make_add_token_proposal(header, "meme".to_string(), first_token_id);
		let msg = keccak_256(&add_token_proposal_bytes);
		let sig: Signature = pair.sign_prehashed(&msg).into();
		// set the new maintainer
		assert_ok!(SignatureBridge::force_set_maintainer(
			Origin::root(),
			public_uncompressed.to_vec()
		));

		let add_token_call: Call =
			codec::Decode::decode(&mut &add_token_proposal_bytes[40..]).unwrap();
		// Create proposal (& vote)
		assert_ok!(SignatureBridge::execute_proposal(
			Origin::signed(RELAYER_A),
			src_id,
			Box::new(add_token_call.clone()),
			add_token_proposal_bytes,
			sig.0.to_vec(),
		));
		// Check that first_token_id is part of pool
		assert_eq!(AssetRegistry::contains_asset(pool_share_id, first_token_id), true);
	})
}

#[test]
fn should_remove_token_with_sig_succeed() {
	let src_chain = webb_proposals::TypedChainId::Substrate(1);
	let this_chain_id = webb_proposals::TypedChainId::Substrate(5);
	let target_system = webb_proposals::TargetSystem::Substrate(SubstrateTargetSystem {
		pallet_index: 7,
		tree_id: 0,
	});
	let r_id = webb_proposals::ResourceId::new(target_system, this_chain_id);
	let r_id_add_token = webb_proposals::ResourceId::new(target_system, this_chain_id);
	let r_id_remove_token = webb_proposals::ResourceId::new(target_system, this_chain_id);

	let src_id = src_chain.chain_id();

	let public_uncompressed = get_public_uncompressed_key();
	let pair = get_edsca_account();
	new_test_ext_initialized(src_id, r_id, b"System.remark".to_vec()).execute_with(|| {
		assert_ok!(SignatureBridge::set_resource(Origin::root(), r_id_add_token));
		assert_ok!(SignatureBridge::set_resource(Origin::root(), r_id_remove_token));

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
		let nonce = webb_proposals::Nonce::from(0x0001);
		let header = make_proposal_header(r_id, ADD_TOKEN_FUNCTION_SIG, nonce);
		let add_token_proposal_bytes =
			make_add_token_proposal(header, "meme".to_string(), first_token_id);

		let msg = keccak_256(&add_token_proposal_bytes);
		let sig: Signature = pair.sign_prehashed(&msg).into();

		// set the new maintainer
		assert_ok!(SignatureBridge::force_set_maintainer(
			Origin::root(),
			public_uncompressed.to_vec()
		));

		let add_token_call: Call =
			codec::Decode::decode(&mut &add_token_proposal_bytes[40..]).unwrap();
		// Create proposal (& vote)
		assert_ok!(SignatureBridge::execute_proposal(
			Origin::signed(RELAYER_A),
			src_id,
			Box::new(add_token_call.clone()),
			add_token_proposal_bytes,
			sig.0.to_vec(),
		));
		// Check that first_token_id is part of pool
		assert_eq!(AssetRegistry::contains_asset(pool_share_id, first_token_id), true);
		let nonce = webb_proposals::Nonce::from(0x0002);
		let header = make_proposal_header(r_id, REMOVE_TOKEN_FUNCTION_SIG, nonce);
		let remove_token_proposal_bytes =
			make_remove_token_proposal(header, "meme".to_string(), first_token_id);
		let msg = keccak_256(&remove_token_proposal_bytes);
		let sig: Signature = pair.sign_prehashed(&msg).into();

		let remove_token_call: Call =
			codec::Decode::decode(&mut &remove_token_proposal_bytes[40..]).unwrap();

		assert_ok!(SignatureBridge::execute_proposal(
			Origin::signed(RELAYER_A),
			src_id,
			Box::new(remove_token_call.clone()),
			remove_token_proposal_bytes,
			sig.0.to_vec(),
		));

		assert_eq!(AssetRegistry::contains_asset(pool_share_id, first_token_id), false);
	})
}

#[test]
fn should_fail_to_remove_token_not_in_pool_with_sig() {
	let src_chain = webb_proposals::TypedChainId::Substrate(1);
	let this_chain_id = webb_proposals::TypedChainId::Substrate(5);
	let target_system = webb_proposals::TargetSystem::Substrate(SubstrateTargetSystem {
		pallet_index: 7,
		tree_id: 0,
	});
	let r_id = webb_proposals::ResourceId::new(target_system, this_chain_id);

	let src_id = src_chain.chain_id();

	let public_uncompressed = get_public_uncompressed_key();
	let pair = get_edsca_account();
	let remove_token_resource_bytes = get_remove_token_resource();
	new_test_ext_initialized(src_id, r_id, remove_token_resource_bytes).execute_with(|| {
		let existential_balance: u32 = 1000;

		let first_token_id = AssetRegistry::register_asset(
			b"btcs".to_vec().try_into().unwrap(),
			AssetType::Token,
			existential_balance.into(),
		)
		.unwrap();

		AssetRegistry::register_asset(
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
		let nonce = webb_proposals::Nonce::from(0x0001);
		let header = make_proposal_header(r_id, REMOVE_TOKEN_FUNCTION_SIG, nonce);
		let remove_token_proposal_bytes =
			make_remove_token_proposal(header, "meme".to_string(), first_token_id);
		let msg = keccak_256(&remove_token_proposal_bytes);
		let sig: Signature = pair.sign_prehashed(&msg).into();
		let remove_token_call: Call =
			codec::Decode::decode(&mut &remove_token_proposal_bytes[40..]).unwrap();
		assert_err!(
			SignatureBridge::execute_proposal(
				Origin::signed(RELAYER_A),
				src_id,
				Box::new(remove_token_call.clone()),
				remove_token_proposal_bytes,
				sig.0.to_vec(),
			),
			asset_registry::Error::<Test>::AssetNotFoundInPool
		);
	})
}

#[test]
fn should_add_many_tokens_with_sig_succeed() {
	let src_chain = webb_proposals::TypedChainId::Substrate(1);
	let this_chain_id = webb_proposals::TypedChainId::Substrate(5);
	let target_system = webb_proposals::TargetSystem::Substrate(SubstrateTargetSystem {
		pallet_index: 7,
		tree_id: 0,
	});
	let r_id = webb_proposals::ResourceId::new(target_system, this_chain_id);

	let src_id = src_chain.chain_id();

	let public_uncompressed = get_public_uncompressed_key();
	let pair = get_edsca_account();
	let add_token_resource_bytes = get_add_token_resource();
	new_test_ext_initialized(src_id, r_id, add_token_resource_bytes).execute_with(|| {
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
		let nonce = webb_proposals::Nonce::from(0x0001);
		let header = make_proposal_header(r_id, ADD_TOKEN_FUNCTION_SIG, nonce);
		let add_token_proposal_bytes =
			make_add_token_proposal(header, "meme".to_string(), first_token_id);
		let add_token_call: Call =
			codec::Decode::decode(&mut &add_token_proposal_bytes[40..]).unwrap();
		let msg = keccak_256(&add_token_proposal_bytes);
		let sig: Signature = pair.sign_prehashed(&msg).into();
		// Create proposal (& vote)
		assert_ok!(SignatureBridge::execute_proposal(
			Origin::signed(RELAYER_A),
			src_id,
			Box::new(add_token_call.clone()),
			add_token_proposal_bytes,
			sig.0.to_vec(),
		));
		let nonce = webb_proposals::Nonce::from(0x0002);
		let header = make_proposal_header(r_id, ADD_TOKEN_FUNCTION_SIG, nonce);
		let add_token_proposal_bytes =
			make_add_token_proposal(header, "meme".to_string(), second_token_id);
		let add_token_call: Call =
			codec::Decode::decode(&mut &add_token_proposal_bytes[40..]).unwrap();
		let msg = keccak_256(&add_token_proposal_bytes);
		let sig: Signature = pair.sign_prehashed(&msg).into();

		// Create proposal (& vote)
		assert_ok!(SignatureBridge::execute_proposal(
			Origin::signed(RELAYER_A),
			src_id,
			Box::new(add_token_call.clone()),
			add_token_proposal_bytes,
			sig.0.to_vec(),
		));
		let nonce = webb_proposals::Nonce::from(0x0003);
		let header = make_proposal_header(r_id, ADD_TOKEN_FUNCTION_SIG, nonce);
		let add_token_proposal_bytes =
			make_add_token_proposal(header, "meme".to_string(), third_token_id);
		let add_token_call: Call =
			codec::Decode::decode(&mut &add_token_proposal_bytes[40..]).unwrap();
		let msg = keccak_256(&add_token_proposal_bytes);
		let sig: Signature = pair.sign_prehashed(&msg).into();

		// Create proposal (& vote)
		assert_ok!(SignatureBridge::execute_proposal(
			Origin::signed(RELAYER_A),
			src_id,
			Box::new(add_token_call.clone()),
			add_token_proposal_bytes,
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
	let src_chain = webb_proposals::TypedChainId::Substrate(1);
	let this_chain_id = webb_proposals::TypedChainId::Substrate(5);
	let target_system = webb_proposals::TargetSystem::Substrate(SubstrateTargetSystem {
		pallet_index: 7,
		tree_id: 0,
	});
	let r_id = webb_proposals::ResourceId::new(target_system, this_chain_id);

	let src_id = src_chain.chain_id();

	let public_uncompressed = get_public_uncompressed_key();
	let pair = get_edsca_account();
	let add_token_resource_bytes = get_add_token_resource();
	new_test_ext_initialized(src_id, r_id, add_token_resource_bytes).execute_with(|| {
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
		let nonce = webb_proposals::Nonce::from(0x0001);
		let header = make_proposal_header(r_id, ADD_TOKEN_FUNCTION_SIG, nonce);
		let add_token_proposal_bytes =
			make_add_token_proposal(header, "meme".to_string(), first_token_id);
		let add_token_call: Call =
			codec::Decode::decode(&mut &add_token_proposal_bytes[40..]).unwrap();
		let msg = keccak_256(&add_token_proposal_bytes);
		let sig: Signature = pair.sign_prehashed(&msg).into();

		// set the new maintainer
		assert_ok!(SignatureBridge::force_set_maintainer(
			Origin::root(),
			public_uncompressed.to_vec()
		));
		// Create proposal
		assert_ok!(SignatureBridge::execute_proposal(
			Origin::signed(RELAYER_A),
			src_id,
			Box::new(add_token_call.clone()),
			add_token_proposal_bytes.clone(),
			sig.0.to_vec(),
		));
		// Check that first_token_id is part of pool
		assert_eq!(AssetRegistry::contains_asset(pool_share_id, first_token_id), true);

		// Have to remake prop_data with incremented nonce
		let nonce = webb_proposals::Nonce::from(0x0002);
		let header = make_proposal_header(r_id, ADD_TOKEN_FUNCTION_SIG, nonce);
		let add_token_proposal_bytes =
			make_add_token_proposal(header, "meme".to_string(), first_token_id);
		let add_token_call: Call =
			codec::Decode::decode(&mut &add_token_proposal_bytes[40..]).unwrap();
		let msg = keccak_256(&add_token_proposal_bytes);
		let sig: Signature = pair.sign_prehashed(&msg).into();

		assert_err!(
			SignatureBridge::execute_proposal(
				Origin::signed(RELAYER_A),
				src_id,
				Box::new(add_token_call.clone()),
				add_token_proposal_bytes.clone(),
				sig.0.to_vec(),
			),
			asset_registry::Error::<Test>::AssetExistsInPool
		);
	})
}

#[test]
fn should_fail_to_add_non_existent_token_with_sig() {
	let src_chain = webb_proposals::TypedChainId::Substrate(1);
	let this_chain_id = webb_proposals::TypedChainId::Substrate(5);
	let target_system = webb_proposals::TargetSystem::Substrate(SubstrateTargetSystem {
		pallet_index: 7,
		tree_id: 0,
	});
	let r_id = webb_proposals::ResourceId::new(target_system, this_chain_id);

	let src_id = src_chain.chain_id();

	let public_uncompressed = get_public_uncompressed_key();
	let pair = get_edsca_account();

	new_test_ext_initialized(
		src_id,
		r_id,
		b"TokenWrapperHandler.execute_add_token_to_pool_shares".to_vec(),
	)
	.execute_with(|| {
		let existential_balance: u32 = 1000;

		let first_token_id = 100;

		AssetRegistry::register_asset(
			b"meme".to_vec().try_into().unwrap(),
			AssetType::PoolShare(vec![]),
			existential_balance.into(),
		)
		.unwrap();
		let nonce = webb_proposals::Nonce::from(0x0001);
		let header = make_proposal_header(r_id, ADD_TOKEN_FUNCTION_SIG, nonce);
		let add_token_proposal_bytes =
			make_add_token_proposal(header, "meme".to_string(), first_token_id);
		let add_token_call: Call =
			codec::Decode::decode(&mut &add_token_proposal_bytes[40..]).unwrap();
		let msg = keccak_256(&add_token_proposal_bytes);
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
				src_id,
				Box::new(add_token_call.clone()),
				add_token_proposal_bytes.clone(),
				sig.0.to_vec(),
			),
			asset_registry::Error::<Test>::AssetNotRegistered
		);
	})
}
