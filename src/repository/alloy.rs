use std::str::FromStr;
use std::sync::Arc;

use alloy::network::EthereumWallet;
use alloy::primitives::{
    Address, U256,
    aliases::{U24, U160},
};
use alloy::providers::Provider;
use alloy::signers::local::PrivateKeySigner;
use async_trait::async_trait;
use rust_decimal::Decimal;
use tracing::instrument;

use super::error::RepositoryError;
use crate::repository::contract::{
    IERC20, IQuoterV2, ISwapRouter, IUniswapV2Factory, IUniswapV2Pair, IUniswapV2Router02,
};
use crate::repository::{EthereumRepository, RepoResult};

/// Uniswap V2 Factory contract address on Ethereum mainnet
const UNISWAP_V2_FACTORY: &str = "0x5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6f";

/// Uniswap V2 Router02 contract address on Ethereum mainnet
const UNISWAP_V2_ROUTER: &str = "0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D";

/// Uniswap V3 QuoterV2 contract address on Ethereum mainnet
const UNISWAP_V3_QUOTER_V2: &str = "0x61fFE014bA17989E743c5F6cB21bF9697530B21e";

/// Uniswap V3 SwapRouter contract address on Ethereum mainnet
const UNISWAP_V3_SWAP_ROUTER: &str = "0xE592427A0AEce92De3Edee1F18E0157C05861564";

// USDC address on Ethereum mainnet
const USDC_ADDRESS: &str = "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48";

// WETH address on Ethereum mainnet
const WETH_ADDRESS: &str = "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2";

#[derive(Debug, Clone)]
pub struct TokenBalance {
    pub balance: U256,
    pub decimals: u8,
    pub symbol: String,
}

#[derive(Debug, Clone)]
pub struct TokenMetadata {
    pub decimals: u8,
    pub symbol: String,
}

pub struct AlloyEthereumRepository<P> {
    provider: Arc<P>,
    wallet: Option<EthereumWallet>,
}

impl<P: Provider + Clone + 'static> AlloyEthereumRepository<P> {
    pub fn new(provider: Arc<P>) -> Self {
        Self {
            provider,
            wallet: None,
        }
    }

    pub fn new_with_wallet(provider: Arc<P>, private_key: &str) -> Result<Self, RepositoryError> {
        let signer = PrivateKeySigner::from_str(private_key)
            .map_err(|e| RepositoryError::ParseError(format!("Invalid private key: {e}")))?;

        let wallet = EthereumWallet::from(signer);

        Ok(Self {
            provider,
            wallet: Some(wallet),
        })
    }

    pub fn wallet_address(&self) -> Option<Address> {
        self.wallet.as_ref().map(|w| w.default_signer().address())
    }
}

