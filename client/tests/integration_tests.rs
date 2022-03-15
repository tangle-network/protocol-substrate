use wasm_utils::ANCHOR_COUNT;
use webb_client::{self, client, webb_runtime, WebbRuntimeApi};

use sp_keyring::AccountKeyring;
use subxt::{sp_core::sr25519::Pair, DefaultConfig, DefaultExtra, PairSigner};

mod utils;

use codec::Encode;
use utils::*;
use webb_primitives::{hashing::ethereum::keccak_256, utils::compute_chain_id_type, IntoAbiToken};

use ark_bn254::Fr as Bn254Fr;
use arkworks_native_gadgets::poseidon::Poseidon;
use arkworks_setups::{
	common::{setup_params, verify_unchecked, verify_unchecked_raw},
	utxo::Utxo,
	Curve,
};
use subxt::sp_runtime::AccountId32;

use ark_ff::{BigInteger, PrimeField};

#[tokio::test]
async fn test_mixer() -> Result<(), Box<dyn std::error::Error>> {
	let api = client().await?;

	let signer = PairSigner::<DefaultConfig, DefaultExtra<DefaultConfig>, _>::new(
		AccountKeyring::Alice.pair(),
	);

	let pk_bytes = include_bytes!(
		"../../protocol-substrate-fixtures/mixer/bn254/x5/proving_key_uncompressed.bin"
	);
	let vk_bytes = include_bytes!(
		"../../protocol-substrate-fixtures/mixer/bn254/x5/verifying_key_uncompressed.bin"
	);
	let recipient = AccountKeyring::Bob.to_account_id();
	let relayer = AccountKeyring::Bob.to_account_id();
	let recipient_bytes = truncate_and_pad(&recipient.encode());
	let relayer_bytes = truncate_and_pad(&relayer.encode());
	let fee = 0;
	let refund = 0;

	let (leaf, secret, nullifier, nullifier_hash) = setup_mixer_leaf();

	// Get the mixer transaction API
	let mixer = api.tx().mixer_bn254();
	// Get the mixer storage API
	let mt_storage = api.storage().merkle_tree_bn254();

	let tree_id = 0;
	let deposit_tx = mixer.deposit(tree_id, leaf.into());
	let mut deposit_res = deposit_tx.sign_and_submit_then_watch(&signer).await?;

	expect_event::<webb_runtime::mixer_bn254::events::Deposit>(&mut deposit_res).await?;

	let tree_metadata_res = mt_storage.trees(tree_id, None).await?;
	let leaf_count = tree_metadata_res.unwrap().leaf_count;

	let mut leaves = Vec::new();
	for i in 0..leaf_count {
		let leaf = mt_storage.leaves(tree_id, i, None).await?;
		leaves.push(leaf.0.to_vec());
	}

	println!("Number of leaves in the tree: {:?}", leaves.len());
	println!("Leaf count: {:?}", leaf_count);

	let (proof_bytes, root) = setup_mixer_circuit(
		leaves,
		(leaf_count - 1) as u64,
		secret.0.to_vec(),
		nullifier.0.to_vec(),
		recipient_bytes.clone(),
		relayer_bytes.clone(),
		fee,
		refund,
		pk_bytes.to_vec(),
	);

	// Fetch the root from chain storage and check if it equals the local root
	let tree_metadata_res = mt_storage.trees(0, None).await?;
	if let Some(tree_metadata) = tree_metadata_res {
		let chain_root = tree_metadata.root;
		assert_eq!(chain_root.0, root.0);
	}

	println!("nullifier_hash: {:?} {}", nullifier_hash.0.to_vec(), nullifier_hash.0.len());
	println!("root: {:?} {}", root.0.to_vec(), root.0.len());
	println!("recipient_bytes: {:?} {}", recipient_bytes, recipient_bytes.len());
	println!("relayer_bytes: {:?} {}", relayer_bytes, relayer_bytes.len());
	println!("fee_bytes: {:?}", fee.encode());
	println!("refund_bytes: {:?}", refund.encode());

	let mut aribtrary_bytes = Vec::new();
	aribtrary_bytes.extend(recipient_bytes);
	aribtrary_bytes.extend(relayer_bytes);
	aribtrary_bytes.extend(fee.encode());
	aribtrary_bytes.extend(refund.encode());
	let arbitrary_input = keccak_256(&aribtrary_bytes);

	// Verify the proof locally
	let mut pi = Vec::new();
	pi.push(nullifier_hash.0.to_vec());
	pi.push(root.0.to_vec());
	pi.push(arbitrary_input.to_vec());

	let res = verify_unchecked_raw::<ark_bn254::Bn254>(&pi, &vk_bytes.to_vec(), &proof_bytes)?;
	assert!(res, "Invalid proof");

	// Do the withdraw
	let withdraw_tx = mixer.withdraw(
		tree_id,
		proof_bytes,
		root.into(),
		nullifier_hash.into(),
		recipient,
		relayer,
		fee,
		refund,
	);
	let mut withdraw_res = withdraw_tx.sign_and_submit_then_watch(&signer).await?;

	expect_event::<webb_runtime::mixer_bn254::events::Withdraw>(&mut withdraw_res).await?;

	Ok(())
}

