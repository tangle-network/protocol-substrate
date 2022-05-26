#![cfg_attr(not(feature = "std"), no_std)]

pub mod hasher;
pub mod runtime;
pub mod signing;
pub mod traits;
pub mod types;
pub mod utils;
pub mod verifier;

#[cfg(feature = "hashing")]
pub mod hashing;

#[cfg(feature = "verifying")]
pub mod verifying;

#[cfg(feature = "field_ops")]
pub mod field_ops;

pub use hasher::*;
pub use runtime::*;
pub use traits::*;
pub use types::*;
pub use verifier::*;

pub use runtime::*;

pub use webb_proposals;

/// Opaque types. These are used by the CLI to instantiate machinery that don't
/// need to know the specifics of the runtime. They can then be made to be
/// agnostic over specific formats of data like extrinsics, allowing for them to
/// continue syncing the network through upgrades to even the core data
/// structures.
pub mod opaque {
	use super::*;
	use sp_runtime::{generic, traits::BlakeTwo256};

	pub use sp_runtime::OpaqueExtrinsic as UncheckedExtrinsic;
	/// Opaque block header type.
	pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
	/// Opaque block type.
	pub type Block = generic::Block<Header, UncheckedExtrinsic>;
	/// Opaque block identifier type.
	pub type BlockId = generic::BlockId<Block>;
}
