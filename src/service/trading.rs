use std::str::FromStr;
use std::sync::Arc;

use alloy::primitives::{Address, U256};
use alloy::providers::ProviderBuilder;
use rmcp::handler::server::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{Json, ServerHandler, tool, tool_handler, tool_router};
use rust_decimal::Decimal;
use tracing::instrument;

use crate::config::Config;
use crate::repository::{AlloyEthereumRepository, EthereumRepository};
use crate::service::token_registry::TokenRegistry;
use crate::service::types::{
    GetBalanceRequest, GetBalanceResponse, GetBalanceResult, GetTokenPriceRequest,
    GetTokenPriceResponse, GetTokenPriceResult, SwapTokensRequest, SwapTokensResponse,
    SwapTokensResult,
};
use crate::service::utils::{
    calculate_exchange_rate, calculate_minimum_output, calculate_price, calculate_price_impact,
    format_balance, parse_amount,
};
use crate::service::{ServiceError, ServiceResult};

/// ETH decimals - Ethereum uses 18 decimal places (1 ETH = 10^18 wei)
const ETH_DECIMALS: u8 = 18;

pub struct EthereumTradingService {
    tool_router: ToolRouter<Self>,
    repository: Box<dyn EthereumRepository>,
    token_registry: TokenRegistry,
}

// MCP Tool Layer
#[tool_router]
impl EthereumTradingService {
    pub fn new(config: &Config) -> Self {
        // Use RPC URL from configuration
        let rpc_url = &config.rpc.url;

        let provider =
            ProviderBuilder::new().connect_http(rpc_url.parse().expect("Invalid RPC URL"));

        // Create repository with wallet if private key is provided
        let repository: Box<dyn EthereumRepository> = if !config.wallet.private_key.is_empty() {
            match AlloyEthereumRepository::new_with_wallet(
                Arc::new(provider),
                &config.wallet.private_key,
            ) {
                Ok(repo) => {
                    if let Some(address) = repo.wallet_address() {
                        tracing::info!("Initialized with wallet address: {address}");
                    }
                    Box::new(repo)
                }
                Err(e) => {
                    tracing::warn!("Failed to initialize wallet: {e}. Using read-only mode.");
                    Box::new(AlloyEthereumRepository::new(Arc::new(
                        ProviderBuilder::new()
                            .connect_http(rpc_url.parse().expect("Invalid RPC URL")),
                    )))
                }
            }
        } else {
            tracing::info!("No private key provided. Running in read-only mode.");
            Box::new(AlloyEthereumRepository::new(Arc::new(provider)))
        };

        Self {
            tool_router: Self::tool_router(),
            repository,
            token_registry: TokenRegistry::new(),
        }
    }

    #[instrument(skip(self))]
    #[tool(description = "Query ETH and ERC20 token balances")]
    pub async fn get_balance(
        &self,
        Parameters(req): Parameters<GetBalanceRequest>,
    ) -> Json<GetBalanceResult> {
        match self.get_balance_impl(req).await {
            Ok(response) => Json(GetBalanceResult::Success(response)),
            Err(e) => {
                tracing::error!("Failed to get balance: {e}");
                Json(GetBalanceResult::Error { error: e })
            }
        }
    }

    #[instrument(skip(self))]
    #[tool(description = "Get current token price in USD or ETH")]
    pub async fn get_token_price(
        &self,
        Parameters(req): Parameters<GetTokenPriceRequest>,
    ) -> Json<GetTokenPriceResult> {
        match self.get_token_price_impl(req).await {
            Ok(response) => Json(GetTokenPriceResult::Success(response)),
            Err(e) => {
                tracing::error!("Failed to get token price: {e}");
                Json(GetTokenPriceResult::Error { error: e })
            }
        }
    }

    #[instrument(skip(self))]
    #[tool(description = "Execute a token swap simulation on Uniswap V2 or V3.")]
    pub async fn swap_tokens(
        &self,
        Parameters(req): Parameters<SwapTokensRequest>,
    ) -> Json<SwapTokensResult> {
        match self.swap_tokens_impl(req).await {
            Ok(response) => Json(SwapTokensResult::Success(response)),
            Err(e) => {
                tracing::error!("Failed to simulate swap: {e}");
                Json(SwapTokensResult::Error { error: e })
            }
        }
    }
}