#[tokio::test]
async fn test_anchor() -> Result<(), Box<dyn std::error::Error>> {
	let api = client().await?;

	let signer = PairSigner::<DefaultConfig, DefaultExtra<DefaultConfig>, _>::new(
		AccountKeyring::Alice.pair(),
	);

	let pk_bytes = include_bytes!(
		"../../protocol-substrate-fixtures/fixed-anchor/bn254/x5/proving_key_uncompressed.bin"
	);
	let vk_bytes = include_bytes!(
		"../../protocol-substrate-fixtures/fixed-anchor/bn254/x5/verifying_key_uncompressed.bin"
	);
	let recipient = AccountKeyring::Bob.to_account_id();
	let relayer = AccountKeyring::Bob.to_account_id();
	let recipient_bytes = truncate_and_pad(&recipient.encode());
	let relayer_bytes = truncate_and_pad(&relayer.encode());
	let commitment = Element([0u8; 32]);
	let chain_id = 2199023256632u64;
	let fee = 0;
	let refund = 0;

	let (leaf, secret, nullifier, nullifier_hash) = setup_anchor_leaf(chain_id);

	// Get the anchor transaction API
	let anchor = api.tx().anchor_bn254();
	// Get the anchor storage API
	let mt_storage = api.storage().merkle_tree_bn254();

	let tree_id = 4;
	let deposit_tx = anchor.deposit(tree_id, leaf.into());
	let mut deposit_res = deposit_tx.sign_and_submit_then_watch(&signer).await?;

	expect_event::<webb_runtime::anchor_bn254::events::Deposit>(&mut deposit_res).await?;

	let tree_metadata_res = mt_storage.trees(tree_id, None).await?;
	let tree_metadata = tree_metadata_res.unwrap();
	let leaf_count = tree_metadata.leaf_count;
	let chain_root = tree_metadata.root;

	let mut leaves = Vec::new();
	for i in 0..leaf_count {
		let leaf = mt_storage.leaves(tree_id, i, None).await?;
		leaves.push(leaf.0.to_vec());
	}

	println!("Number of leaves in the tree: {:?}", leaves.len());
	println!("Leaf count: {:?}", leaf_count);

	let zero_root = Element([0u8; 32]);
	let root_elemets = vec![chain_root, zero_root.into()];
	let roots: Vec<Vec<u8>> = root_elemets.iter().map(|x| x.0.to_vec()).collect();

	let (proof_bytes, root) = setup_anchor_circuit(
		roots.clone(),
		leaves,
		(leaf_count - 1) as u64,
		chain_id,
		secret.0.to_vec(),
		nullifier.0.to_vec(),
		recipient_bytes.clone(),
		relayer_bytes.clone(),
		fee,
		refund,
		commitment.0.to_vec(),
		pk_bytes.to_vec(),
	);

	println!("nullifier_hash: {:?} {}", nullifier_hash.0.to_vec(), nullifier_hash.0.len());
	println!("root: {:?} {}", root.0.to_vec(), root.0.len());
	println!("recipient_bytes: {:?} {}", recipient_bytes, recipient_bytes.len());
	println!("relayer_bytes: {:?} {}", relayer_bytes, relayer_bytes.len());
	println!("fee_bytes: {:?}", fee.encode());
	println!("refund_bytes: {:?}", refund.encode());

	let mut aribtrary_bytes = Vec::new();
	aribtrary_bytes.extend(recipient_bytes);
	aribtrary_bytes.extend(relayer_bytes);
	aribtrary_bytes.extend(fee.encode());
	aribtrary_bytes.extend(refund.encode());
	aribtrary_bytes.extend(commitment.0.to_vec());
	let arbitrary_input = keccak_256(&aribtrary_bytes);

	// Verify the proof locally
	let mut pi = Vec::new();
	pi.push(nullifier_hash.0.to_vec());
	pi.push(arbitrary_input.to_vec());
	pi.push(chain_id.encode());
	for root in &roots {
		pi.push(root.to_vec());
	}
	let res = verify_unchecked_raw::<ark_bn254::Bn254>(&pi, &vk_bytes.to_vec(), &proof_bytes)?;
	assert!(res, "Invalid proof");

	// Do the withdraw
	let withdraw_tx = anchor.withdraw(
		tree_id,
		proof_bytes,
		root_elemets,
		nullifier_hash.into(),
		recipient,
		relayer,
		fee,
		refund,
		commitment.into(),
	);
	let mut withdraw_res = withdraw_tx.sign_and_submit_then_watch(&signer).await?;

	expect_event::<webb_runtime::anchor_bn254::events::Withdraw>(&mut withdraw_res).await?;

	Ok(())
}

