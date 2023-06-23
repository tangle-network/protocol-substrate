use super::*;
use crate::{mock::*, test_utils::*, Error};
use webb_primitives::ElementTrait;

use crate::Instance1;
use ark_serialize::CanonicalSerialize;
use circom_proving::{generate_proof, verify_proof};
use frame_support::{assert_err, assert_ok};
use webb_primitives::webb_proposals::{
	ResourceId, SubstrateTargetSystem, TargetSystem, TypedChainId,
};

#[test]
fn should_initialize_parameters() {
	new_test_ext().execute_with(|| {});
}

/// testing update roots
#[test]
fn should_fail_update_without_resource_id_initialization() {
	new_test_ext().execute_with(|| {
		setup_environment();
		let src_id = TypedChainId::Substrate(1);
		let target_system =
			TargetSystem::Substrate(SubstrateTargetSystem { pallet_index: 11, tree_id: 0 });
		let src_target_system = target_system;
		let src_resource_id = ResourceId::new(src_target_system, src_id);

		let raw = include_str!("../firstTransactionInputs.json");
		let inputs_raw: InputsRaw = serde_json::from_str(raw).unwrap();
		let circuit_inputs = RewardCircuitInputs::from_raw(&inputs_raw);

		let unspent_update_0 = AnonymityMiningClaims::update_unspent_root(
			src_resource_id,
			Element::from_bytes(&circuit_inputs.unspent_roots[0].to_bytes_be().1),
		);
		assert_err!(unspent_update_0, Error::<Test, Instance1>::InvalidResourceId);

		let unspent_update_1 = AnonymityMiningClaims::update_unspent_root(
			src_resource_id,
			Element::from_bytes(&circuit_inputs.unspent_roots[1].to_bytes_be().1),
		);
		assert_err!(unspent_update_1, Error::<Test, Instance1>::InvalidResourceId);
	})
}

/// testing update roots
#[test]
fn should_init_and_update_roots() {
	new_test_ext().execute_with(|| {
		setup_environment();
		let src_id = TypedChainId::Substrate(1);
		let target_system =
			TargetSystem::Substrate(SubstrateTargetSystem { pallet_index: 11, tree_id: 0 });
		let src_target_system = target_system;
		let src_resource_id = ResourceId::new(src_target_system, src_id);

		let max_edges = 2u8;
		let depth = 30u8;
		let _tree_id = AnonymityMiningClaims::create(None, depth, max_edges, 0u32, 1u32).unwrap();

		let raw = include_str!("../firstTransactionInputs.json");
		let inputs_raw: InputsRaw = serde_json::from_str(raw).unwrap();
		let circuit_inputs: RewardCircuitInputs = RewardCircuitInputs::from_raw(&inputs_raw);
		let unspent_root_0 = Element::from_bytes(&circuit_inputs.unspent_roots[0].to_bytes_be().1);
		let unspent_root_1 = Element::from_bytes(&circuit_inputs.unspent_roots[1].to_bytes_be().1);
		let spent_root_0 = Element::from_bytes(&circuit_inputs.spent_roots[0].to_bytes_be().1);
		let spent_root_1 = Element::from_bytes(&circuit_inputs.spent_roots[1].to_bytes_be().1);

		let init_call = AnonymityMiningClaims::init_resource_id_history(
			src_resource_id,
			unspent_root_0,
			spent_root_0,
		);
		assert_ok!(init_call);

		let update_unspent_call =
			AnonymityMiningClaims::update_unspent_root(src_resource_id, unspent_root_1);
		assert_ok!(update_unspent_call);
		let update_spent_call =
			AnonymityMiningClaims::update_spent_root(src_resource_id, spent_root_1);
		assert_ok!(update_spent_call);
		let zero: RootIndex = 0u32;
		let cached_unspent_root_0 =
			AnonymityMiningClaims::cached_unspent_roots(src_resource_id, zero);
		assert_eq!(cached_unspent_root_0, unspent_root_0);
		let cached_spent_root_0 = AnonymityMiningClaims::cached_spent_roots(src_resource_id, zero);
		assert_eq!(cached_spent_root_0, spent_root_0);
		let one: RootIndex = 1u32;
		let cached_unspent_root_1 =
			AnonymityMiningClaims::cached_unspent_roots(src_resource_id, one);
		assert_eq!(cached_unspent_root_1, unspent_root_1);
		let cached_spent_root_1 = AnonymityMiningClaims::cached_spent_roots(src_resource_id, one);
		assert_eq!(cached_spent_root_1, spent_root_1);
	})
}

