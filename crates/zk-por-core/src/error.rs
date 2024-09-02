use config::ConfigError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PoRError {
    #[error("Proof is not valid")]
    InvalidProof,

    #[error("Merkle proof is not valid for accountID {0}")]
    InvalidMerkleProof(String),

    #[error("config error: {0}")]
    ConfigError(#[from] ConfigError),

    #[error("IO error occurred: {0}")]
    Io(#[from] std::io::Error),

    #[error("Unknown error")]
    Unknown,

    #[error("Invalid parameter: {0}")]
    InvalidParameter(String),

    #[error("The verification circuit digest does not match the prover. ")]
    CircuitDigestMismatch,

    #[error("User is not valid")]
    InvalidUser,
}
