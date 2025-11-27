use rmcp::schemars::{self, JsonSchema};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::repository::RepositoryError;

#[derive(Debug, Clone, Error, JsonSchema, Serialize, Deserialize)]
#[serde(tag = "type", content = "message")]
pub enum ServiceError {
    // Business validation errors
    /// The provided wallet address is invalid or malformed.
    #[error("Invalid wallet address: {0}")]
    InvalidWalletAddress(String),

    /// The token was not found or is not supported by the service.
    #[error("Token not found or not supported: {0}")]
    TokenNotFound(String),

    /// The requested amount is invalid (e.g., negative, zero, or malformed).
    #[error("Invalid amount: {0}")]
    InvalidAmount(String),

    /// The wallet has insufficient balance for the requested operation.
    #[error("Insufficient balance: required {required}, available {available}")]
    InsufficientBalance { required: String, available: String },

    /// The price impact of the swap exceeds acceptable limits.
    #[error("Price impact too high: {impact}%, maximum allowed: {max}%")]
    PriceImpactTooHigh { impact: String, max: String },

    /// The actual slippage exceeded the user's tolerance.
    #[error("Slippage tolerance exceeded")]
    SlippageExceeded,

    /// The swap amount is below the minimum required amount.
    #[error("Swap amount too small: minimum {0}")]
    SwapAmountTooSmall(String),

    /// No liquidity pool found for the requested token pair.
    #[error("Liquidity pool not found for pair {token0}/{token1}")]
    LiquidityPoolNotFound { token0: String, token1: String },

    /// Insufficient liquidity in the pool for the requested swap.
    #[error("Insufficient liquidity in pool: {0}")]
    InsufficientLiquidity(String),

    /// Swap simulation failed.
    #[error("Swap simulation failed: {0}")]
    SwapSimulationFailed(String),

    // External API errors
    /// An error occurred while querying an external API (e.g., CoinGecko).
    #[error("External API error: {0}")]
    ExternalApiError(String),

    // Infrastructure errors (abstracted from repository layer)
    /// An error occurred while communicating with the blockchain.
    #[error("Blockchain connection error: {0}")]
    BlockchainError(String),

    /// An unexpected internal error occurred.
    #[error("Internal error: {0}")]
    InternalError(String),
}

impl From<RepositoryError> for ServiceError {
    fn from(err: RepositoryError) -> Self {
        match err {
            RepositoryError::RpcError(msg)
            | RepositoryError::NetworkError(msg)
            | RepositoryError::ContractError(msg) => {
                ServiceError::BlockchainError(format!("Failed to interact with blockchain: {msg}"))
            }
            RepositoryError::ParseError(msg) => ServiceError::InvalidWalletAddress(msg),
            RepositoryError::Other(msg) => ServiceError::InternalError(msg),
        }
    }
}
