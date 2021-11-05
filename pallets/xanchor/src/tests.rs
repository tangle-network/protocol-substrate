use super::{mock::*, *};

use codec::Encode;
use frame_support::assert_ok;
use xcm::latest::prelude::*;
use xcm_simulator::TestExt;

// Helper function for forming buy execution message
fn buy_execution<C>(fees: impl Into<MultiAsset>) -> Instruction<C> {
	BuyExecution {
		fees: fees.into(),
		weight_limit: Unlimited,
	}
}

// sanity check that XCM is working
#[test]
fn dmp() {
	MockNet::reset();

	let remark =
		parachain::Call::System(frame_system::Call::<parachain::Runtime>::remark_with_event { remark: vec![1, 2, 3] });
	Relay::execute_with(|| {
		assert_ok!(RelayChainPalletXcm::send_xcm(
			Here,
			Parachain(PARAID_A),
			Xcm(vec![Transact {
				origin_type: OriginKind::SovereignAccount,
				require_weight_at_most: INITIAL_BALANCE as u64,
				call: remark.encode().into(),
			}]),
		));
	});

	ParaA::execute_with(|| {
		use parachain::{Event, System};
		assert!(System::events()
			.iter()
			.any(|r| matches!(r.event, Event::System(frame_system::Event::Remarked(_, _)))));
	});
}

#[test]
fn ump() {
	MockNet::reset();

	let remark = relay_chain::Call::System(frame_system::Call::<relay_chain::Runtime>::remark_with_event {
		remark: vec![1, 2, 3],
	});
	ParaA::execute_with(|| {
		assert_ok!(ParachainPalletXcm::send_xcm(
			Here,
			Parent,
			Xcm(vec![Transact {
				origin_type: OriginKind::SovereignAccount,
				require_weight_at_most: INITIAL_BALANCE as u64,
				call: remark.encode().into(),
			}]),
		));
	});

	Relay::execute_with(|| {
		use relay_chain::{Event, System};
		assert!(System::events()
			.iter()
			.any(|r| matches!(r.event, Event::System(frame_system::Event::Remarked(_, _)))));
	});
}

#[test]
fn xcmp() {
	MockNet::reset();

	let remark =
		parachain::Call::System(frame_system::Call::<parachain::Runtime>::remark_with_event { remark: vec![1, 2, 3] });
	ParaA::execute_with(|| {
		assert_ok!(ParachainPalletXcm::send_xcm(
			Here,
			(Parent, Parachain(PARAID_B)),
			Xcm(vec![Transact {
				origin_type: OriginKind::SovereignAccount,
				require_weight_at_most: INITIAL_BALANCE as u64,
				call: remark.encode().into(),
			}]),
		));
	});

	ParaB::execute_with(|| {
		use parachain::{Event, System};
		assert!(System::events()
			.iter()
			.any(|r| matches!(r.event, Event::System(frame_system::Event::Remarked(_, _)))));
	});
}
