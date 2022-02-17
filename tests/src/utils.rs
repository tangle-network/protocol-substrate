use subxt::{DefaultConfig, Event, TransactionProgress};
use core::fmt::Debug;

use super::webb_runtime;
use webb_runtime::runtime_types::sp_runtime::DispatchError;

pub async fn expect_event<E: Event + Debug>(tx_progess: &mut TransactionProgress<'_, DefaultConfig, DispatchError>) -> Result<(), Box<dyn std::error::Error>> {
    while let Some(ev) = tx_progess.next_item().await {
        let ev = ev?;
        use subxt::TransactionStatus::*;

        // Made it into a block, but not finalized.
        if let InBlock(details) = ev {
            println!(
                "Transaction {:?} made it into block {:?}",
                details.extrinsic_hash(),
                details.block_hash()
            );

            let events = details.wait_for_success().await?;
            let transfer_event =
                events.find_first_event::<E>()?;

            if let Some(event) = transfer_event {
                println!(
                    "In block (but not finalized): {event:?}"
                );
            } else {
                println!("Failed to find Event");
            }
        }
        // Finalized!
        else if let Finalized(details) = ev {
            println!(
                "Transaction {:?} is finalized in block {:?}",
                details.extrinsic_hash(),
                details.block_hash()
            );

            let events = details.wait_for_success().await?;
            let transfer_event =
                events.find_first_event::<E>()?;

            if let Some(event) = transfer_event {
                println!("Transaction success: {event:?}");
            } else {
                println!("Failed to find Balances::Transfer Event");
            }
        }
        // Report other statuses we see.
        else {
            println!("Current transaction status: {:?}", ev);
        }
    }

    Ok(())
}

pub fn truncate_and_pad(t: &[u8]) -> Vec<u8> {
	let mut truncated_bytes = t[..20].to_vec();
	truncated_bytes.extend_from_slice(&[0u8; 12]);
	truncated_bytes
}