// Business Logic - Core implementation
impl EthereumTradingService {
    #[instrument(skip(self), err)]
    async fn get_balance_impl(&self, req: GetBalanceRequest) -> ServiceResult<GetBalanceResponse> {
        let address = Address::from_str(&req.wallet_address)
            .map_err(|e| ServiceError::InvalidWalletAddress(e.to_string()))?;

        tracing::info!("Querying balance for address: {}", address);

        match req.token_contract_address {
            Some(token_address) => {
                // ERC20 token balance
                let token_addr = Address::from_str(&token_address)
                    .map_err(|e| ServiceError::InvalidWalletAddress(e.to_string()))?;

                let token_balance = self
                    .repository
                    .get_erc20_balance(token_addr, address)
                    .await?;

                let formatted_balance =
                    format_balance(token_balance.balance, token_balance.decimals);

                Ok(GetBalanceResponse {
                    balance: token_balance.balance.to_string(),
                    formatted_balance,
                    decimals: token_balance.decimals,
                    symbol: token_balance.symbol,
                })
            }
            None => {
                // Native ETH balance
                let balance = self.repository.get_eth_balance(address).await?;
                let formatted_balance = format_balance(balance, ETH_DECIMALS);

                Ok(GetBalanceResponse {
                    balance: balance.to_string(),
                    formatted_balance,
                    decimals: ETH_DECIMALS,
                    symbol: "ETH".to_string(),
                })
            }
        }
    }

    #[instrument(skip(self), err)]
    async fn get_token_price_impl(
        &self,
        req: GetTokenPriceRequest,
    ) -> ServiceResult<GetTokenPriceResponse> {
        // Lookup token address from registry or dynamic sources
        let (token_address, symbol) = match req {
            GetTokenPriceRequest::Symbol { symbol } => {
                let addr = self.lookup_token_address(&symbol)?;
                (addr, symbol)
            }
            GetTokenPriceRequest::ContractAddress { contract_address } => {
                let addr = Address::from_str(&contract_address)
                    .map_err(|e| ServiceError::InvalidWalletAddress(e.to_string()))?;
                let metadata = self.repository.get_token_metadata(addr).await?;
                (contract_address, metadata.symbol)
            }
        };

        let token_addr = Address::from_str(&token_address)
            .map_err(|e| ServiceError::InvalidWalletAddress(e.to_string()))?;

        // Special handling for ETH/WETH - return ETH USD price directly
        let weth_address = Address::from_str(TokenRegistry::weth_address())
            .map_err(|e| ServiceError::InvalidWalletAddress(e.to_string()))?;

        tracing::info!("Getting price for token: {} ({})", symbol, token_address);

        let (price_eth, price_usd) = if token_addr == weth_address {
            // For ETH/WETH, price in ETH is 1.0, and get USD price from USDC pair
            let eth_usd = self.repository.get_eth_usd_price().await?;
            ("1.0".to_string(), eth_usd.to_string())
        } else {
            // For other tokens, get price from Uniswap V2 WETH pair
            self.get_price_from_uniswap(token_addr, weth_address)
                .await?
        };

        Ok(GetTokenPriceResponse {
            symbol,
            address: token_address.to_string(),
            price_usd,
            price_eth,
            timestamp: chrono::Utc::now().timestamp(),
        })
    }

    #[instrument(skip(self), err)]
    async fn swap_tokens_impl(&self, req: SwapTokensRequest) -> ServiceResult<SwapTokensResponse> {
        // Determine which Uniswap version to use (default to V2)
        let uniswap_version = req.uniswap_version.as_deref().unwrap_or("v2");

        match uniswap_version.to_lowercase().as_str() {
            "v2" => self.swap_tokens_v2(req).await,
            "v3" => self.swap_tokens_v3(req).await,
            _ => Err(ServiceError::InvalidAmount(format!(
                "Invalid Uniswap version: {}. Must be 'v2' or 'v3'",
                uniswap_version
            ))),
        }
    }