async fn make_vanchor_tx(
	api: &WebbRuntimeApi,
	signer: &PairSigner<DefaultConfig, DefaultExtra<DefaultConfig>, Pair>,
	pk_bytes: &[u8],
	vk_bytes: &[u8],
	recipient: &AccountId32,
	relayer: &AccountId32,
	public_amount: i128,
	custom_roots: Option<[Vec<u8>; 2]>,
	leaves: Vec<Vec<u8>>,
	in_utxos: [Utxo<Bn254Fr>; 2],
	out_utxos: [Utxo<Bn254Fr>; 2],
) -> Result<(), Box<dyn std::error::Error>> {
	let chain_type = [2, 0];
	let chain_id = compute_chain_id_type(1080u32, chain_type);
	let ext_amount = public_amount;
	let fee = 0u128;

	let output1: [u8; 32] = out_utxos[0].commitment.into_repr().to_bytes_le().try_into().unwrap();
	let output2: [u8; 32] = out_utxos[1].commitment.into_repr().to_bytes_le().try_into().unwrap();
	let ext_data = ExtData::new(
		recipient.clone(),
		relayer.clone(),
		ext_amount,
		fee,
		Element(output1),
		Element(output2),
	);

	let ext_data_hash = keccak_256(&ext_data.encode_abi());

	let (proof, public_inputs) = setup_vanchor_circuit(
		public_amount,
		chain_id,
		ext_data_hash.to_vec(),
		in_utxos,
		out_utxos.clone(),
		custom_roots,
		leaves,
		pk_bytes.to_vec(),
	);

	let res = verify_unchecked::<ark_bn254::Bn254>(&public_inputs, &vk_bytes.to_vec(), &proof)?;
	assert!(res, "Invalid proof");

	// Deconstructing public inputs
	let (_chain_id, public_amount, root_set, nullifiers, commitments, ext_data_hash) =
		deconstruct_vanchor_pi_el(&public_inputs);

	println!("chain id {:?}", chain_id);
	println!("public amount {:?}", public_amount);
	println!("root set {:?}", root_set);
	println!("nullifiers {:?}", nullifiers);
	println!("commitments {:?}", commitments);
	println!("ext data hash {:?}", ext_data_hash);

	// Constructing proof data
	let proof_data =
		ProofData::new(proof, public_amount, root_set, nullifiers, commitments, ext_data_hash);

	// mixer = 0..3
	// anchor = 3..6
	// vanchor = 6
	let tree_id = 6;
	// Get the vanchor transaction API
	let vanchor = api.tx().v_anchor_bn254();

	let transact_tx = vanchor.transact(tree_id, proof_data.into(), ext_data.into());

	let mut transact_res = transact_tx.sign_and_submit_then_watch(signer).await?;

	expect_event::<webb_runtime::v_anchor_bn254::events::Transaction>(&mut transact_res).await?;

	Ok(())
}

