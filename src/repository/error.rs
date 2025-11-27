use thiserror::Error;

#[derive(Debug, Clone, Error)]
pub enum RepositoryError {
    #[error("RPC error: {0}")]
    RpcError(String),

    #[error("Contract call error: {0}")]
    ContractError(String),

    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("{0}")]
    Other(String),
}
