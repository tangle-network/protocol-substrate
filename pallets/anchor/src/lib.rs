// This file is part of Webb.

// Copyright (C) 2021 Webb Technologies Inc.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! # Anchor Module
//!
//! A simple module for building Anchors.
//!
//! ## Overview
//!
//! The Anchor module provides functionality for the following:
//!
//! * Inserting elements to the tree
//!
//! The supported dispatchable functions are documented in the [`Call`] enum.
//!
//! ### Terminology
//!
//! * **Anchor**: Connected instances that contains an on-chain merkle tree and tracks a set of
//!   connected __anchors__ across chains (through edges) in its local storage.
//! * **Edge**: An edge is a directed connection or link between two anchors.
//!
//! ## Interface
//!
//! ### Permissioned Functions
//!
//! * `create`: Creates an anchor. This can be only called by the Root.
//!
//! ### Permissionless Functions
//!
//! * `deposit`: Inserts elements into on-chain merkle tree.
//! * `deposit_and_update_linked_anchors`: Same as [Self::deposit] but with another call to update
//!   the linked anchors cross-chain (if any).
//! * `withdraw`: Withdraw requires a zero-knowledge proof of a unspent deposit in some anchors’
//!   merkle tree on either this chain or a neighboring chain
//!
//! ## Related Modules
//!
//! * Linkable-tree pallet

// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::type_complexity, clippy::too_many_arguments)]
#[cfg(test)]
pub mod mock;
#[cfg(test)]
mod test_utils;
#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod zk_config;

mod benchmarking;

pub mod types;
pub mod weights;
use codec::Encode;
use frame_support::{dispatch::DispatchResult, ensure, pallet_prelude::DispatchError, traits::Get};
use orml_traits::{currency::transactional, MultiCurrency};
use sp_runtime::traits::AccountIdConversion;
use sp_std::prelude::*;
use types::*;
use webb_primitives::{
	anchor::{AnchorConfig, AnchorInspector, AnchorInterface},
	hasher::InstanceHasher,
	linkable_tree::{LinkableTreeInspector, LinkableTreeInterface},
	verifier::*,
	ElementTrait,
};
pub use weights::WeightInfo;

/// Type alias for the orml_traits::MultiCurrency::Balance type
pub type BalanceOf<T, I> =
	<<T as Config<I>>::Currency as MultiCurrency<<T as frame_system::Config>::AccountId>>::Balance;
/// Type alias for the orml_traits::MultiCurrency::CurrencyId type
pub type CurrencyIdOf<T, I> = <<T as pallet::Config<I>>::Currency as MultiCurrency<
	<T as frame_system::Config>::AccountId,
>>::CurrencyId;

