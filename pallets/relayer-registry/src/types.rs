// This file is part of Substrate.

// Copyright (C) 2021-2022 Parity Technologies (UK) Ltd.
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

use super::*;
use codec::{Decode, Encode, MaxEncodedLen};
use frame_support::{
	traits::{ConstU32, Get},
	BoundedVec, CloneNoBound, PartialEqNoBound, RuntimeDebugNoBound,
};
use pallet_identity::Data;
use scale_info::{
	build::{Fields, Variants},
	meta_type, Path, Type, TypeInfo, TypeParameter,
};
use sp_runtime::{traits::Zero, RuntimeDebug};
use sp_std::{fmt::Debug, iter::once, ops::Add, prelude::*};
use webb_proposals::{TargetSystem, TypedChainId};

/// Information concerning the identity of the controller of an account.
///
/// NOTE: This should be stored at the end of the storage item to facilitate the addition of extra
/// fields in a backwards compatible way through a specialized `Decode` impl.
#[derive(
	CloneNoBound, Encode, Decode, Eq, MaxEncodedLen, PartialEqNoBound, RuntimeDebugNoBound, TypeInfo,
)]
#[codec(mel_bound())]
#[cfg_attr(test, derive(frame_support::DefaultNoBound))]
#[scale_info(skip_type_params(FieldLimit))]
pub struct ResourceInfo<FieldLimit: Get<u32>> {
	/// Additional fields of the identity that are not catered for with the struct's explicit
	/// fields.
	pub additional: BoundedVec<(Data, Data), FieldLimit>,

	/// A reasonable display name for the controller of the account. This should be whatever it is
	/// that it is typically known as and should not be confusable with other entities, given
	/// reasonable context.
	///
	/// Stored as UTF-8.
	pub display: Data,

	/// The TypedChainId of where the resource is located.
	pub chain: TypedChainId,

	/// The TargetSystem of the resource
	pub target_system: TargetSystem,
}

/// Information concerning the identity of the controller of an account.
///
/// NOTE: This is stored separately primarily to facilitate the addition of extra fields in a
/// backwards compatible way through a specialized `Decode` impl.
#[derive(
	CloneNoBound, Encode, Eq, MaxEncodedLen, PartialEqNoBound, RuntimeDebugNoBound, TypeInfo,
)]
#[codec(mel_bound())]
#[scale_info(skip_type_params(MaxAdditionalFields))]
pub struct ResourceRecord<
	Balance: Encode + Decode + MaxEncodedLen + Copy + Clone + Debug + Eq + PartialEq,
	MaxAdditionalFields: Get<u32>,
> {
	/// Amount held on deposit for this information.
	pub deposit: Balance,

	/// Information on the identity.
	pub info: ResourceInfo<MaxAdditionalFields>,
}

impl<
		Balance: Encode + Decode + MaxEncodedLen + Copy + Clone + Debug + Eq + PartialEq,
		MaxAdditionalFields: Get<u32>,
	> Decode for ResourceRecord<Balance, MaxAdditionalFields>
{
	fn decode<I: codec::Input>(input: &mut I) -> sp_std::result::Result<Self, codec::Error> {
		let (deposit, info) = Decode::decode(&mut AppendZerosInput::new(input))?;
		Ok(Self { deposit, info })
	}
}