// fn create_claims_pallet(asset_id: u32) -> u32 {
// 	let max_edges = 2u32;
// 	let depth = 30u8;
// 	assert_ok!(AnonymityMiningClaims::create(
// 		None,
// 		depth,
// 		max_edges,
// 		asset_id,
// 		0u32.into()
// 	));
// 	MerkleTree::next_tree_id() - 1
// }
//
#[test]
fn should_create_pallet() {
	new_test_ext().execute_with(|| {
		setup_environment_with_circom();
		let max_edges = 2u8;
		let depth = 30u8;
		let call = AnonymityMiningClaims::create(None, depth, max_edges, 0u32, 1u32);
		assert_ok!(call);
	})
}

#[test]
#[ignore = "Needs to update the fixtures to the latest commit"]
fn circom_should_complete_30x2_reward_claim_with_json_file() {
	new_test_ext().execute_with(|| {
		let (params_2_2, wc_2_2) = setup_environment_with_circom();

		let src_id = TypedChainId::Substrate(1);
		let target_id = TypedChainId::Substrate(5);
		let target_system =
			TargetSystem::Substrate(SubstrateTargetSystem { pallet_index: 11, tree_id: 0 });
		let _r_id: ResourceId = ResourceId::new(target_system, target_id);

		let _root = Element::from_bytes(&[1; 32]);
		let _latest_leaf_index = 5;
		let src_target_system = target_system;
		let src_resource_id = ResourceId::new(src_target_system, src_id);

		let dest_target_system = target_system;
		let dest_resource_id = ResourceId::new(dest_target_system, target_id);

		let raw = include_str!("../firstTransactionInputs.json");
		let inputs_raw: InputsRaw = serde_json::from_str(raw).unwrap();
		let circuit_inputs: RewardCircuitInputs = RewardCircuitInputs::from_raw(&inputs_raw);
		// println!("inputs: {inputs_raw:?}");
		println!("circuitInputs: {circuit_inputs:?}");
		let max_edges = 2u8;
		let depth = 30u8;
		let tree_id = AnonymityMiningClaims::create(None, depth, max_edges, 0u32, 1u32).unwrap();

		let init_call_0 = AnonymityMiningClaims::init_resource_id_history(
			src_resource_id,
			Element::from_bytes(&circuit_inputs.unspent_roots[0].to_bytes_be().1),
			Element::from_bytes(&circuit_inputs.spent_roots[0].to_bytes_be().1),
		);
		assert_ok!(init_call_0);
		let init_call_1 = AnonymityMiningClaims::init_resource_id_history(
			dest_resource_id,
			Element::from_bytes(&circuit_inputs.unspent_roots[1].to_bytes_be().1),
			Element::from_bytes(&circuit_inputs.spent_roots[1].to_bytes_be().1),
		);
		assert_ok!(init_call_1);

		let inputs_for_proof = [
			("rate", circuit_inputs.rate.clone()),
			("fee", circuit_inputs.fee.clone()),
			("rewardNullifier", circuit_inputs.reward_nullifier.clone()),
			("extDataHash", circuit_inputs.ext_data_hash.clone()),
			("noteChainID", circuit_inputs.note_chain_id.clone()),
			("noteAmount", circuit_inputs.note_amount.clone()),
			("noteAssetID", circuit_inputs.note_asset_id.clone()),
			("noteTokenID", circuit_inputs.note_token_id.clone()),
			("note_ak_X", circuit_inputs.note_ak_x.clone()),
			("note_ak_Y", circuit_inputs.note_ak_y.clone()),
			("noteBlinding", circuit_inputs.note_blinding.clone()),
			("notePathElements", circuit_inputs.note_path_elements.clone()),
			("notePathIndices", circuit_inputs.note_path_indices.clone()),
			("note_alpha", circuit_inputs.note_alpha.clone()),
			("note_ak_alpha_X", circuit_inputs.note_ak_alpha_x.clone()),
			("note_ak_alpha_Y", circuit_inputs.note_ak_alpha_y.clone()),
			("inputChainID", circuit_inputs.input_chain_id.clone()),
			("inputAmount", circuit_inputs.input_amount.clone()),
			("inputPrivateKey", circuit_inputs.input_private_key.clone()),
			("inputBlinding", circuit_inputs.input_blinding.clone()),
			("inputNullifier", circuit_inputs.input_nullifier.clone()),
			("inputRoot", circuit_inputs.input_root.clone()),
			("inputPathElements", circuit_inputs.input_path_elements.clone()),
			("inputPathIndices", circuit_inputs.input_path_indices.clone()),
			("outputChainID", circuit_inputs.output_chain_id.clone()),
			("outputAmount", circuit_inputs.output_amount.clone()),
			("outputPrivateKey", circuit_inputs.output_private_key.clone()),
			("outputBlinding", circuit_inputs.output_blinding.clone()),
			("outputCommitment", circuit_inputs.output_commitment.clone()),
			("unspentTimestamp", circuit_inputs.unspent_timestamp.clone()),
			("unspentRoots", circuit_inputs.unspent_roots.clone()),
			("unspentPathIndices", circuit_inputs.unspent_path_indices.clone()),
			("unspentPathElements", circuit_inputs.unspent_path_elements.clone()),
			("spentTimestamp", circuit_inputs.spent_timestamp.clone()),
			("spentRoots", circuit_inputs.spent_roots.clone()),
			("spentPathIndices", circuit_inputs.spent_path_indices.clone()),
			("spentPathElements", circuit_inputs.spent_path_elements),
		];
		let x = generate_proof(wc_2_2, &params_2_2, inputs_for_proof.clone());

		let num_inputs = params_2_2.1.num_instance_variables;

		let (proof, full_assignment) = x.unwrap();

		let mut proof_bytes = Vec::new();
		proof.serialize(&mut proof_bytes).unwrap();

		// let reward_proof_data = RewardProofData::new(proof_bytes, circuit_inputs.rate[0],
		// circuit_inputs.fee[0], circuit_inputs.reward_nullifier[0],
		// circuit_inputs.note_ak_alpha_x[0], circuit_inputs.note_ak_alpha_y[0],
		// circuit_inputs.ext_data_hash[0], circuit_inputs.input_root[0],
		// circuit_inputs.input_nullifier[0], circuit_inputs.output_commitment[0],
		// vec![circuit_inputs.spent_roots[0], circuit_inputs.spent_roots[1]],
		// vec![circuit_inputs.unspent_roots[0], circuit_inputs.unspent_roots[1]]);
		let inputs_for_verification = &full_assignment[1..num_inputs];
		let (
			rate,
			fee,
			reward_nullifier,
			note_ak_alpha_x,
			note_ak_alpha_y,
			ext_data_hash,
			input_root,
			input_nullifier,
			output_commitment,
			unspent_roots,
			spent_roots,
		) = deconstruct_public_inputs_reward_proof_el(max_edges, &inputs_for_verification.to_vec());

		let mut proof_bytes = Vec::new();
		proof.serialize(&mut proof_bytes).unwrap();
		let reward_proof_data = RewardProofData {
			proof: proof_bytes,
			rate,
			fee,
			reward_nullifier,
			note_ak_alpha_x,
			note_ak_alpha_y,
			ext_data_hash,
			input_root,
			input_nullifier,
			output_commitment,
			unspent_roots,
			spent_roots,
		};

		let did_proof_work =
			verify_proof(&params_2_2.0.vk, &proof, inputs_for_verification.to_vec()).unwrap();
		assert!(did_proof_work);

		let src_id = TypedChainId::Substrate(1);
		let target_id = TypedChainId::Substrate(5);
		let target_system =
			TargetSystem::Substrate(SubstrateTargetSystem { pallet_index: 11, tree_id: 0 });
		let _r_id: ResourceId = ResourceId::new(target_system, target_id);

		let _root = Element::from_bytes(&[1; 32]);
		let _latest_leaf_index = 5;
		let src_target_system = target_system;
		let src_resource_id = ResourceId::new(src_target_system, src_id);

		let dest_target_system = target_system;
		let dest_resource_id = ResourceId::new(dest_target_system, target_id);

		let resource_ids = [src_resource_id, dest_resource_id];
		println!("inputs_for_verification: {inputs_for_verification:?}");
		let claim_ap_call =
			AnonymityMiningClaims::claim_ap(tree_id, reward_proof_data, resource_ids.to_vec());
		assert_ok!(claim_ap_call);

		let raw = include_str!("../secondTransactionInputs.json");
		let inputs_raw: InputsRaw = serde_json::from_str(raw).unwrap();
		let circuit_inputs: RewardCircuitInputs = RewardCircuitInputs::from_raw(&inputs_raw);

		let inputs_for_proof = [
			("rate", circuit_inputs.rate.clone()),
			("fee", circuit_inputs.fee.clone()),
			("rewardNullifier", circuit_inputs.reward_nullifier.clone()),
			("extDataHash", circuit_inputs.ext_data_hash.clone()),
			("noteChainID", circuit_inputs.note_chain_id.clone()),
			("noteAmount", circuit_inputs.note_amount.clone()),
			("noteAssetID", circuit_inputs.note_asset_id.clone()),
			("noteTokenID", circuit_inputs.note_token_id.clone()),
			("note_ak_X", circuit_inputs.note_ak_x.clone()),
			("note_ak_Y", circuit_inputs.note_ak_y.clone()),
			("noteBlinding", circuit_inputs.note_blinding.clone()),
			("notePathElements", circuit_inputs.note_path_elements.clone()),
			("notePathIndices", circuit_inputs.note_path_indices.clone()),
			("note_alpha", circuit_inputs.note_alpha.clone()),
			("note_ak_alpha_X", circuit_inputs.note_ak_alpha_x.clone()),
			("note_ak_alpha_Y", circuit_inputs.note_ak_alpha_y.clone()),
			("inputChainID", circuit_inputs.input_chain_id.clone()),
			("inputAmount", circuit_inputs.input_amount.clone()),
			("inputPrivateKey", circuit_inputs.input_private_key.clone()),
			("inputBlinding", circuit_inputs.input_blinding.clone()),
			("inputNullifier", circuit_inputs.input_nullifier.clone()),
			("inputRoot", circuit_inputs.input_root.clone()),
			("inputPathElements", circuit_inputs.input_path_elements.clone()),
			("inputPathIndices", circuit_inputs.input_path_indices.clone()),
			("outputChainID", circuit_inputs.output_chain_id.clone()),
			("outputAmount", circuit_inputs.output_amount.clone()),
			("outputPrivateKey", circuit_inputs.output_private_key.clone()),
			("outputBlinding", circuit_inputs.output_blinding.clone()),
			("outputCommitment", circuit_inputs.output_commitment.clone()),
			("unspentTimestamp", circuit_inputs.unspent_timestamp.clone()),
			("unspentRoots", circuit_inputs.unspent_roots.clone()),
			("unspentPathIndices", circuit_inputs.unspent_path_indices.clone()),
			("unspentPathElements", circuit_inputs.unspent_path_elements.clone()),
			("spentTimestamp", circuit_inputs.spent_timestamp.clone()),
			("spentRoots", circuit_inputs.spent_roots.clone()),
			("spentPathIndices", circuit_inputs.spent_path_indices.clone()),
			("spentPathElements", circuit_inputs.spent_path_elements.clone()),
		];

		let x = generate_proof(wc_2_2, &params_2_2, inputs_for_proof.clone());

		let num_inputs = params_2_2.1.num_instance_variables;

		let (proof, full_assignment) = x.unwrap();

		let mut proof_bytes = Vec::new();
		proof.serialize(&mut proof_bytes).unwrap();

		let inputs_for_verification = &full_assignment[1..num_inputs];
		let (
			rate,
			fee,
			reward_nullifier,
			note_ak_alpha_x,
			note_ak_alpha_y,
			ext_data_hash,
			input_root,
			input_nullifier,
			output_commitment,
			unspent_roots,
			spent_roots,
		) = deconstruct_public_inputs_reward_proof_el(max_edges, &inputs_for_verification.to_vec());
		let reward_proof_data = RewardProofData {
			proof: proof_bytes,
			rate,
			fee,
			reward_nullifier,
			note_ak_alpha_x,
			note_ak_alpha_y,
			ext_data_hash,
			input_root,
			input_nullifier,
			output_commitment,
			unspent_roots,
			spent_roots,
		};

		println!("unspent_root: {:#?}", circuit_inputs.unspent_roots[0].to_bytes_be());
		let unspent_update_0 = AnonymityMiningClaims::update_unspent_root(
			src_resource_id,
			Element::from_bytes(
				&circuit_inputs.unspent_roots[0].to_biguint().unwrap().to_bytes_be(),
			),
		);
		assert_ok!(unspent_update_0);

		let spent_update_0 = AnonymityMiningClaims::update_spent_root(
			src_resource_id,
			Element::from_bytes(&circuit_inputs.spent_roots[0].to_biguint().unwrap().to_bytes_be()),
		);
		assert_ok!(spent_update_0);

		let unspent_update_1 = AnonymityMiningClaims::update_unspent_root(
			dest_resource_id,
			Element::from_bytes(
				&circuit_inputs.unspent_roots[1].to_biguint().unwrap().to_bytes_be(),
			),
		);
		assert_ok!(unspent_update_1);

		let spent_update_1 = AnonymityMiningClaims::update_spent_root(
			dest_resource_id,
			Element::from_bytes(&circuit_inputs.spent_roots[1].to_biguint().unwrap().to_bytes_be()),
		);
		assert_ok!(spent_update_1);

		let did_proof_work =
			verify_proof(&params_2_2.0.vk, &proof, inputs_for_verification.to_vec()).unwrap();
		assert!(did_proof_work);

		let claim_ap_call =
			AnonymityMiningClaims::claim_ap(tree_id, reward_proof_data, resource_ids.to_vec());
		assert_ok!(claim_ap_call);
	});
}