pub use pallet::*;
#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::{dispatch::DispatchResultWithPostInfo, pallet_prelude::*, PalletId};
	use frame_system::pallet_prelude::*;

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	#[pallet::without_storage_info]
	pub struct Pallet<T, I = ()>(_);

	#[pallet::config]
	/// The module configuration trait.
	pub trait Config<I: 'static = ()>:
		frame_system::Config + pallet_linkable_tree::Config<I>
	{
		/// The overarching event type.
		type Event: From<Event<Self, I>> + IsType<<Self as frame_system::Config>::Event>;

		#[pallet::constant]
		type PalletId: Get<PalletId>;

		/// The tree type
		type LinkableTree: LinkableTreeInterface<pallet_linkable_tree::LinkableTreeConfigration<Self, I>>
			+ LinkableTreeInspector<pallet_linkable_tree::LinkableTreeConfigration<Self, I>>;

		/// The verifier
		type Verifier: VerifierModule;

		/// Arbitrary data hasher
		type ArbitraryHasher: InstanceHasher;

		/// Currency type for taking deposits
		type Currency: MultiCurrency<Self::AccountId>;

		type PostDepositHook: PostDepositHook<Self, I>;

		/// Native currency id
		#[pallet::constant]
		type NativeCurrencyId: Get<CurrencyIdOf<Self, I>>;

		/// Weight info for pallet
		type WeightInfo: WeightInfo;
	}

	/// The map of trees to their anchor metadata
	#[pallet::storage]
	#[pallet::getter(fn anchors)]
	pub type Anchors<T: Config<I>, I: 'static = ()> = StorageMap<
		_,
		Blake2_128Concat,
		T::TreeId,
		AnchorMetadata<BalanceOf<T, I>, CurrencyIdOf<T, I>>,
		OptionQuery,
	>;

	/// The map of trees to their spent nullifier hashes
	#[pallet::storage]
	#[pallet::getter(fn nullifier_hashes)]
	pub type NullifierHashes<T: Config<I>, I: 'static = ()> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		T::TreeId,
		Blake2_128Concat,
		T::Element,
		bool,
		ValueQuery,
	>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config<I>, I: 'static = ()> {
		/// New tree created
		AnchorCreation { tree_id: T::TreeId },
		/// Amount has been withdrawn from the anchor
		Withdraw { who: T::AccountId, amount: BalanceOf<T, I> },
		/// A transaction has been refreshed (one spent, another inserted)
		Refresh { tree_id: T::TreeId, leaf: T::Element },
		/// Amount has been deposited into the anchor
		Deposit {
			depositor: T::AccountId,
			tree_id: T::TreeId,
			leaf: T::Element,
			amount: BalanceOf<T, I>,
		},
		/// Post deposit hook has executed successfully
		PostDeposit { depositor: T::AccountId, tree_id: T::TreeId, leaf: T::Element },
	}

	#[pallet::error]
	pub enum Error<T, I = ()> {
		/// Invalid Merkle Roots
		InvalidMerkleRoots,
		/// Unknown root
		UnknownRoot,
		/// Invalid withdraw proof
		InvalidWithdrawProof,
		/// Anchor not found.
		NoAnchorFound,
		// Invalid arbitrary data passed
		InvalidArbitraryData,
		/// Invalid nullifier that is already used
		/// (this error is returned when a nullifier is used twice)
		AlreadyRevealedNullifier,
	}

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config<I>, I: 'static = ()> {
		// (asset_id, deposit_size, max_edges)
		pub anchors: Vec<(CurrencyIdOf<T, I>, BalanceOf<T, I>, u32)>,
	}

	#[cfg(feature = "std")]
	impl<T: Config<I>, I: 'static> Default for GenesisConfig<T, I> {
		fn default() -> Self {
			GenesisConfig::<T, I> { anchors: Vec::new() }
		}
	}

	#[pallet::genesis_build]
	impl<T: Config<I>, I: 'static> GenesisBuild<T, I> for GenesisConfig<T, I> {
		fn build(&self) {
			self.anchors.iter().for_each(|(asset_id, deposit_size, max_edges)| {
				let _ = <Pallet<T, I> as AnchorInterface<_>>::create(
					None,
					deposit_size.clone(),
					30,
					*max_edges,
					asset_id.clone(),
				)
				.map_err(|_| panic!("Failed to create anchor"));
			})
		}
	}

	#[pallet::hooks]
	impl<T: Config<I>, I: 'static> Hooks<BlockNumberFor<T>> for Pallet<T, I> {}

	#[pallet::call]
	impl<T: Config<I>, I: 'static> Pallet<T, I> {
		#[pallet::weight(<T as Config<I>>::WeightInfo::create(*depth as u32, *max_edges))]
		pub fn create(
			origin: OriginFor<T>,
			deposit_size: BalanceOf<T, I>,
			max_edges: u32,
			depth: u8,
			asset: CurrencyIdOf<T, I>,
		) -> DispatchResultWithPostInfo {
			// Should it only be the root who can create anchors?
			ensure_root(origin)?;
			let tree_id =
				<Self as AnchorInterface<_>>::create(None, deposit_size, depth, max_edges, asset)?;
			Self::deposit_event(Event::AnchorCreation { tree_id });
			Ok(().into())
		}

		#[transactional]
		#[pallet::weight(<T as Config<I>>::WeightInfo::deposit())]
		pub fn deposit(
			origin: OriginFor<T>,
			tree_id: T::TreeId,
			leaf: T::Element,
		) -> DispatchResultWithPostInfo {
			let origin = ensure_signed(origin)?;
			<Self as AnchorInterface<_>>::deposit(origin, tree_id, leaf)?;
			Ok(().into())
		}

		/// Same as [Self::deposit] but with another call to update the linked
		/// anchors cross-chain (if any).
		// FIXME: update the weight here
		#[transactional]
		#[pallet::weight(<T as Config<I>>::WeightInfo::deposit())]
		pub fn deposit_and_update_linked_anchors(
			origin: OriginFor<T>,
			tree_id: T::TreeId,
			leaf: T::Element,
		) -> DispatchResultWithPostInfo {
			let origin = ensure_signed(origin)?;
			<Self as AnchorInterface<_>>::deposit(origin.clone(), tree_id, leaf)?;
			T::PostDepositHook::post_deposit(origin.clone(), tree_id, leaf)?;
			Self::deposit_event(Event::PostDeposit { depositor: origin, tree_id, leaf });
			Ok(().into())
		}

		#[transactional]
		#[pallet::weight(<T as Config<I>>::WeightInfo::withdraw())]
		pub fn withdraw(
			origin: OriginFor<T>,
			id: T::TreeId,
			proof_bytes: Vec<u8>,
			roots: Vec<T::Element>,
			nullifier_hash: T::Element,
			recipient: T::AccountId,
			relayer: T::AccountId,
			fee: BalanceOf<T, I>,
			refund: BalanceOf<T, I>,
			commitment: T::Element,
		) -> DispatchResultWithPostInfo {
			ensure_signed(origin)?;
			<Self as AnchorInterface<_>>::withdraw(
				id,
				proof_bytes.as_slice(),
				roots,
				nullifier_hash,
				recipient,
				relayer,
				fee,
				refund,
				commitment,
			)?;
			Ok(().into())
		}
	}
}

