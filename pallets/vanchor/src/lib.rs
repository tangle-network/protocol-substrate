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

//! # VAnchor Module
//!
//! A simple module for building variable Anchors.
//!
//! ## Overview
//!
//! The VAnchor module provides functionality for the following:
//!
//! * Creating new instances
//!
//! * Making transactions with variable amount of tokens
//!
//! The supported dispatchable functions are documented in the [`Call`] enum.
//!
//! ### Terminology
//!
//! ### Goals
//!
//! The VAnchor system in Webb is designed to make the following possible:
//!
//! * Define.
//!
//! ## Interface
//!
//! ## Related Modules
//!
//! * [`System`](../frame_system/index.html)
//! * [`Support`](../frame_support/index.html)

// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::type_complexity, clippy::too_many_arguments)]
#[cfg(test)]
pub mod mock;
#[cfg(test)]
mod test_utils;
#[cfg(test)]
mod tests;

use codec::Encode;
use webb_primitives::{
	field_ops::IntoPrimeField,
	hasher::InstanceHasher,
	linkable_tree::{LinkableTreeInspector, LinkableTreeInterface},
	traits::vanchor::{VAnchorConfig, VAnchorInspector, VAnchorInterface},
	types::{
		vanchor::{ExtData, ProofData, VAnchorMetadata},
		ElementTrait, IntoAbiToken,
	},
	utils::element_encoder,
	verifier::*,
};
use frame_support::{dispatch::DispatchResult, ensure, pallet_prelude::DispatchError, traits::Get};
use orml_traits::{
	arithmetic::{Signed, Zero},
	MultiCurrency, MultiCurrencyExtended,
};
use orml_traits::currency::transactional;

use sp_runtime::traits::AccountIdConversion;
use sp_std::{
	convert::{TryFrom, TryInto},
	prelude::*,
};

/// Type alias for the orml_traits::MultiCurrency::Balance type
pub type BalanceOf<T, I> =
	<<T as Config<I>>::Currency as MultiCurrency<<T as frame_system::Config>::AccountId>>::Balance;
/// Type alias for the orml_traits::MultiCurrency::Balance type
pub type AmountOf<T, I> =
	<<T as Config<I>>::Currency as MultiCurrencyExtended<<T as frame_system::Config>::AccountId>>::Amount;
/// Type alias for the orml_traits::MultiCurrency::CurrencyId type
pub type CurrencyIdOf<T, I> =
	<<T as pallet::Config<I>>::Currency as MultiCurrency<<T as frame_system::Config>::AccountId>>::CurrencyId;

