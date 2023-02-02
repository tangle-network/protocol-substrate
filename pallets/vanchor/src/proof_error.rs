use std::{error::Error, fmt};

#[derive(Debug)]
pub enum ProofError {
	WitnessGenerationError,
}

impl ProofError {
	fn new() -> Self {
		ProofError::WitnessGenerationError
	}
}

impl fmt::Display for ProofError {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "Witness could not be generated")
	}
}

impl Error for ProofError {
	fn description(&self) -> &str {
		"Witness could not be generated"
	}
}
