use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProofError {
    #[error("Proof is not valid")]
    InvalidProof,

    #[error("Unable to read file")]
    FileReadError,

    #[error("IO error occurred: {0}")]
    Io(#[from] std::io::Error),

    #[error("Unknown error")]
    Unknown,
}