#[async_trait]
impl<P: Provider + Clone + Send + Sync + 'static> EthereumRepository
    for AlloyEthereumRepository<P>
{
    #[instrument(skip(self), err)]
    async fn get_eth_balance(&self, address: Address) -> RepoResult<U256> {
        self.provider.get_balance(address).await.map_err(|e| {
            if e.to_string().contains("429") {
                tracing::warn!("Rate limited while getting ETH balance for {}", address);
            }
            RepositoryError::RpcError(e.to_string())
        })
    }

    #[instrument(skip(self), err)]
    async fn get_erc20_balance(&self, token: Address, owner: Address) -> RepoResult<TokenBalance> {
        let contract = IERC20::new(token, self.provider.clone());

        let balance = contract
            .balanceOf(owner)
            .call()
            .await
            .map_err(|e| RepositoryError::ContractError(e.to_string()))?;

        let decimals = contract
            .decimals()
            .call()
            .await
            .map_err(|e| RepositoryError::ContractError(e.to_string()))?;

        let symbol = contract
            .symbol()
            .call()
            .await
            .map_err(|e| RepositoryError::ContractError(e.to_string()))?;

        Ok(TokenBalance {
            balance,
            decimals,
            symbol,
        })
    }

    #[instrument(skip(self), err)]
    async fn get_token_metadata(&self, token: Address) -> RepoResult<TokenMetadata> {
        let contract = IERC20::new(token, self.provider.clone());

        let decimals = contract
            .decimals()
            .call()
            .await
            .map_err(|e| RepositoryError::ContractError(e.to_string()))?;

        let symbol = contract
            .symbol()
            .call()
            .await
            .map_err(|e| RepositoryError::ContractError(e.to_string()))?;

        Ok(TokenMetadata { decimals, symbol })
    }

    #[instrument(skip(self), err)]
    async fn get_gas_price(&self) -> RepoResult<u128> {
        self.provider
            .get_gas_price()
            .await
            .map_err(|e| RepositoryError::RpcError(e.to_string()))
    }

    #[instrument(skip(self), err)]
    async fn get_uniswap_pair_reserves(
        &self,
        token_a: Address,
        token_b: Address,
    ) -> RepoResult<(U256, U256, Address, Address)> {
        // 1. Get Factory contract
        let factory_address = Address::from_str(UNISWAP_V2_FACTORY)
            .map_err(|e| RepositoryError::ParseError(e.to_string()))?;
        let factory = IUniswapV2Factory::new(factory_address, self.provider.clone());

        // 2. Get pair address from factory
        let pair_address = factory
            .getPair(token_a, token_b)
            .call()
            .await
            .map_err(|e| RepositoryError::ContractError(format!("Failed to get pair: {}", e)))?;

        // Check if pair exists (non-zero address)
        if pair_address == Address::ZERO {
            return Err(RepositoryError::ContractError(format!(
                "No Uniswap V2 pair found for tokens {} and {}",
                token_a, token_b
            )));
        }

        // 3. Get pair contract
        let pair = IUniswapV2Pair::new(pair_address, self.provider.clone());

        // 4. Get reserves
        let reserves = pair.getReserves().call().await.map_err(|e| {
            RepositoryError::ContractError(format!("Failed to get reserves: {}", e))
        })?;

        // 5. Get token0 and token1 to determine order
        let token0 =
            pair.token0().call().await.map_err(|e| {
                RepositoryError::ContractError(format!("Failed to get token0: {}", e))
            })?;

        let token1 =
            pair.token1().call().await.map_err(|e| {
                RepositoryError::ContractError(format!("Failed to get token1: {}", e))
            })?;

        // Convert reserves from u112 to U256
        let reserve0 = U256::from(reserves.reserve0);
        let reserve1 = U256::from(reserves.reserve1);

        // Return reserves in the order matching token_a and token_b
        if token0 == token_a {
            Ok((reserve0, reserve1, token0, token1))
        } else {
            Ok((reserve1, reserve0, token1, token0))
        }
    }

    #[instrument(skip(self), err)]
    async fn get_eth_usd_price(&self) -> RepoResult<Decimal> {
        let usdc_address = Address::from_str(USDC_ADDRESS)
            .map_err(|e| RepositoryError::ParseError(e.to_string()))?;
        let weth_address = Address::from_str(WETH_ADDRESS)
            .map_err(|e| RepositoryError::ParseError(e.to_string()))?;

        // Get USDC/WETH reserves
        let (reserve_usdc, reserve_weth, _, _) = self
            .get_uniswap_pair_reserves(usdc_address, weth_address)
            .await?;

        if reserve_usdc.is_zero() || reserve_weth.is_zero() {
            return Err(RepositoryError::ContractError(
                "No liquidity in USDC/WETH pair".to_string(),
            ));
        }

        // USDC has 6 decimals, WETH has 18 decimals
        // Convert to Decimal for precise calculation
        let usdc_decimal = Decimal::from_str(&reserve_usdc.to_string()).map_err(|e| {
            RepositoryError::ParseError(format!("Failed to parse USDC reserve: {}", e))
        })?;

        let weth_decimal = Decimal::from_str(&reserve_weth.to_string()).map_err(|e| {
            RepositoryError::ParseError(format!("Failed to parse WETH reserve: {}", e))
        })?;

        // Adjust for decimals: USDC (6 decimals) / WETH (18 decimals)
        // Scale USDC up by 10^12 to match WETH decimals
        let usdc_scaled = usdc_decimal * Decimal::from(10_u64.pow(12));

        // Calculate price: (reserve_usdc * 10^12) / reserve_weth
        let eth_price = usdc_scaled / weth_decimal;

        Ok(eth_price)
    }

    #[instrument(skip(self), err)]
    async fn get_swap_amounts_out(
        &self,
        amount_in: U256,
        path: Vec<Address>,
    ) -> RepoResult<Vec<U256>> {
        tracing::debug!(
            "Getting swap amounts for path: {:?}, amount_in: {}",
            path,
            amount_in
        );

        let router_address = Address::from_str(UNISWAP_V2_ROUTER)
            .map_err(|e| RepositoryError::ParseError(e.to_string()))?;
        let router = IUniswapV2Router02::new(router_address, self.provider.clone());

        let amounts = router
            .getAmountsOut(amount_in, path.clone())
            .call()
            .await
            .map_err(|e| {
                tracing::error!("Failed to get amounts out for path {:?}: {}", path, e);
                RepositoryError::ContractError(format!("Failed to get amounts out: {}", e))
            })?;

        tracing::debug!("Swap amounts result: {:?}", amounts);
        Ok(amounts.to_vec())
    }

    #[instrument(skip(self), err)]
    async fn simulate_swap(
        &self,
        from: Address,
        amount_in: U256,
        amount_out_min: U256,
        path: Vec<Address>,
        deadline: U256,
    ) -> RepoResult<u64> {
        let router_address = Address::from_str(UNISWAP_V2_ROUTER)
            .map_err(|e| RepositoryError::ParseError(e.to_string()))?;
        let router = IUniswapV2Router02::new(router_address, self.provider.clone());

        // Build the swap transaction call
        let call = router.swapExactTokensForTokens(
            amount_in,
            amount_out_min,
            path.clone(),
            from,
            deadline,
        );

        // First, simulate the transaction using eth_call to verify it would succeed
        // This executes the transaction locally without broadcasting it to the network
        let _swap_result = call.call().await.map_err(|e| {
            tracing::debug!("Gas simulation failed: {}", e);
            RepositoryError::ContractError(format!("Swap simulation failed: {}", e))
        })?;

        // Then estimate gas for the transaction
        let gas_estimate = call.estimate_gas().await.map_err(|e| {
            RepositoryError::ContractError(format!("Failed to estimate gas: {}", e))
        })?;

        Ok(gas_estimate)
    }

    #[instrument(skip(self), err)]
    async fn get_v3_quote(
        &self,
        token_in: Address,
        token_out: Address,
        amount_in: U256,
        fee: u32,
    ) -> RepoResult<(U256, u64)> {
        let quoter_address = Address::from_str(UNISWAP_V3_QUOTER_V2)
            .map_err(|e| RepositoryError::ParseError(e.to_string()))?;
        let quoter = IQuoterV2::new(quoter_address, self.provider.clone());

        // Prepare quote parameters
        let params = IQuoterV2::QuoteExactInputSingleParams {
            tokenIn: token_in,
            tokenOut: token_out,
            amountIn: amount_in,
            fee: U24::from(fee),
            sqrtPriceLimitX96: U160::ZERO,
        };

        // Call quoteExactInputSingle
        let result = quoter
            .quoteExactInputSingle(params)
            .call()
            .await
            .map_err(|e| {
                tracing::error!(
                    "Failed to get V3 quote for {} -> {} (fee: {}): {}",
                    token_in,
                    token_out,
                    fee,
                    e
                );
                RepositoryError::ContractError(format!("Failed to get V3 quote: {}", e))
            })?;

        tracing::debug!(
            "V3 quote result - amountOut: {}, gasEstimate: {}",
            result.amountOut,
            result.gasEstimate
        );

        Ok((result.amountOut, result.gasEstimate.to::<u64>()))
    }

    #[instrument(skip(self), err)]
    async fn simulate_v3_swap(
        &self,
        from: Address,
        token_in: Address,
        token_out: Address,
        amount_in: U256,
        amount_out_min: U256,
        fee: u32,
        deadline: U256,
    ) -> RepoResult<u64> {
        let router_address = Address::from_str(UNISWAP_V3_SWAP_ROUTER)
            .map_err(|e| RepositoryError::ParseError(e.to_string()))?;
        let router = ISwapRouter::new(router_address, self.provider.clone());

        // Build the swap transaction call
        let params = ISwapRouter::ExactInputSingleParams {
            tokenIn: token_in,
            tokenOut: token_out,
            fee: U24::from(fee),
            recipient: from,
            deadline: deadline,
            amountIn: amount_in,
            amountOutMinimum: amount_out_min,
            sqrtPriceLimitX96: U160::ZERO,
        };

        let call = router.exactInputSingle(params);

        // First, simulate the transaction using eth_call to verify it would succeed
        let _swap_result = call.call().await.map_err(|e| {
            tracing::debug!("V3 swap simulation failed: {}", e);
            RepositoryError::ContractError(format!("V3 swap simulation failed: {}", e))
        })?;

        // Then estimate gas for the transaction
        let gas_estimate = call.estimate_gas().await.map_err(|e| {
            RepositoryError::ContractError(format!("Failed to estimate V3 gas: {}", e))
        })?;

        Ok(gas_estimate)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy::providers::ProviderBuilder;
    use std::str::FromStr;
    use std::time::Duration;

    // Test addresses
    const VITALIK_ADDRESS: &str = "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045";
    const RANDOM_ADDRESS: &str = "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb0";
    const BINANCE_HOT_WALLET: &str = "0x28C6c06298d514Db089934071355E5743bf21d60";
    const INVALID_CONTRACT: &str = "0x0000000000000000000000000000000000000001";

    // Token contract addresses
    const USDT_CONTRACT: &str = "0xdac17f958d2ee523a2206206994597c13d831ec7";
    const DAI_CONTRACT: &str = "0x6b175474e89094c44da98b954eedeac495271d0f";
    const WETH_CONTRACT: &str = "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2";
    const USDC_CONTRACT: &str = "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48";

    // Rate limiting delay between tests (in milliseconds)
    const TEST_DELAY_MS: u64 = 1000;

    const RPC_URL: &str = "https://eth.llamarpc.com";

    /// Helper function to add delay between tests to avoid rate limiting
    async fn rate_limit_delay() {
        tokio::time::sleep(Duration::from_millis(TEST_DELAY_MS)).await;
    }

    fn create_test_repository() -> AlloyEthereumRepository<impl Provider + Clone> {
        let rpc_url = std::env::var("RPC_URL").unwrap_or_else(|_| RPC_URL.to_string());

        let provider =
            ProviderBuilder::new().connect_http(rpc_url.parse().expect("Invalid RPC URL"));

        AlloyEthereumRepository::new(Arc::new(provider))
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_wallet_initialization_with_valid_key() {
        let provider =
            ProviderBuilder::new().connect_http(RPC_URL.parse().expect("Invalid RPC URL"));

        // Use a test private key (DO NOT use in production!)
        let test_private_key = "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80";

        let result = AlloyEthereumRepository::new_with_wallet(Arc::new(provider), test_private_key);
        assert!(
            result.is_ok(),
            "Failed to create repository with wallet: {:?}",
            result.err()
        );

        let repo = result.unwrap();
        let wallet_address = repo.wallet_address();
        assert!(wallet_address.is_some(), "Wallet address should be set");

        let address = wallet_address.unwrap();
        println!("✅ Wallet initialized with address: {}", address);

        // The test private key should derive to this address
        let expected_address = Address::from_str("0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266")
            .expect("Invalid expected address");
        assert_eq!(address, expected_address, "Wallet address mismatch");
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_wallet_initialization_with_invalid_key() {
        let provider =
            ProviderBuilder::new().connect_http(RPC_URL.parse().expect("Invalid RPC URL"));

        let invalid_key = "not_a_valid_private_key";

        let result = AlloyEthereumRepository::new_with_wallet(Arc::new(provider), invalid_key);
        assert!(result.is_err(), "Should fail with invalid private key");

        if let Err(e) = result {
            match e {
                RepositoryError::ParseError(msg) => {
                    println!("✅ Got expected parse error: {}", msg);
                    assert!(
                        msg.contains("Invalid private key"),
                        "Error message should mention invalid private key"
                    );
                }
                _ => panic!("Expected ParseError, got: {:?}", e),
            }
        }
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_repository_without_wallet() {
        let repo = create_test_repository();
        let wallet_address = repo.wallet_address();
        assert!(
            wallet_address.is_none(),
            "Repository without wallet should have no address"
        );
        println!("✅ Repository created in read-only mode (no wallet)");
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_get_eth_balance_should_work() {
        rate_limit_delay().await;
        let repo = create_test_repository();

        // Vitalik's address - known to have ETH balance
        let address = Address::from_str(VITALIK_ADDRESS).expect("Invalid address");

        let result = repo.get_eth_balance(address).await;
        assert!(
            result.is_ok(),
            "Failed to get ETH balance: {:?}",
            result.err()
        );

        let balance = result.unwrap();
        // Vitalik's address should have some ETH
        assert!(balance > U256::ZERO, "Expected non-zero balance");
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_get_eth_balance_random_address_should_work() {
        rate_limit_delay().await;
        let repo = create_test_repository();

        // A random address that likely has no balance
        let address = Address::from_str(RANDOM_ADDRESS).expect("Invalid address");

        let result = repo.get_eth_balance(address).await;
        assert!(
            result.is_ok(),
            "Failed to get ETH balance: {:?}",
            result.err()
        );

        // Just verify we can get a balance (may be zero or non-zero)
        let _balance = result.unwrap();
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_get_erc20_balance_usdt_should_work() {
        rate_limit_delay().await;
        let repo = create_test_repository();

        // USDT contract address
        let token = Address::from_str(USDT_CONTRACT).expect("Invalid token address");

        // Binance hot wallet - known to hold USDT
        let owner = Address::from_str(BINANCE_HOT_WALLET).expect("Invalid owner address");

        let result = repo.get_erc20_balance(token, owner).await;
        assert!(
            result.is_ok(),
            "Failed to get USDT balance: {:?}",
            result.err()
        );

        let token_balance = result.unwrap();
        assert_eq!(token_balance.decimals, 6, "USDT should have 6 decimals");
        assert_eq!(token_balance.symbol, "USDT", "Symbol should be USDT");
        assert!(
            token_balance.balance > U256::ZERO,
            "Expected non-zero USDT balance"
        );
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_get_token_metadata_dai_should_work() {
        rate_limit_delay().await;
        let repo = create_test_repository();

        // DAI contract address
        let token = Address::from_str(DAI_CONTRACT).expect("Invalid token address");

        let result = repo.get_token_metadata(token).await;
        assert!(
            result.is_ok(),
            "Failed to get DAI metadata: {:?}",
            result.err()
        );

        let metadata = result.unwrap();
        assert_eq!(metadata.decimals, 18, "DAI should have 18 decimals");
        assert_eq!(metadata.symbol, "DAI", "Symbol should be DAI");
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_get_gas_price() {
        rate_limit_delay().await;
        let repo = create_test_repository();

        let result = repo.get_gas_price().await;
        assert!(
            result.is_ok(),
            "Failed to get gas price: {:?}",
            result.err()
        );

        let gas_price = result.unwrap();
        // Gas price should be positive (at least 1 wei)
        assert!(
            gas_price > 0,
            "Expected positive gas price, got: {gas_price}"
        );

        // Sanity check: gas price should be less than 1000 Gwei (1000 * 10^9 wei)
        // This is a reasonable upper bound for normal network conditions
        assert!(
            gas_price < 1_000_000_000_000,
            "Gas price seems unreasonably high: {gas_price}",
        );
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_get_erc20_balance_invalid_contract_should_return_error() {
        rate_limit_delay().await;
        let repo = create_test_repository();

        // Invalid contract address (not an ERC20)
        let token = Address::from_str(INVALID_CONTRACT).expect("Invalid token address");

        let owner = Address::from_str(VITALIK_ADDRESS).expect("Invalid owner address");

        let result = repo.get_erc20_balance(token, owner).await;
        // Should return an error because the address is not a valid ERC20 contract
        assert!(result.is_err(), "Expected error for invalid ERC20 contract");

        if let Err(e) = result {
            match e {
                RepositoryError::ContractError(_) => {}
                _ => panic!("Expected ContractError, got: {:?}", e),
            }
        }
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_get_token_metadata_invalid_contract_should_return_error() {
        rate_limit_delay().await;
        let repo = create_test_repository();

        let token = Address::from_str(INVALID_CONTRACT).expect("Invalid token address");

        let result = repo.get_token_metadata(token).await;
        assert!(result.is_err(), "Expected error for invalid ERC20 contract");

        if let Err(e) = result {
            match e {
                RepositoryError::ContractError(_) => {}
                _ => panic!("Expected ContractError, got: {:?}", e),
            }
        }
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_get_eth_usd_price_should_work() {
        rate_limit_delay().await;
        let repo = create_test_repository();

        let result = repo.get_eth_usd_price().await;

        match result {
            Ok(eth_price) => {
                println!("✅ ETH/USD Price from Uniswap V2 USDC/WETH pair:");
                println!("   Price: ${}", eth_price);

                // Sanity checks: ETH price should be between $500 and $10,000
                let min_price = Decimal::from(500);
                let max_price = Decimal::from(10000);
                assert!(
                    eth_price > min_price && eth_price < max_price,
                    "ETH price seems unreasonable: ${}",
                    eth_price
                );
            }
            Err(RepositoryError::ContractError(msg))
                if msg.contains("429") || msg.contains("rate limit") =>
            {
                println!("⚠️  Rate limited by RPC provider - test skipped");
                println!("   Error: {}", msg);
                // Rate limiting is expected when running tests, skip this test
            }
            Err(e) => {
                panic!("Failed to get ETH/USD price with unexpected error: {:?}", e);
            }
        }
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_get_uniswap_pair_reserves_should_work() {
        rate_limit_delay().await;
        let repo = create_test_repository();

        // Test with USDC/WETH pair - one of the most liquid pairs
        let usdc = Address::from_str(USDC_CONTRACT).expect("Invalid USDC address");
        let weth = Address::from_str(WETH_CONTRACT).expect("Invalid WETH address");

        let result = repo.get_uniswap_pair_reserves(usdc, weth).await;
        assert!(
            result.is_ok(),
            "Failed to get pair reserves: {:?}",
            result.err()
        );

        let (reserve0, reserve1, token0, token1) = result.unwrap();
        println!("✅ USDC/WETH Pair Reserves:");
        println!("   Reserve 0: {}", reserve0);
        println!("   Reserve 1: {}", reserve1);
        println!("   Token 0: {}", token0);
        println!("   Token 1: {}", token1);

        // Both reserves should be non-zero for an active pair
        assert!(reserve0 > U256::ZERO, "Reserve 0 should be non-zero");
        assert!(reserve1 > U256::ZERO, "Reserve 1 should be non-zero");

        // Verify token addresses are returned correctly
        assert!(
            token0 == usdc || token0 == weth,
            "Token0 should be USDC or WETH"
        );
        assert!(
            token1 == usdc || token1 == weth,
            "Token1 should be USDC or WETH"
        );
        assert!(token0 != token1, "Token0 and Token1 should be different");
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_get_uniswap_pair_reserves_nonexistent_pair_should_fail() {
        rate_limit_delay().await;
        let repo = create_test_repository();

        // Try to get reserves for a pair that doesn't exist
        let token1 = Address::from_str(INVALID_CONTRACT).expect("Invalid address");
        let token2 = Address::from_str(RANDOM_ADDRESS).expect("Invalid address");

        let result = repo.get_uniswap_pair_reserves(token1, token2).await;
        assert!(result.is_err(), "Expected error for non-existent pair");

        if let Err(e) = result {
            match e {
                RepositoryError::ContractError(msg) => {
                    assert!(
                        msg.contains("No Uniswap V2 pair found"),
                        "Expected pair not found error"
                    );
                }
                _ => panic!("Expected ContractError, got: {:?}", e),
            }
        }
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_get_swap_amounts_out_should_work() {
        rate_limit_delay().await;
        let repo = create_test_repository();

        // Test swap from USDC to WETH
        let usdc = Address::from_str(USDC_CONTRACT).expect("Invalid USDC address");
        let weth = Address::from_str(WETH_CONTRACT).expect("Invalid WETH address");

        // Swap 1000 USDC (USDC has 6 decimals)
        let amount_in = U256::from(1000) * U256::from(10u64).pow(U256::from(6u64));
        let path = vec![usdc, weth];

        let result = repo.get_swap_amounts_out(amount_in, path).await;
        assert!(
            result.is_ok(),
            "Failed to get swap amounts: {:?}",
            result.err()
        );

        let amounts = result.unwrap();
        println!("✅ Swap Amounts Out (1000 USDC -> WETH):");
        println!("   Input: {} USDC", amounts[0]);
        println!("   Output: {} WETH", amounts[1]);

        // Should return 2 amounts (input and output)
        assert_eq!(amounts.len(), 2, "Should return 2 amounts");
        assert_eq!(amounts[0], amount_in, "First amount should equal input");
        assert!(amounts[1] > U256::ZERO, "Output amount should be non-zero");
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_get_swap_amounts_out_multi_hop_should_work() {
        rate_limit_delay().await;
        let repo = create_test_repository();

        // Test multi-hop swap: USDC -> WETH -> DAI
        let usdc = Address::from_str(USDC_CONTRACT).expect("Invalid USDC address");
        let weth = Address::from_str(WETH_CONTRACT).expect("Invalid WETH address");
        let dai = Address::from_str(DAI_CONTRACT).expect("Invalid DAI address");

        // Swap 1000 USDC
        let amount_in = U256::from(1000) * U256::from(10u64).pow(U256::from(6u64));
        let path = vec![usdc, weth, dai];

        let result = repo.get_swap_amounts_out(amount_in, path).await;
        assert!(
            result.is_ok(),
            "Failed to get multi-hop swap amounts: {:?}",
            result.err()
        );

        let amounts = result.unwrap();
        println!("✅ Multi-hop Swap Amounts (USDC -> WETH -> DAI):");
        for (i, amount) in amounts.iter().enumerate() {
            println!("   Amount {}: {}", i, amount);
        }

        // Should return 3 amounts for 3 tokens in path
        assert_eq!(amounts.len(), 3, "Should return 3 amounts for 3-token path");
        assert_eq!(amounts[0], amount_in, "First amount should equal input");
        assert!(
            amounts[1] > U256::ZERO,
            "Intermediate amount should be non-zero"
        );
        assert!(amounts[2] > U256::ZERO, "Final output should be non-zero");
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_simulate_swap_should_handle_transfer_failure() {
        rate_limit_delay().await;
        let repo = create_test_repository();

        // Test swap simulation from USDC to WETH
        let usdc = Address::from_str(USDC_CONTRACT).expect("Invalid USDC address");
        let weth = Address::from_str(WETH_CONTRACT).expect("Invalid WETH address");
        let from = Address::from_str(RANDOM_ADDRESS).expect("Invalid from address");

        // Swap 1000 USDC
        let amount_in = U256::from(1000) * U256::from(10u64).pow(U256::from(6u64));
        let amount_out_min = U256::from(1); // Very low minimum for testing
        let path = vec![usdc, weth];
        let deadline = U256::from(chrono::Utc::now().timestamp() + 3600);

        let result = repo
            .simulate_swap(from, amount_in, amount_out_min, path, deadline)
            .await;

        // This should fail because the address doesn't have USDC balance or approval
        // The important thing is that the RPC call works, even if it returns an error
        // We expect either success (unlikely) or a specific contract error
        match result {
            Ok(gas_estimate) => {
                println!("✅ Swap Simulation succeeded (unexpected but valid):");
                println!("   Estimated gas: {gas_estimate}");
                assert!(
                    gas_estimate > 50_000 && gas_estimate < 500_000,
                    "Gas estimate seems unreasonable: {gas_estimate}",
                );
            }
            Err(RepositoryError::ContractError(msg)) => {
                println!("✅ Swap Simulation failed as expected:");
                println!("   Error: {msg}");
                // Expected error - no balance, approval, or RPC issues
                assert!(
                    msg.contains("TRANSFER_FROM_FAILED")
                        || msg.contains("execution reverted")
                        || msg.contains("insufficient")
                        || msg.contains("no response")
                        || msg.contains("error code"),
                    "Expected transfer, balance, or RPC error, got: {msg}"
                );
            }
            Err(e) => {
                panic!("Unexpected error type: {:?}", e);
            }
        }
    }
}