pub use pallet::*;
#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::{dispatch::DispatchResultWithPostInfo, pallet_prelude::*, PalletId};
	use frame_system::pallet_prelude::*;

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T, I = ()>(_);

	#[pallet::config]
	/// The module configuration trait.
	pub trait Config<I: 'static = ()>: frame_system::Config + pallet_linkable_tree::Config<I> {
		/// The overarching event type.
		type Event: From<Event<Self, I>> + IsType<<Self as frame_system::Config>::Event>;

		#[pallet::constant]
		type PalletId: Get<PalletId>;

		/// The tree type
		type LinkableTree: LinkableTreeInterface<pallet_linkable_tree::LinkableTreeConfigration<Self, I>>
			+ LinkableTreeInspector<pallet_linkable_tree::LinkableTreeConfigration<Self, I>>;

		/// The verifier
		type Verifier2x2: VerifierModule;

		type EthereumHasher: InstanceHasher;

		type IntoField: IntoPrimeField<AmountOf<Self, I>>;

		/// Currency type for taking deposits
		type Currency: MultiCurrencyExtended<Self::AccountId>;

		type PostDepositHook: PostDepositHook<Self, I>;

		/// Native currency id
		#[pallet::constant]
		type NativeCurrencyId: Get<CurrencyIdOf<Self, I>>;

		// Max deposit amount
		type MaxDepositAmount: Get<BalanceOf<Self, I>>;
		// Max withdraw amount
		type MinWithdrawAmount: Get<BalanceOf<Self, I>>;
		// Max external amount
		type MaxExtAmount: Get<BalanceOf<Self, I>>;
		// Max fee for relayer
		type MaxFee: Get<BalanceOf<Self, I>>;
	}

	/// The map of trees to their anchor metadata
	#[pallet::storage]
	#[pallet::getter(fn vanchors)]
	pub type VAnchors<T: Config<I>, I: 'static = ()> = StorageMap<
		_,
		Blake2_128Concat,
		T::TreeId,
		VAnchorMetadata<T::AccountId, CurrencyIdOf<T, I>>,
		OptionQuery,
	>;

	/// The map of trees to their spent nullifier hashes
	#[pallet::storage]
	#[pallet::getter(fn nullifier_hashes)]
	pub type NullifierHashes<T: Config<I>, I: 'static = ()> =
		StorageDoubleMap<_, Blake2_128Concat, T::TreeId, Blake2_128Concat, T::Element, bool, ValueQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config<I>, I: 'static = ()> {
		/// New tree created
		VAnchorCreation { tree_id: T::TreeId },
		/// Transaction has been made
		Transaction {
			transactor: T::AccountId,
			tree_id: T::TreeId,
			leafs: Vec<T::Element>,
			amount: AmountOf<T, I>,
		},
		/// Deposit hook has executed successfully
		Deposit {
			depositor: T::AccountId,
			tree_id: T::TreeId,
			leaf: T::Element,
		},
	}

	#[pallet::error]
	pub enum Error<T, I = ()> {
		/// Invalid transaction proof
		InvalidTransactionProof,
		/// Variable Anchor not found.
		NoVAnchorFound,
		/// Invalid nullifier that is already used
		/// (this error is returned when a nullifier is used twice)
		AlreadyRevealedNullifier,
		// Invalid external amount
		InvalidExtAmount,
		// Maximum deposit amount exceeded
		InvalidDepositAmount,
		// Maximum withdraw amount exceeded
		InvalidWithdrawAmount,
		// Invalid external data
		InvalidExtData,
		// Invalid input nullifiers
		InvalidInputNullifiers,
		// Invalid fee
		InvalidFee,
		// Invalid public amount
		InvalidPublicAmount,
	}

	#[pallet::hooks]
	impl<T: Config<I>, I: 'static> Hooks<BlockNumberFor<T>> for Pallet<T, I> {}

	#[pallet::call]
	impl<T: Config<I>, I: 'static> Pallet<T, I> {
		#[pallet::weight(0)]
		pub fn create(
			origin: OriginFor<T>,
			max_edges: u32,
			depth: u8,
			asset: CurrencyIdOf<T, I>,
		) -> DispatchResultWithPostInfo {
			ensure_root(origin)?;
			let tree_id = <Self as VAnchorInterface<_>>::create(None, depth, max_edges, asset)?;
			Self::deposit_event(Event::VAnchorCreation { tree_id });
			Ok(().into())
		}

		#[transactional]
		#[pallet::weight(0)] // TODO: Fix after benchmarks
		pub fn transact(
			origin: OriginFor<T>,
			id: T::TreeId,
			proof_data: ProofData<T::Element>,
			ext_data: ExtData<T::AccountId, AmountOf<T, I>, BalanceOf<T, I>, T::Element>,
		) -> DispatchResultWithPostInfo {
			let sender = ensure_signed(origin)?;
			<Self as VAnchorInterface<_>>::transact(sender, id, proof_data, ext_data)?;
			Ok(().into())
		}
	}
}

pub struct VAnchorConfigration<T: Config<I>, I: 'static>(core::marker::PhantomData<T>, core::marker::PhantomData<I>);

impl<T: Config<I>, I: 'static> VAnchorConfig for VAnchorConfigration<T, I> {
	type AccountId = T::AccountId;
	type Amount = AmountOf<T, I>;
	type Balance = BalanceOf<T, I>;
	type ChainId = T::ChainId;
	type CurrencyId = CurrencyIdOf<T, I>;
	type Element = T::Element;
	type LeafIndex = T::LeafIndex;
	type TreeId = T::TreeId;
}

