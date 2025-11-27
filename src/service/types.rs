use rmcp::schemars::{self, JsonSchema};
use serde::{Deserialize, Serialize};

use crate::service::ServiceError;

// Response types that include error handling
#[derive(Debug, JsonSchema, Serialize)]
#[serde(untagged)]
pub enum GetBalanceResult {
    Success(GetBalanceResponse),
    Error { error: ServiceError },
}

#[derive(Debug, JsonSchema, Serialize)]
#[serde(untagged)]
pub enum GetTokenPriceResult {
    Success(GetTokenPriceResponse),
    Error { error: ServiceError },
}

#[derive(Debug, JsonSchema, Serialize)]
#[serde(untagged)]
pub enum SwapTokensResult {
    Success(SwapTokensResponse),
    Error { error: ServiceError },
}

#[derive(Debug, JsonSchema, Serialize, Deserialize)]
pub struct GetBalanceRequest {
    /// Wallet address to query balance for
    pub wallet_address: String,
    /// Optional ERC20 token contract address. If not provided, returns ETH balance
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_contract_address: Option<String>,
}

#[derive(Debug, JsonSchema, Serialize)]
pub struct GetBalanceResponse {
    /// Raw balance value
    pub balance: String,
    /// Balance formatted with proper decimals
    pub formatted_balance: String,
    /// Token decimals
    pub decimals: u8,
    /// Token symbol (ETH or token symbol)
    pub symbol: String,
}

#[derive(Debug, JsonSchema, Serialize, Deserialize)]
#[serde(untagged)]
pub enum GetTokenPriceRequest {
    /// Query by token symbol (e.g., "ETH", "USDT", "BTC")
    Symbol { symbol: String },
    /// Query by token contract address (e.g., "0xdac17f958d2ee523a2206206994597c13d831ec7")
    ContractAddress { contract_address: String },
}

impl GetTokenPriceRequest {
    pub fn symbol(symbol: impl ToString) -> Self {
        let symbol = symbol.to_string();
        Self::Symbol { symbol }
    }

    pub fn contract_address(address: impl ToString) -> Self {
        let contract_address = address.to_string();
        Self::ContractAddress { contract_address }
    }
}

#[allow(dead_code)]
#[derive(Debug, JsonSchema, Serialize)]
pub struct GetTokenPriceResponse {
    /// Token symbol
    pub symbol: String,
    /// Token contract address
    pub address: String,
    /// Price in USD
    pub price_usd: String,
    /// Price in ETH
    pub price_eth: String,
    /// Timestamp of the price data
    pub timestamp: i64,
}

#[allow(dead_code)]
#[derive(Debug, JsonSchema, Serialize, Deserialize)]
pub struct SwapTokensRequest {
    /// Source token symbol or address (e.g., "ETH", "WETH", or "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2")
    pub from_token: String,

    /// Destination token symbol or address (e.g., "USDC", "DAI", or "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48")
    pub to_token: String,

    /// Amount to swap in human-readable format (e.g., "1" for 1 ETH, "100.5" for 100.5 USDC)
    /// This will be automatically converted to the token's smallest unit based on its decimals
    pub amount: String,

    /// Slippage tolerance in percentage (e.g., "0.5" for 0.5%, "2" for 2%)
    pub slippage_tolerance: String,

    /// Optional: Uniswap version to use ("v2" or "v3", defaults to "v2")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uniswap_version: Option<String>,

    /// Optional: Wallet address for simulation (defaults to a standard address)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from_address: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, JsonSchema, Serialize)]
pub struct SwapTokensResponse {
    /// Estimated output amount (formatted with decimals)
    pub estimated_output: String,

    /// Estimated output amount (raw)
    pub estimated_output_raw: String,

    /// Minimum output amount after slippage (formatted)
    pub minimum_output: String,

    /// Estimated gas cost in wei
    pub estimated_gas: String,

    /// Estimated gas cost in ETH
    pub estimated_gas_eth: String,

    /// Price impact percentage
    pub price_impact: String,

    /// Exchange rate (from_token per to_token)
    pub exchange_rate: String,

    /// Transaction data (for reference, not for execution)
    pub transaction_data: String,
}