    #[instrument(skip(self), err)]
    async fn swap_tokens_v2(&self, req: SwapTokensRequest) -> ServiceResult<SwapTokensResponse> {
        let from_token = self.parse_token_address_or_symbol(&req.from_token).await?;

        let to_token = self.parse_token_address_or_symbol(&req.to_token).await?;

        // Get from_token metadata to know its decimals
        let from_metadata = self.repository.get_token_metadata(from_token).await?;

        // Parse amount with proper decimals (converts human-readable amount to smallest unit)
        let amount_in = parse_amount(&req.amount, from_metadata.decimals)
            .map_err(|e| ServiceError::InvalidAmount(e))?;
        tracing::info!(
            "Amount in (parsed): {} ({})",
            amount_in,
            format_balance(amount_in, from_metadata.decimals)
        );

        let slippage = Decimal::from_str(&req.slippage_tolerance)
            .map_err(|e| ServiceError::InvalidAmount(format!("Invalid slippage: {e}")))?;

        // Build swap path
        let path = vec![from_token, to_token];

        // Get expected output and calculate minimum with slippage
        let amount_out = self.get_swap_output_amount(amount_in, &path).await?;
        tracing::info!("Amount out: {}", amount_out);

        // Check if amount_out is zero and provide helpful error
        if amount_out.is_zero() {
            // Get to_token metadata for better error messages
            let to_metadata = self.repository.get_token_metadata(to_token).await.ok();

            let from_symbol = &from_metadata.symbol;
            let to_symbol = to_metadata
                .as_ref()
                .map(|m| m.symbol.as_str())
                .unwrap_or("Unknown");
            let from_decimals = from_metadata.decimals;

            // Try to get reserves to provide more context
            match self
                .repository
                .get_uniswap_pair_reserves(from_token, to_token)
                .await
            {
                Ok((reserve_in, reserve_out, _, _)) => {
                    return Err(ServiceError::SwapSimulationFailed(format!(
                        "Estimated output is 0 {} for {} {}. This could be due to:\n\
                         1. Insufficient liquidity (Reserve {}: {}, Reserve {}: {})\n\
                         2. Input amount too small (try a larger amount)\n\
                         3. The swap path may need intermediate tokens\n\
                         \n\
                         Suggestion: Try using WETH as an intermediate token, or increase the swap amount.",
                        to_symbol,
                        format_balance(amount_in, from_decimals),
                        from_symbol,
                        from_symbol,
                        reserve_in,
                        to_symbol,
                        reserve_out
                    )));
                }
                Err(_) => {
                    return Err(ServiceError::SwapSimulationFailed(format!(
                        "No liquidity pool found for {}/{} pair. The trading pair may not exist on Uniswap V2.\n\
                         \n\
                         Suggestions:\n\
                         - Use a different DEX or token pair\n\
                         - Try routing through WETH (e.g., {} -> WETH -> {})",
                        from_symbol, to_symbol, from_symbol, to_symbol
                    )));
                }
            }
        }

        let minimum_output = calculate_minimum_output(amount_out, slippage);

        // Get to_token metadata for proper decimal formatting
        let to_metadata = self.repository.get_token_metadata(to_token).await?;

        // Get reserves for price impact calculation
        let (reserve_in, reserve_out, _, _) = self
            .repository
            .get_uniswap_pair_reserves(from_token, to_token)
            .await?;

        // Estimate gas cost
        let (estimated_gas, gas_cost_eth) = self
            .estimate_swap_gas(&req.from_address, amount_in, minimum_output, path)
            .await?;

        // Calculate metrics
        let price_impact = calculate_price_impact(amount_in, amount_out, reserve_in, reserve_out);
        let exchange_rate = calculate_exchange_rate(
            amount_in,
            amount_out,
            from_metadata.decimals,
            to_metadata.decimals,
        );

        let response = SwapTokensResponse {
            estimated_output: format_balance(amount_out, to_metadata.decimals),
            estimated_output_raw: amount_out.to_string(),
            minimum_output: format_balance(minimum_output, to_metadata.decimals),
            estimated_gas,
            estimated_gas_eth: gas_cost_eth,
            price_impact: price_impact.clone(),
            exchange_rate: exchange_rate.clone(),
            transaction_data: format!("Swap simulation (V2): {from_token} -> {to_token}"),
        };

        tracing::info!(
            "V2 swap simulation complete: output={}, impact={}, rate={}",
            response.estimated_output,
            price_impact,
            exchange_rate
        );

        Ok(response)
    }