pub struct AnchorConfigration<T: Config<I>, I: 'static>(
	core::marker::PhantomData<T>,
	core::marker::PhantomData<I>,
);

impl<T: Config<I>, I: 'static> AnchorConfig for AnchorConfigration<T, I> {
	type AccountId = T::AccountId;
	type Balance = BalanceOf<T, I>;
	type ChainId = T::ChainId;
	type CurrencyId = CurrencyIdOf<T, I>;
	type Element = T::Element;
	type LeafIndex = T::LeafIndex;
	type TreeId = T::TreeId;
}

impl<T: Config<I>, I: 'static> AnchorInterface<AnchorConfigration<T, I>> for Pallet<T, I> {
	fn create(
		creator: Option<T::AccountId>,
		deposit_size: BalanceOf<T, I>,
		depth: u8,
		max_edges: u32,
		asset: CurrencyIdOf<T, I>,
	) -> Result<T::TreeId, DispatchError> {
		let id = T::LinkableTree::create(creator, max_edges, depth)?;
		Anchors::<T, I>::insert(id, AnchorMetadata { deposit_size, asset });
		Ok(id)
	}

	fn deposit(
		depositor: T::AccountId,
		id: T::TreeId,
		leaf: T::Element,
	) -> Result<(), DispatchError> {
		// get the anchor if it exists
		let anchor = Self::get_anchor(id)?;
		// insert the leaf
		T::LinkableTree::insert_in_order(id, leaf)?;
		// transfer tokens to the pallet
		<T as Config<I>>::Currency::transfer(
			anchor.asset,
			&depositor,
			&Self::account_id(),
			anchor.deposit_size,
		)?;

		Self::deposit_event(Event::Deposit {
			depositor,
			tree_id: id,
			leaf,
			amount: anchor.deposit_size,
		});

		Ok(())
	}

	fn withdraw(
		id: T::TreeId,
		proof_bytes: &[u8],
		roots: Vec<T::Element>,
		nullifier_hash: T::Element,
		recipient: T::AccountId,
		relayer: T::AccountId,
		fee: BalanceOf<T, I>,
		refund: BalanceOf<T, I>,
		commitment: T::Element,
	) -> Result<(), DispatchError> {
		// double check the number of roots
		T::LinkableTree::ensure_max_edges(id, roots.len())?;
		// Check if local root is known
		T::LinkableTree::ensure_known_root(id, roots[0])?;
		// Check if neighbor roots are known
		T::LinkableTree::ensure_known_neighbor_roots(id, &roots[1..].to_vec())?;

		// Check nullifier and add or return `InvalidNullifier`
		Self::ensure_nullifier_unused(id, nullifier_hash)?;
		Self::add_nullifier_hash(id, nullifier_hash)?;
		// Format proof public inputs for verification
		// FIXME: This is for a specfic gadget so we ought to create a generic handler
		// FIXME: Such as a unpack/pack public inputs trait
		// FIXME: 	-> T::PublicInputTrait::validate(public_bytes: &[u8])
		//
		// nullifier_hash (0..32)
		// recipient (32..64)
		// relayer (64..96)
		// fee (96..128)
		// refund (128..160)
		// chain_id (160..192)
		// root[1] (224..256)
		// root[2] (256..288)
		// root[m - 1] (...)
		let mut bytes = vec![];

		let element_encoder = |v: &[u8]| {
			let mut output = [0u8; 32];
			output.iter_mut().zip(v).for_each(|(b1, b2)| *b1 = *b2);
			output
		};
		let recipient_bytes = truncate_and_pad(&recipient.using_encoded(element_encoder)[..]);
		let relayer_bytes = truncate_and_pad(&relayer.using_encoded(element_encoder)[..]);
		let chain_id_type_bytes =
			T::LinkableTree::get_chain_id_type().using_encoded(element_encoder);

		let mut arbitrary_data_bytes = Vec::new();
		arbitrary_data_bytes.extend_from_slice(&recipient_bytes);
		arbitrary_data_bytes.extend_from_slice(&relayer_bytes);
		arbitrary_data_bytes.extend_from_slice(&fee.encode());
		arbitrary_data_bytes.extend_from_slice(&refund.encode());
		arbitrary_data_bytes.extend_from_slice(&commitment.to_bytes());
		let arbitrary_data = T::ArbitraryHasher::hash(&arbitrary_data_bytes, &[])
			.map_err(|_| Error::<T, I>::InvalidArbitraryData)?;

		bytes.extend_from_slice(&nullifier_hash.encode());
		bytes.extend_from_slice(&arbitrary_data);
		bytes.extend_from_slice(&chain_id_type_bytes);
		for root in &roots {
			bytes.extend_from_slice(&root.encode());
		}
		let result = <T as pallet::Config<I>>::Verifier::verify(&bytes, proof_bytes)?;
		ensure!(result, Error::<T, I>::InvalidWithdrawProof);
		// withdraw or refresh depending on the refresh commitment value
		let anchor = Self::get_anchor(id)?;
		if commitment.encode() == T::Element::default().encode() {
			// transfer the deposit to the recipient when the commitment is default / zero
			// (a withdrawal)
			<T as Config<I>>::Currency::transfer(
				anchor.asset,
				&Self::account_id(),
				&recipient,
				anchor.deposit_size,
			)?;
			Self::deposit_event(Event::Withdraw {
				who: recipient.clone(),
				amount: anchor.deposit_size,
			});
		} else {
			// deposit the new commitment when the commitment is not default / zero (a
			// refresh)
			T::LinkableTree::insert_in_order(id, commitment)?;
			Self::deposit_event(Event::Refresh { tree_id: id, leaf: commitment });
		}

		Ok(())
	}

	fn add_nullifier_hash(id: T::TreeId, nullifier_hash: T::Element) -> Result<(), DispatchError> {
		NullifierHashes::<T, I>::insert(id, nullifier_hash, true);
		Ok(())
	}

	fn add_edge(
		id: T::TreeId,
		src_chain_id: T::ChainId,
		root: T::Element,
		latest_leaf_index: T::LeafIndex,
		target: T::Element,
	) -> Result<(), DispatchError> {
		T::LinkableTree::add_edge(id, src_chain_id, root, latest_leaf_index, target)
	}

	fn update_edge(
		id: T::TreeId,
		src_chain_id: T::ChainId,
		root: T::Element,
		latest_leaf_index: T::LeafIndex,
		target: T::Element,
	) -> Result<(), DispatchError> {
		T::LinkableTree::update_edge(id, src_chain_id, root, latest_leaf_index, target)
	}
}

