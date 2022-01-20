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
fn should_create_anchor_with_sig_suceed() {
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
			// Create proposal (& vote)
			assert_ok!(SignatureBridge::execute_proposal(
				Origin::signed(RELAYER_A),
				prop_id,
				src_id,
				r_id,
				Box::new(proposal.clone()),
				sig.0.to_vec(),
			));
		},
	)
}
