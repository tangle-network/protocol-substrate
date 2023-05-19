// This file is part of Webb.

// Copyright (C) 2021-2023 Webb Technologies Inc.
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
	/// The Linkable Tree RPC endpoint is not ready.
	#[error("Linkable Tree RPC endpoint not ready")]
	EndpointNotReady,
	/// The roots request failed
	#[error("Linkable Tree roots request failed")]
	RootsRequestFailure,
	/// The edges request failed
	#[error("Linkable Tree edges request failed")]
	EdgesRequestFailure,
}

/// The error codes returned by jsonrpc.
pub enum ErrorCode {
	/// Returned when Linkable Tree RPC endpoint is not ready.
	NotReady = 1,
	/// Roots request failed
	RootsRequestFailure,
	/// Edges request failed
	EdgesRequestFailure,
}

impl From<Error> for ErrorCode {
	fn from(error: Error) -> Self {
		match error {
			Error::EndpointNotReady => ErrorCode::NotReady,
			Error::RootsRequestFailure => ErrorCode::RootsRequestFailure,
			Error::EdgesRequestFailure => ErrorCode::EdgesRequestFailure,
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