impl<T: Config<I>, I: 'static> AnchorInspector<AnchorConfigration<T, I>> for Pallet<T, I> {
	fn is_nullifier_used(tree_id: T::TreeId, nullifier_hash: T::Element) -> bool {
		NullifierHashes::<T, I>::contains_key(tree_id, nullifier_hash)
	}

	fn ensure_nullifier_unused(id: T::TreeId, nullifier: T::Element) -> Result<(), DispatchError> {
		ensure!(!Self::is_nullifier_used(id, nullifier), Error::<T, I>::AlreadyRevealedNullifier);
		Ok(())
	}

	fn has_edge(id: T::TreeId, src_chain_id: T::ChainId) -> bool {
		T::LinkableTree::has_edge(id, src_chain_id)
	}

	fn get_chain_id_type() -> T::ChainId {
		T::LinkableTree::get_chain_id_type()
	}

	fn get_chain_type() -> [u8; 2] {
		T::LinkableTree::get_chain_type()
	}
}

impl<T: Config<I>, I: 'static> Pallet<T, I> {
	pub fn account_id() -> T::AccountId {
		T::PalletId::get().into_account()
	}

	pub fn get_anchor(
		id: T::TreeId,
	) -> Result<AnchorMetadata<BalanceOf<T, I>, CurrencyIdOf<T, I>>, DispatchError> {
		let anchor = Anchors::<T, I>::get(id);
		ensure!(anchor.is_some(), Error::<T, I>::NoAnchorFound);
		Ok(anchor.unwrap())
	}
}

pub trait PostDepositHook<T: Config<I>, I: 'static> {
	fn post_deposit(depositor: T::AccountId, id: T::TreeId, leaf: T::Element) -> DispatchResult;
}

impl<T: Config<I>, I: 'static> PostDepositHook<T, I> for () {
	fn post_deposit(_: T::AccountId, _: T::TreeId, _: T::Element) -> DispatchResult {
		Ok(())
	}
}
/// Truncate and pad 256 bit slice
pub fn truncate_and_pad(t: &[u8]) -> Vec<u8> {
	let mut truncated_bytes = t[..20].to_vec();
	truncated_bytes.extend_from_slice(&[0u8; 12]);
	truncated_bytes
}