impl<T: Config<I>, I: 'static> VAnchorInterface<VAnchorConfigration<T, I>> for Pallet<T, I> {
	fn create(
		creator: Option<T::AccountId>,
		depth: u8,
		max_edges: u32,
		asset: CurrencyIdOf<T, I>,
	) -> Result<T::TreeId, DispatchError> {
		let id = T::LinkableTree::create(creator.clone(), max_edges, depth)?;
		VAnchors::<T, I>::insert(id, VAnchorMetadata { creator, asset });
		Ok(id)
	}

	fn transact(
		transactor: T::AccountId,
		id: T::TreeId,
		proof_data: ProofData<T::Element>,
		ext_data: ExtData<T::AccountId, AmountOf<T, I>, BalanceOf<T, I>, T::Element>,
	) -> Result<(), DispatchError> {
		// double check the number of roots
		T::LinkableTree::ensure_max_edges(id, proof_data.roots.len())?;
		// Check if local root is known
		T::LinkableTree::ensure_known_root(id, proof_data.roots[0])?;
		// Check if neighbor roots are known
		T::LinkableTree::ensure_known_neighbor_roots(id, &proof_data.roots)?;

		// Check nullifier and add or return `InvalidNullifier`
		for nullifier in &proof_data.input_nullifiers {
			Self::ensure_nullifier_unused(id, *nullifier)?;
		}

		let vanchor = Self::get_vanchor(id)?;

		// Compute hash of abi encoded ext_data, reduced into field from config
		let computed_ext_data_hash =
			T::EthereumHasher::hash(&ext_data.encode_abi(), &[]).map_err(|_| Error::<T, I>::InvalidExtData)?;
		// Ensure that the passed external data hash matches the computed one
		ensure!(
			proof_data.ext_data_hash.to_bytes() == &computed_ext_data_hash,
			Error::<T, I>::InvalidExtData
		);

		// Making sure that public amount and fee are correct
		ensure!(ext_data.fee < T::MaxFee::get(), Error::<T, I>::InvalidFee);
		let ext_amount_unsigned: BalanceOf<T, I> = ext_data
			.ext_amount
			.abs()
			.try_into()
			.map_err(|_| Error::<T, I>::InvalidExtAmount)?;
		ensure!(
			ext_amount_unsigned < T::MaxExtAmount::get(),
			Error::<T, I>::InvalidExtAmount
		);

		// Public amounnt can also be negative, in which
		// case it would wrap around the field, so we should check if FIELD_SIZE -
		// public_amount == proof_data.public_amount, in case of a negative ext_amount
		let fee_amount = AmountOf::<T, I>::try_from(ext_data.fee).map_err(|_| Error::<T, I>::InvalidFee)?;
		let calc_public_amount = ext_data.ext_amount - fee_amount;
		let calc_public_amount_bytes = T::IntoField::into_field(calc_public_amount);
		let calc_public_amount_element = T::Element::from_bytes(&calc_public_amount_bytes);
		ensure!(
			proof_data.public_amount == calc_public_amount_element,
			Error::<T, I>::InvalidPublicAmount
		);

		let chain_id = <T as pallet_linkable_tree::Config<I>>::ChainIdentifier::get();

		// Construct public inputs
		let mut bytes = Vec::new();
		bytes.extend_from_slice(&proof_data.public_amount.to_bytes());
		bytes.extend_from_slice(&proof_data.ext_data_hash.to_bytes());
		for null in &proof_data.input_nullifiers {
			bytes.extend_from_slice(&null.to_bytes());
		}
		for comm in &proof_data.output_commitments {
			bytes.extend_from_slice(&comm.to_bytes());
		}
		bytes.extend_from_slice(&chain_id.using_encoded(element_encoder));
		for root in proof_data.roots {
			bytes.extend_from_slice(&root.to_bytes());
		}

		let res = match (proof_data.input_nullifiers.len(), proof_data.output_commitments.len()) {
			(2, 2) => T::Verifier2x2::verify(&bytes, &proof_data.proof)?,
			_ => false,
		};
		ensure!(res, Error::<T, I>::InvalidTransactionProof);

		// Flag nullifiers as used
		for nullifier in &proof_data.input_nullifiers {
			Self::add_nullifier_hash(id, *nullifier)?;
		}

		let is_deposit = ext_data.ext_amount.is_positive();
		let is_negative = ext_data.ext_amount.is_negative();

		let abs_amount: BalanceOf<T, I> = ext_data
			.ext_amount
			.abs()
			.try_into()
			.map_err(|_| Error::<T, I>::InvalidExtAmount)?;

		if is_deposit {
			ensure!(
				abs_amount <= T::MaxDepositAmount::get(),
				Error::<T, I>::InvalidDepositAmount
			);

			// deposit tokens to the pallet from the transactor's account
			<T as Config<I>>::Currency::transfer(vanchor.asset, &transactor, &Self::account_id(), abs_amount)?;
		} else if is_negative {
			let min_withdraw = T::MinWithdrawAmount::get();
			ensure!(abs_amount >= min_withdraw, Error::<T, I>::InvalidWithdrawAmount);

			// Withdraw to recipient account
			<T as Config<I>>::Currency::transfer(vanchor.asset, &Self::account_id(), &ext_data.recipient, abs_amount)?;
		}

		let fee_exists = ext_data.fee > BalanceOf::<T, I>::zero();

		if fee_exists {
			// Send fee to the relayer
			<T as Config<I>>::Currency::transfer(vanchor.asset, &Self::account_id(), &ext_data.relayer, ext_data.fee)?;
		}

		// Insert output commitments into the tree
		for comm in &proof_data.output_commitments {
			T::LinkableTree::insert_in_order(id, *comm)?;
		}

		Self::deposit_event(Event::Transaction {
			transactor: transactor.clone(),
			tree_id: id,
			leafs: proof_data.output_commitments,
			amount: calc_public_amount,
		});
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
	) -> Result<(), DispatchError> {
		T::LinkableTree::add_edge(id, src_chain_id, root, latest_leaf_index)
	}

	fn update_edge(
		id: T::TreeId,
		src_chain_id: T::ChainId,
		root: T::Element,
		latest_leaf_index: T::LeafIndex,
	) -> Result<(), DispatchError> {
		T::LinkableTree::update_edge(id, src_chain_id, root, latest_leaf_index)
	}
}

