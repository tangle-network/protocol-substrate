use ark_ff::ToBytes;
use ark_groth16::{ProvingKey};
use ark_relations::r1cs::ConstraintMatrices;
use sp_keyring::AccountKeyring;

use webb_client::webb_runtime;
use webb_primitives::ElementTrait;

mod utils;

use codec::Encode;
use utils::*;
use webb_primitives::{hashing::ethereum::keccak_256, utils::compute_chain_id_type, IntoAbiToken};

use ark_bn254::{Bn254, Fr as Bn254Fr};
use arkworks_native_gadgets::ark_std::rand::rngs::OsRng;
use arkworks_setups::{
	common::{verify_unchecked_raw},
	utxo::Utxo,
};
use subxt::{
	ext::sp_runtime::AccountId32,
	tx::{PairSigner, TxProgress},
	OnlineClient, SubstrateConfig,
};
use utils::ExtData;

use ark_circom::{read_zkey, WitnessCalculator};
use ark_ff::{BigInteger, PrimeField};

use std::{fs::File, sync::Mutex};

#[tokio::test]
#[ignore]
async fn test_mixer() -> Result<(), Box<dyn std::error::Error>> {
	let api: OnlineClient<_> = OnlineClient::<SubstrateConfig>::new().await?;
	let signer = PairSigner::new(AccountKeyring::Alice.pair());

	let pk_bytes = include_bytes!(
		"../../substrate-fixtures/substrate-fixtures/mixer/bn254/x5/proving_key_uncompressed.bin"
	);
	let vk_bytes = include_bytes!(
		"../../substrate-fixtures/substrate-fixtures/mixer/bn254/x5/verifying_key_uncompressed.bin"
	);
	let recipient = AccountKeyring::Bob.to_account_id();
	let relayer = AccountKeyring::Bob.to_account_id();
	let recipient_bytes = truncate_and_pad(&recipient.encode());
	let relayer_bytes = truncate_and_pad(&relayer.encode());
	let fee = 0;
	let refund = 0;

	let (leaf, secret, nullifier, nullifier_hash) = setup_mixer_leaf();

	// Get the mixer transaction API
	let mixer = webb_runtime::tx().mixer_bn254();
	// Get the mixer storage API
	let mt_storage = webb_runtime::storage().merkle_tree_bn254();

	let tree_id = 0;
	let deposit_tx = mixer.deposit(tree_id, leaf.into());
	let mut deposit_res: TxProgress<_, _> =
		api.tx().sign_and_submit_then_watch_default(&deposit_tx, &signer).await?;

	expect_event::<
		webb_runtime::mixer_bn254::events::Deposit,
		SubstrateConfig,
		OnlineClient<SubstrateConfig>,
	>(&mut deposit_res)
	.await?;

	let tree_metadata_storage_key = mt_storage.trees(tree_id);
	let tree_metadata_res = api.storage().fetch(&tree_metadata_storage_key, None).await?;
	let leaf_count = tree_metadata_res.unwrap().leaf_count;

	let mut leaves = Vec::new();
	for i in 0..leaf_count {
		let leaf_storage_key = mt_storage.leaves(tree_id, i);
		let leaf = api.storage().fetch(&leaf_storage_key, None).await?.unwrap();
		leaves.push(leaf.0.to_vec());
	}

	println!("Number of leaves in the tree: {:?}", leaves.len());
	println!("Leaf count: {leaf_count:?}");
	let mut rng = OsRng {};

	let (proof_bytes, root) = create_mixer_proof(
		leaves,
		(leaf_count - 1) as u64,
		secret.0.to_vec(),
		nullifier.0.to_vec(),
		recipient_bytes.clone(),
		relayer_bytes.clone(),
		fee,
		refund,
		pk_bytes.to_vec(),
		&mut rng,
	);

	// Fetch the root from chain storage and check if it equals the local root
	let tree_metadata_storage_key = mt_storage.trees(0);
	let tree_metadata_res = api.storage().fetch(&tree_metadata_storage_key, None).await?;
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

	let res = verify_unchecked_raw::<ark_bn254::Bn254>(&pi, vk_bytes.as_ref(), &proof_bytes)?;
	assert!(res, "Invalid proof");

	// Do the withdraw
	let withdraw_tx = mixer.withdraw(
		tree_id,
		proof_bytes,
		root.into(),
		nullifier_hash.into(),
		(*<subxt::ext::sp_runtime::AccountId32 as AsRef<[u8; 32]>>::as_ref(&recipient)).into(),
		(*<subxt::ext::sp_runtime::AccountId32 as AsRef<[u8; 32]>>::as_ref(&relayer)).into(),
		fee,
		refund,
	);
	let mut withdraw_res =
		api.tx().sign_and_submit_then_watch_default(&withdraw_tx, &signer).await?;

	expect_event::<
		webb_runtime::mixer_bn254::events::Withdraw,
		SubstrateConfig,
		OnlineClient<SubstrateConfig>,
	>(&mut withdraw_res)
	.await?;

	Ok(())
}

