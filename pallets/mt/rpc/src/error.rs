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

use jsonrpsee::{
	core::Error as JsonRpseeError,
	types::error::{CallError, ErrorObject},
};

#[derive(Debug, thiserror::Error)]
/// Top-level error type for the RPC handler
pub enum Error {
	/// The Merkle Tree RPC endpoint is not ready.
	#[error("Merkle Tree RPC endpoint not ready")]
	EndpointNotReady,
	/// Too many leaves requested
	#[error("Merkle Tree leaves request is too large")]
	TooManyLeavesRequested,
	/// Invalid TreeId
	#[error("Invalid Treeid")]
	InvalidTreeId,
}

/// The error codes returned by jsonrpc.
pub enum ErrorCode {
	/// Returned when Merkle Tree RPC endpoint is not ready.
	NotReady = 1,
	/// Too many leaves are requested
	TooManyLeaves,
	/// Invalid TreeId
	InvalidTreeId,
}

impl From<Error> for ErrorCode {
	fn from(error: Error) -> Self {
		match error {
			Error::EndpointNotReady => ErrorCode::NotReady,
			Error::TooManyLeavesRequested => ErrorCode::TooManyLeaves,
			Error::InvalidTreeId => ErrorCode::InvalidTreeId,
		}
	}
}

impl From<Error> for JsonRpseeError {
	fn from(error: Error) -> Self {
		let message = error.to_string();
		let code = ErrorCode::from(error);
		JsonRpseeError::Call(CallError::Custom(ErrorObject::owned(
			code as i32,
			message,
			None::<()>,
		)))
	}
}