impl<T: Config<I>, I: 'static> VAnchorInspector<VAnchorConfigration<T, I>> for Pallet<T, I> {
	fn is_nullifier_used(tree_id: T::TreeId, nullifier_hash: T::Element) -> bool {
		NullifierHashes::<T, I>::contains_key(tree_id, nullifier_hash)
	}

	fn ensure_nullifier_unused(id: T::TreeId, nullifier: T::Element) -> Result<(), DispatchError> {
		ensure!(
			!Self::is_nullifier_used(id, nullifier),
			Error::<T, I>::AlreadyRevealedNullifier
		);
		Ok(())
	}

	fn has_edge(id: T::TreeId, src_chain_id: T::ChainId) -> bool {
		T::LinkableTree::has_edge(id, src_chain_id)
	}
}

impl<T: Config<I>, I: 'static> Pallet<T, I> {
	pub fn account_id() -> T::AccountId {
		T::PalletId::get().into_account()
	}

	pub fn get_vanchor(id: T::TreeId) -> Result<VAnchorMetadata<T::AccountId, CurrencyIdOf<T, I>>, DispatchError> {
		let vanchor = VAnchors::<T, I>::get(id);
		ensure!(vanchor.is_some(), Error::<T, I>::NoVAnchorFound);
		Ok(vanchor.unwrap())
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
