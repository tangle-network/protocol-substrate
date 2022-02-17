use subxt::{DefaultConfig, DefaultExtra, ClientBuilder, BasicError, PairSigner};
use sp_keyring::AccountKeyring;
use codec::Encode;
use proof::{setup_mixer_circuit, setup_anchor_circuit, verify_unchecked_raw, setup_mixer_leaf, setup_anchor_leaf};

mod proof;
mod utils;

use utils::{truncate_and_pad, expect_event};

#[subxt::subxt(runtime_metadata_path = "metadata/webb_metadata.scale")]
pub mod webb_runtime {}

type WebbRuntimeApi = webb_runtime:: RuntimeApi<DefaultConfig, DefaultExtra<DefaultConfig>>;
use webb_runtime::runtime_types::webb_standalone_runtime::Element;

pub async fn client() -> Result<WebbRuntimeApi, BasicError> {
	let client = ClientBuilder::new()
		.build()
		.await?;
	Ok(client.to_runtime_api())
}

async fn test_mixer() -> Result<(), Box<dyn std::error::Error>> {
    let api = client().await?;

    let mut signer = PairSigner::<DefaultConfig, DefaultExtra<DefaultConfig>, _>::new(AccountKeyring::Alice.pair());

    let pk_bytes = include_bytes!("../../protocol-substrate-fixtures/mixer/bn254/x5/proving_key_uncompressed.bin");
    let vk_bytes = include_bytes!("../../protocol-substrate-fixtures/mixer/bn254/x5/verifying_key_uncompressed.bin");
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
    let deposit_tx = mixer.deposit(tree_id, leaf);
    let mut deposit_res = deposit_tx
        .sign_and_submit_then_watch(&signer)
        .await?;

    expect_event::<webb_runtime::mixer_bn254::events::Deposit>(&mut deposit_res).await?;

    let tree_metadata_res = mt_storage.trees(0, None).await?;
    let leaf_count = tree_metadata_res.unwrap().leaf_count;

    let mut leaves = Vec::new();
    for i in 0..leaf_count {
        let leaf = mt_storage.leaves(0, i, None).await?;
        leaves.push(leaf.0.to_vec());
    }

    println!("Number of leaves in the tree: {:?}", leaves.len());
    println!("Leaf count: {:?}", leaf_count);

    let (proof_bytes, root) = setup_mixer_circuit(leaves, (leaf_count - 1) as u64, secret.0.to_vec(), nullifier.0.to_vec(), recipient_bytes.clone(), relayer_bytes.clone(), pk_bytes.to_vec(), fee, refund);

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

    // Verify the proof locally
    let mut pi = Vec::new();
    pi.push(nullifier_hash.0.to_vec());
    pi.push(root.0.to_vec());
    pi.push(recipient_bytes);
    pi.push(relayer_bytes);
    pi.push(fee.encode());
    pi.push(refund.encode());
    let res = verify_unchecked_raw::<ark_bn254::Bn254>(&pi, &vk_bytes.to_vec(), &proof_bytes)?;
    assert!(res, "Invalid proof");

    // Do the withdraw
    signer.increment_nonce();
    let withdraw_tx = mixer.withdraw(tree_id, proof_bytes, root, nullifier_hash, recipient, relayer, fee, refund);
    let mut withdraw_res = withdraw_tx
        .sign_and_submit_then_watch(&signer)
        .await?;
    
    expect_event::<webb_runtime::mixer_bn254::events::Withdraw>(&mut withdraw_res).await?;

    Ok(())
}

async fn test_anchor() -> Result<(), Box<dyn std::error::Error>> {
    let api = client().await?;

    let mut signer = PairSigner::<DefaultConfig, DefaultExtra<DefaultConfig>, _>::new(AccountKeyring::Alice.pair());

    let pk_bytes = include_bytes!("../../protocol-substrate-fixtures/fixed-anchor/bn254/x5/proving_key_uncompressed.bin");
    let vk_bytes = include_bytes!("../../protocol-substrate-fixtures/fixed-anchor/bn254/x5/verifying_key_uncompressed.bin");
    let recipient = AccountKeyring::Bob.to_account_id();
    let relayer = AccountKeyring::Bob.to_account_id();
    let recipient_bytes = truncate_and_pad(&recipient.encode());
    let relayer_bytes = truncate_and_pad(&relayer.encode());
    let commitment = vec![0u8; 32];
    let chain_id = 2199023256632u128;
    let fee = 0;
    let refund = 0;

    let (leaf, secret, nullifier, nullifier_hash) = setup_anchor_leaf(chain_id);

    // Get the anchor transaction API
    let anchor = api.tx().anchor_bn254();
    // Get the anchor storage API
    let mt_storage = api.storage().merkle_tree_bn254();

    let tree_id = 0;
    let deposit_tx = anchor.deposit(tree_id, leaf);
    let mut deposit_res = deposit_tx
        .sign_and_submit_then_watch(&signer)
        .await?;

    expect_event::<webb_runtime::anchor_bn254::events::Deposit>(&mut deposit_res).await?;

    // let tree_metadata_res = mt_storage.trees(0, None).await?;
    // let leaf_count = tree_metadata_res.unwrap().leaf_count;

    // let mut leaves = Vec::new();
    // for i in 0..leaf_count {
    //     let leaf = mt_storage.leaves(0, i, None).await?;
    //     leaves.push(leaf.0.to_vec());
    // }

    // println!("Number of leaves in the tree: {:?}", leaves.len());
    // println!("Leaf count: {:?}", leaf_count);

    // let tree_metadata_res = mt_storage.trees(0, None).await?;
    // // Fetch the root from chain storage and check if it equals the local root
    // let chain_root = tree_metadata_res.unwrap().root;
    // let zero_root = vec![0u8; 32];
    // let roots = vec![chain_root.0.to_vec(), zero_root];

    // let (proof_bytes, root) = setup_anchor_circuit(roots.clone(), leaves, (leaf_count - 1) as u64, chain_id, secret.0.to_vec(), nullifier.0.to_vec(), recipient_bytes.clone(), relayer_bytes.clone(), pk_bytes.to_vec(), fee, refund, commitment.clone());
    // assert_eq!(chain_root.0, root.0);

    // println!("nullifier_hash: {:?} {}", nullifier_hash.0.to_vec(), nullifier_hash.0.len());
    // println!("root: {:?} {}", root.0.to_vec(), root.0.len());
    // println!("recipient_bytes: {:?} {}", recipient_bytes, recipient_bytes.len());
    // println!("relayer_bytes: {:?} {}", relayer_bytes, relayer_bytes.len());
    // println!("fee_bytes: {:?}", fee.encode());
    // println!("refund_bytes: {:?}", refund.encode());

    // // Verify the proof locally
    // let mut pi = Vec::new();
    // pi.push(chain_id.encode());
    // pi.push(nullifier_hash.0.to_vec());
    // for root in &roots {
    //     pi.push(root.to_vec());
    // }
    // pi.push(recipient_bytes);
    // pi.push(relayer_bytes);
    // pi.push(fee.encode());
    // pi.push(refund.encode());
    // pi.push(commitment);
    // let res = verify_unchecked_raw::<ark_bn254::Bn254>(&pi, &vk_bytes.to_vec(), &proof_bytes)?;
    // assert!(res, "Invalid proof");

    Ok(())
}

#[async_std::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	// test_mixer().await?;
    test_anchor().await?;

    Ok(())
}