    #[instrument(skip(self), err)]
    async fn swap_tokens_v3(&self, req: SwapTokensRequest) -> ServiceResult<SwapTokensResponse> {
        let from_token = self.parse_token_address_or_symbol(&req.from_token).await?;
        let to_token = self.parse_token_address_or_symbol(&req.to_token).await?;

        // Get token metadata
        let from_metadata = self.repository.get_token_metadata(from_token).await?;
        let to_metadata = self.repository.get_token_metadata(to_token).await?;

        // Parse amount with proper decimals
        let amount_in = parse_amount(&req.amount, from_metadata.decimals)
            .map_err(|e| ServiceError::InvalidAmount(e))?;
        tracing::info!(
            "V3 Amount in (parsed): {} ({})",
            amount_in,
            format_balance(amount_in, from_metadata.decimals)
        );

        let slippage = Decimal::from_str(&req.slippage_tolerance)
            .map_err(|e| ServiceError::InvalidAmount(format!("Invalid slippage: {e}")))?;

        // Try different fee tiers for V3 (0.05%, 0.3%, 1%)
        // Most common is 0.3% (3000), but we'll try all three
        let fee_tiers = [3000u32, 500u32, 10000u32];
        let mut best_quote: Option<(U256, u64, u32)> = None;

        for fee in fee_tiers {
            match self
                .repository
                .get_v3_quote(from_token, to_token, amount_in, fee)
                .await
            {
                Ok((amount_out, gas_estimate)) => {
                    tracing::info!(
                        "V3 quote for fee tier {}: amount_out={}, gas={}",
                        fee,
                        amount_out,
                        gas_estimate
                    );

                    if !amount_out.is_zero() {
                        // Keep track of the best quote (highest output)
                        if best_quote.is_none() || amount_out > best_quote.as_ref().unwrap().0 {
                            best_quote = Some((amount_out, gas_estimate, fee));
                        }
                    }
                }
                Err(e) => {
                    tracing::debug!("V3 quote failed for fee tier {}: {}", fee, e);
                }
            }
        }

        // Check if we got any valid quote
        let (amount_out, gas_estimate, selected_fee) = best_quote.ok_or_else(|| {
            ServiceError::SwapSimulationFailed(format!(
                "No V3 liquidity pool found for {}/{} pair across all fee tiers (0.05%, 0.3%, 1%).\n\
                 \n\
                 Suggestions:\n\
                 - Try using V2 instead (set uniswap_version to 'v2')\n\
                 - Use a different token pair\n\
                 - Try routing through WETH (e.g., {} -> WETH -> {})",
                from_metadata.symbol,
                to_metadata.symbol,
                from_metadata.symbol,
                to_metadata.symbol
            ))
        })?;

        tracing::info!(
            "Selected V3 pool with fee tier {} ({}%)",
            selected_fee,
            selected_fee as f64 / 10000.0
        );

        let minimum_output = calculate_minimum_output(amount_out, slippage);

        // For V3, we can't easily get reserves for price impact calculation
        // So we'll estimate it based on the output amount vs ideal constant product formula
        // For now, we'll use a simplified calculation or mark it as "N/A"
        let price_impact = "N/A (V3)".to_string();

        // Estimate gas cost
        let (estimated_gas, gas_cost_eth) = if let Some(addr_str) = &req.from_address {
            let from_address = Address::from_str(addr_str)
                .map_err(|e| ServiceError::InvalidWalletAddress(e.to_string()))?;
            let deadline = U256::from(chrono::Utc::now().timestamp() + 3600);

            match self
                .repository
                .simulate_v3_swap(
                    from_address,
                    from_token,
                    to_token,
                    amount_in,
                    minimum_output,
                    selected_fee,
                    deadline,
                )
                .await
            {
                Ok(gas) => self.format_gas_cost(gas).await?,
                Err(_) => {
                    // Use the gas estimate from the quote
                    self.format_gas_cost(gas_estimate).await?
                }
            }
        } else {
            // Use the gas estimate from the quote
            self.format_gas_cost(gas_estimate).await?
        };

        let exchange_rate = calculate_exchange_rate(
            amount_in,
            amount_out,
            from_metadata.decimals,
            to_metadata.decimals,
        );

        tracing::info!(
            "V3 swap simulation complete: fee={}%, output={}, gas={}",
            selected_fee as f64 / 10000.0,
            format_balance(amount_out, to_metadata.decimals),
            estimated_gas
        );

        Ok(SwapTokensResponse {
            estimated_output: format_balance(amount_out, to_metadata.decimals),
            estimated_output_raw: amount_out.to_string(),
            minimum_output: format_balance(minimum_output, to_metadata.decimals),
            estimated_gas,
            estimated_gas_eth: gas_cost_eth,
            price_impact,
            exchange_rate,
            transaction_data: format!(
                "Swap simulation (V3, fee={}): {from_token} -> {to_token}",
                selected_fee
            ),
        })
    }