async fn make_vanchor_tx(
	circom_params: &(ProvingKey<Bn254>, ConstraintMatrices<Bn254Fr>),
	#[cfg(not(target_arch = "wasm32"))] wc: &Mutex<WitnessCalculator>,
	#[cfg(target_arch = "wasm32")] wc: &mut WitnessCalculator,
	recipient: &AccountId32,
	relayer: &AccountId32,
	public_amount: i128,
	custom_roots: Option<[Vec<u8>; 2]>,
	leaves: Vec<Vec<u8>>,
	in_utxos: [Utxo<Bn254Fr>; 2],
	out_utxos: [Utxo<Bn254Fr>; 2],
) -> Result<(), Box<dyn std::error::Error>> {
	let api = OnlineClient::<SubstrateConfig>::new().await?;
	let signer = PairSigner::new(AccountKeyring::Alice.pair());

	let chain_type = [2, 0];
	let chain_id = compute_chain_id_type(1080u32, chain_type);
	let ext_amount = public_amount;
	let fee = 0u128;
	let refund = 0u128;
	let token = u32::MAX - 1;

	let output1: [u8; 32] = out_utxos[0].commitment.into_repr().to_bytes_be().try_into().unwrap();
	let output2: [u8; 32] = out_utxos[1].commitment.into_repr().to_bytes_be().try_into().unwrap();
	let ext_data = ExtData::new(
		recipient.clone(),
		relayer.clone(),
		ext_amount,
		fee,
		refund,
		token,
		Element(output1).to_vec(),
		Element(output2).to_vec(),
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
		circom_params,
		wc,
	);

	let res = verify_proof(&circom_params.0.vk, &proof, &public_inputs);
	assert!(res.unwrap(), "Invalid proof");
	println!("proof verified");

	// Deconstructing public inputs
	let (_chain_id, public_amount, root_set, nullifiers, commitments, ext_data_hash) =
		deconstruct_vanchor_pi_el(&public_inputs);

	println!("chain id {chain_id:?}");
	println!("public amount {public_amount:?}");
	println!("root set {root_set:?}");
	println!("nullifiers {nullifiers:?}");
	println!("commitments {commitments:?}");
	println!("ext data hash {ext_data_hash:?}");

	let mut proof_vec = Vec::new();
	let _ = &proof.write(&mut proof_vec).unwrap();

	// Constructing proof data
	let proof_data =
		ProofData::new(proof_vec, public_amount, root_set, nullifiers, commitments, ext_data_hash);

	println!("my name is: {:?}", &proof_data.roots.len());
	// mixer = 0..3
	// anchor = 3..6
	// vanchor = 6
	let tree_id = 5;
	// Get the vanchor transaction API
	let vanchor = webb_runtime::tx().v_anchor_bn254();

	let transact_tx = vanchor.transact(tree_id, proof_data.into(), ext_data.into());

	let mut transact_res =
		api.tx().sign_and_submit_then_watch_default(&transact_tx, &signer).await?;

	expect_event::<
		webb_runtime::v_anchor_bn254::events::Transaction,
		SubstrateConfig,
		OnlineClient<SubstrateConfig>,
	>(&mut transact_res)
	.await?;

	Ok(())
}

#[tokio::test]
async fn test_vanchor() -> Result<(), Box<dyn std::error::Error>> {
	let api: OnlineClient<_> = OnlineClient::<SubstrateConfig>::new().await?;

	let path_2_2 = "../solidity-fixtures/solidity-fixtures/vanchor_2/2/circuit_final.zkey";
	let mut file_2_2 = File::open(path_2_2).unwrap();
	let params_2_2 = read_zkey(&mut file_2_2).unwrap();

	let wasm_2_2_path =
		"../solidity-fixtures/solidity-fixtures//vanchor_2/2/poseidon_vanchor_2_2.wasm";

	let wc_2_2 = circom_from_folder(wasm_2_2_path);

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

	let custom_roots = Some([[0u8; 32]; 2].map(|x| x.to_vec()));
	let leaf0 = in_utxos[0].commitment.into_repr().to_bytes_be();
	let leaf1 = in_utxos[1].commitment.into_repr().to_bytes_be();
	let leaves: Vec<Vec<u8>> = vec![leaf0, leaf1];

	make_vanchor_tx(
		&params_2_2,
		wc_2_2,
		&(*AsRef::<[u8; 32]>::as_ref(&recipient)).into(),
		&(*AsRef::<[u8; 32]>::as_ref(&relayer)).into(),
		public_amount,
		custom_roots,
		leaves,
		in_utxos,
		out_utxos.clone(),
	)
	.await?;

	// Get the vanchor storage API
	let mt_storage = webb_runtime::storage().merkle_tree_bn254();

	let tree_id = 5;
	let tree_metadata_storage_key = mt_storage.trees(tree_id);
	let tree_metadata_res = api.storage().fetch(&tree_metadata_storage_key, None).await?;
	let tree_metadata = tree_metadata_res.unwrap();
	let leaf_count = tree_metadata.leaf_count;
	let chain_root = tree_metadata.root;

	let mut leaves = Vec::new();
	for i in 0..leaf_count {
		let leaf_storage_key = mt_storage.leaves(tree_id, i);
		let leaf = api.storage().fetch(&leaf_storage_key, None).await?.unwrap();
		leaves.push(leaf.0.to_vec());
	}

	let out_indices = [leaves.len() - 1, leaves.len() - 2];

	for (i, utxo) in out_utxos.iter_mut().enumerate() {
		utxo.set_index(out_indices[i] as u64);
	}

	let new_amount = amount / 2;
	let public_amount = -((amount / 2) as i128);
	let new_out_amounts = [new_amount, 0];
	// Input utxo is now the old out utxo
	let in_utxo = out_utxos.clone();

	// Output utxo is brand new now
	let out_utxos = setup_utxos(out_chain_ids, new_out_amounts, None);

	make_vanchor_tx(
		&params_2_2,
		wc_2_2,
		&(*AsRef::<[u8; 32]>::as_ref(&recipient)).into(),
		&(*AsRef::<[u8; 32]>::as_ref(&relayer)).into(),
		public_amount,
		None,
		leaves.clone(),
		in_utxo,
		out_utxos,
	)
	.await?;

	println!("Number of leaves in the tree: {:?}", leaves.len());
	println!("Leaf count: {leaf_count:?}");
	println!("Chain root {chain_root:?}");

	Ok(())
}