#[tokio::test]
async fn test_vanchor() -> Result<(), Box<dyn std::error::Error>> {
	let api = client().await?;

	let signer = PairSigner::<DefaultConfig, DefaultExtra<DefaultConfig>, _>::new(
		AccountKeyring::Alice.pair(),
	);

	let pk_bytes = include_bytes!(
		"../../protocol-substrate-fixtures/vanchor/bn254/x5/proving_key_uncompressed.bin"
	);
	let vk_bytes = include_bytes!(
		"../../protocol-substrate-fixtures/vanchor/bn254/x5/verifying_key_uncompressed.bin"
	);

	let params4 = setup_params::<Bn254Fr>(Curve::Bn254, 5, 4);
	let poseidon4 = Poseidon::new(params4);

	let recipient = AccountKeyring::Bob.to_account_id();
	let relayer = AccountKeyring::Bob.to_account_id();

	let amount = 100000000;
	let public_amount = amount as i128;

	let chain_type = [2, 0];
	let chain_id = compute_chain_id_type(1080u32, chain_type);
	let in_chain_ids = [chain_id; 2];
	let in_amounts = [0, 0];
	let out_chain_ids = [chain_id; 2];

	let out_amounts = [amount, 0];

	let in_utxos = setup_utxos(in_chain_ids, in_amounts, Some([0, 1]));
	let mut out_utxos = setup_utxos(out_chain_ids, out_amounts, None);

	let custom_roots = Some([[0u8; 32]; ANCHOR_COUNT].map(|x| x.to_vec()));
	let leaf0 = in_utxos[0].commitment.into_repr().to_bytes_le();
	let leaf1 = in_utxos[1].commitment.into_repr().to_bytes_le();
	let leaves: Vec<Vec<u8>> = vec![leaf0, leaf1];

	make_vanchor_tx(
		&api,
		&signer,
		pk_bytes,
		vk_bytes,
		&recipient,
		&relayer,
		public_amount,
		custom_roots,
		leaves,
		in_utxos,
		out_utxos.clone(),
	)
	.await?;

	// Get the vanchor storage API
	let mt_storage = api.storage().merkle_tree_bn254();

	let tree_id = 6;
	let tree_metadata_res = mt_storage.trees(tree_id, None).await?;
	let tree_metadata = tree_metadata_res.unwrap();
	let leaf_count = tree_metadata.leaf_count;
	let chain_root = tree_metadata.root;

	let mut leaves = Vec::new();
	for i in 0..leaf_count {
		let leaf = mt_storage.leaves(tree_id, i, None).await?;
		leaves.push(leaf.0.to_vec());
	}

	let out_indices = [leaves.len() - 1, leaves.len() - 2];

	for (i, utxo) in out_utxos.iter_mut().enumerate() {
		utxo.set_index(out_indices[i] as u64, &poseidon4).unwrap();
	}

	let new_amount = amount / 2;
	let public_amount = -((amount / 2) as i128);
	let new_out_amounts = [new_amount, 0];
	// Input utxo is now the old out utxo
	let in_utxo = out_utxos.clone();

	// Output utxo is brand new now
	let out_utxos = setup_utxos(out_chain_ids, new_out_amounts, None);

	make_vanchor_tx(
		&api,
		&signer,
		pk_bytes,
		vk_bytes,
		&recipient,
		&relayer,
		public_amount,
		None,
		leaves.clone(),
		in_utxo,
		out_utxos,
	)
	.await?;

	println!("Number of leaves in the tree: {:?}", leaves.len());
	println!("Leaf count: {:?}", leaf_count);
	println!("Chain root {:?}", chain_root);

	Ok(())
}