    #[instrument(skip(self), err)]
    async fn get_price_from_uniswap(
        &self,
        token: Address,
        weth: Address,
    ) -> ServiceResult<(String, String)> {
        // Get token metadata to know its decimals
        let token_metadata = self.repository.get_token_metadata(token).await?;

        // Query Uniswap V2 Factory to get the pair address and reserves
        let (reserve_token, reserve_weth, _, _) = self
            .repository
            .get_uniswap_pair_reserves(token, weth)
            .await?;

        // Check if reserves are valid
        if reserve_token.is_zero() || reserve_weth.is_zero() {
            return Err(ServiceError::InsufficientLiquidity(format!(
                "No liquidity in Uniswap pair for token {token} and WETH"
            )));
        }

        // Calculate price in ETH using precise decimal arithmetic
        // Use actual token decimals (e.g., 6 for USDC, 18 for most others)
        let price_eth = calculate_price(reserve_weth, reserve_token, 18, token_metadata.decimals)?;

        // Get ETH/USD price from USDC/WETH Uniswap pair
        let eth_price_usd = self.repository.get_eth_usd_price().await?;
        let price_usd = price_eth * eth_price_usd;

        Ok((price_eth.to_string(), price_usd.to_string()))
    }

    /// Parse token address or symbol (supports both addresses and token symbols like "USDT", "ETH", etc.)
    #[instrument(skip(self), err)]
    async fn parse_token_address_or_symbol(&self, token: &str) -> ServiceResult<Address> {
        // First try to parse as an address
        if let Ok(addr) = Address::from_str(token) {
            return Ok(addr);
        }

        // If not a valid address, try to lookup as a symbol
        let address_str = self.lookup_token_address(token)?;
        Address::from_str(&address_str)
            .map_err(|e| ServiceError::InvalidWalletAddress(e.to_string()))
    }

    /// Get expected output amount from Uniswap Router
    #[instrument(skip(self), err)]
    async fn get_swap_output_amount(
        &self,
        amount_in: U256,
        path: &[Address],
    ) -> ServiceResult<U256> {
        let amounts = self
            .repository
            .get_swap_amounts_out(amount_in, path.to_vec())
            .await?;

        amounts.last().copied().ok_or_else(|| {
            ServiceError::SwapSimulationFailed("No output amount returned".to_string())
        })
    }

    /// Estimate gas cost for swap transaction
    #[instrument(skip(self), err)]
    async fn estimate_swap_gas(
        &self,
        from_address: &Option<String>,
        amount_in: U256,
        minimum_output: U256,
        path: Vec<Address>,
    ) -> ServiceResult<(String, String)> {
        if let Some(addr_str) = from_address {
            let from_address = Address::from_str(addr_str)
                .map_err(|e| ServiceError::InvalidWalletAddress(e.to_string()))?;
            let deadline = U256::from(chrono::Utc::now().timestamp() + 3600);

            match self
                .repository
                .simulate_swap(from_address, amount_in, minimum_output, path, deadline)
                .await
            {
                Ok(gas) => Ok(self.format_gas_cost(gas).await?),
                Err(_) => Ok(self.get_typical_gas_cost().await?),
            }
        } else {
            Ok(self.get_typical_gas_cost().await?)
        }
    }

    /// Format gas cost with current gas price
    #[instrument(skip(self), err)]
    async fn format_gas_cost(&self, gas: u64) -> ServiceResult<(String, String)> {
        let gas_price = self.repository.get_gas_price().await?;
        let gas_cost_wei = U256::from(gas) * U256::from(gas_price);
        let gas_cost = format_balance(gas_cost_wei, ETH_DECIMALS);
        Ok((gas.to_string(), gas_cost))
    }

    /// Get typical Uniswap V2 swap gas estimate
    #[instrument(skip(self), err)]
    async fn get_typical_gas_cost(&self) -> ServiceResult<(String, String)> {
        const TYPICAL_GAS: u64 = 150000;
        self.format_gas_cost(TYPICAL_GAS).await
    }

    /// Lookup token address by symbol from registry
    #[instrument(skip(self), err)]
    fn lookup_token_address(&self, symbol: &str) -> ServiceResult<String> {
        self.token_registry
            .lookup(symbol)
            .map(|addr| addr.to_string())
            .ok_or_else(|| {
                tracing::warn!("Token symbol not found in registry: {}", symbol);
                ServiceError::TokenNotFound(format!(
                    "{} (Supported tokens: {})",
                    symbol,
                    self.token_registry.supported_tokens().join(", ")
                ))
            })
    }
}

#[tool_handler]
impl ServerHandler for EthereumTradingService {}
