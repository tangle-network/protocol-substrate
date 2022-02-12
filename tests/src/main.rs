use subxt::{DefaultConfig, DefaultExtra, ClientBuilder, BasicError, PairSigner, TransactionStatus};
use sp_keyring::AccountKeyring;
mod proof;
use codec::Encode;

use proof::{setup_wasm_utils_zk_circuit, verify_unchecked_raw};

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

#[async_std::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	let api = client().await?;

    let mut signer = PairSigner::<DefaultConfig, DefaultExtra<DefaultConfig>, _>::new(AccountKeyring::Alice.pair());

    let pk_bytes = include_bytes!("../../protocol-substrate-fixtures/fixed-anchor/bn254/x5/proving_key_uncompressed.bin");
    let recipient = AccountKeyring::Bob.to_account_id();
    let relayer = AccountKeyring::Bob.to_account_id();
    let fee = 0;
    let refund = 0;

    let (proof_bytes, root, nullifier_hash, leaf) = setup_wasm_utils_zk_circuit(recipient.encode(), relayer.encode(), pk_bytes.to_vec(), fee, refund);
    
    let mixer = api.tx().mixer_bn254();

    let tree_id = 0;
    let deposit_tx = mixer.deposit(tree_id, leaf);
    let mut deposit_res = deposit_tx
        .sign_and_submit_then_watch(&signer)
        .await?;

    while let Some(status) = deposit_res.next_item().await {
        println!("{:?}", status);
    };

    signer.increment_nonce();
    let withdraw_tx = mixer.withdraw(tree_id, proof_bytes, root, nullifier_hash, recipient, relayer, fee, refund);
    let mut withdraw_res = withdraw_tx
        .sign_and_submit_then_watch(&signer)
        .await?;
    
    while let Some(status) = withdraw_res.next_item().await {
        match status {
            Ok(TransactionStatus::InBlock(tx)) => {
                let events = tx.fetch_events().await?;

                println!("{:?}", events);
            }
            e => println!("{:?}", e),
        }
    };

    Ok(())
}
