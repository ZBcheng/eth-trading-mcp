pub mod alloy;
pub mod contract;
pub mod error;

use ::alloy::primitives::{Address, U256};
pub use alloy::{AlloyEthereumRepository, TokenBalance, TokenMetadata};
use async_trait::async_trait;
pub use error::RepositoryError;
use rust_decimal::Decimal;

pub(crate) type RepoResult<T> = std::result::Result<T, RepositoryError>;

/// Trait for Ethereum blockchain data access operations.
///
/// This trait provides an abstraction layer for interacting with the Ethereum blockchain,
/// supporting operations such as querying balances, token metadata, and gas prices.
/// Implementations should handle RPC communication and error conversion.
#[async_trait]
pub trait EthereumRepository: Send + Sync {
    /// Retrieves the native ETH balance for a given address.
    ///
    /// # Arguments
    ///
    /// * `address` - The Ethereum address to query
    ///
    /// # Returns
    ///
    /// * `Ok(U256)` - The balance in wei (1 ETH = 10^18 wei)
    /// * `Err(RepositoryError)` - If the RPC call fails or network error occurs
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let balance = repository.get_eth_balance(address).await?;
    /// println!("Balance: {} wei", balance);
    /// ```
    async fn get_eth_balance(&self, address: Address) -> RepoResult<U256>;

    /// Retrieves the ERC20 token balance and metadata for a given token and owner.
    ///
    /// # Arguments
    ///
    /// * `token` - The ERC20 token contract address
    /// * `owner` - The address of the token holder
    ///
    /// # Returns
    ///
    /// * `Ok(TokenBalance)` - Contains balance (in token's smallest unit), decimals, and symbol
    /// * `Err(RepositoryError)` - If the contract call fails or the address is not a valid ERC20 contract
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let token_balance = repository.get_erc20_balance(usdt_address, wallet_address).await?;
    /// println!("{} {}", token_balance.balance, token_balance.symbol);
    /// ```
    async fn get_erc20_balance(&self, token: Address, owner: Address) -> RepoResult<TokenBalance>;

    /// Retrieves metadata for an ERC20 token contract.
    ///
    /// # Arguments
    ///
    /// * `token` - The ERC20 token contract address
    ///
    /// # Returns
    ///
    /// * `Ok(TokenMetadata)` - Contains decimals and symbol
    /// * `Err(RepositoryError)` - If the contract call fails or the address is not a valid ERC20 contract
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let metadata = repository.get_token_metadata(dai_address).await?;
    /// println!("{} has {} decimals", metadata.symbol, metadata.decimals);
    /// ```
    async fn get_token_metadata(&self, token: Address) -> RepoResult<TokenMetadata>;

    /// Retrieves the current gas price from the network.
    ///
    /// # Returns
    ///
    /// * `Ok(u128)` - The current gas price in wei
    /// * `Err(RepositoryError)` - If the RPC call fails or network error occurs
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let gas_price = repository.get_gas_price().await?;
    /// println!("Current gas price: {} gwei", gas_price / 1_000_000_000);
    /// ```
    async fn get_gas_price(&self) -> RepoResult<u128>;

    /// Retrieves the reserves from a Uniswap V2 pair contract.
    ///
    /// # Arguments
    ///
    /// * `token_a` - The address of the first token
    /// * `token_b` - The address of the second token
    ///
    /// # Returns
    ///
    /// * `Ok((U256, U256, Address, Address))` - Tuple containing:
    ///   - Reserve of token A
    ///   - Reserve of token B
    ///   - Address of token0 (from pair contract)
    ///   - Address of token1 (from pair contract)
    /// * `Err(RepositoryError)` - If the pair doesn't exist or contract call fails
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let (reserve_a, reserve_b, token0, token1) =
    ///     repository.get_uniswap_pair_reserves(usdt_address, weth_address).await?;
    /// let price = reserve_b as f64 / reserve_a as f64;
    /// ```
    async fn get_uniswap_pair_reserves(
        &self,
        token_a: Address,
        token_b: Address,
    ) -> RepoResult<(U256, U256, Address, Address)>;

    /// Retrieves the current ETH price in USD from Uniswap V2 USDC/WETH pair.
    ///
    /// Uses Decimal for precise financial calculations.
    ///
    /// # Returns
    ///
    /// * `Ok(Decimal)` - The current ETH price in USD with full precision
    /// * `Err(RepositoryError)` - If the pair doesn't exist or contract call fails
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let eth_price = repository.get_eth_usd_price().await?;
    /// println!("ETH price: ${}", eth_price);
    /// ```
    async fn get_eth_usd_price(&self) -> RepoResult<Decimal>;

    /// Retrieves the expected output amounts for a token swap from Uniswap V2 Router.
    ///
    /// # Arguments
    ///
    /// * `amount_in` - The input amount to swap
    /// * `path` - Array of token addresses representing the swap path
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<U256>)` - Array of amounts where the last element is the expected output
    /// * `Err(RepositoryError)` - If the router call fails or path is invalid
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let amounts = repository.get_swap_amounts_out(amount, vec![token_a, token_b]).await?;
    /// let output = amounts.last().unwrap();
    /// ```
    async fn get_swap_amounts_out(
        &self,
        amount_in: U256,
        path: Vec<Address>,
    ) -> RepoResult<Vec<U256>>;

    /// Simulates a swap transaction using eth_call to estimate gas and validate the swap.
    ///
    /// # Arguments
    ///
    /// * `from` - The sender address
    /// * `amount_in` - The input amount to swap
    /// * `amount_out_min` - The minimum output amount (for slippage protection)
    /// * `path` - Array of token addresses representing the swap path
    /// * `deadline` - Unix timestamp deadline for the swap
    ///
    /// # Returns
    ///
    /// * `Ok(u64)` - The estimated gas for the swap transaction
    /// * `Err(RepositoryError)` - If the simulation fails
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let gas = repository.simulate_swap(wallet, amount_in, min_out, path, deadline).await?;
    /// println!("Estimated gas: {}", gas);
    /// ```
    async fn simulate_swap(
        &self,
        from: Address,
        amount_in: U256,
        amount_out_min: U256,
        path: Vec<Address>,
        deadline: U256,
    ) -> RepoResult<u64>;

    /// Gets a quote for a Uniswap V3 swap using QuoterV2.
    ///
    /// # Arguments
    ///
    /// * `token_in` - The input token address
    /// * `token_out` - The output token address
    /// * `amount_in` - The input amount to swap
    /// * `fee` - The pool fee tier (500 for 0.05%, 3000 for 0.3%, 10000 for 1%)
    ///
    /// # Returns
    ///
    /// * `Ok((U256, u64))` - Tuple containing:
    ///   - The expected output amount
    ///   - The estimated gas for the swap
    /// * `Err(RepositoryError)` - If the quote fails
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let (amount_out, gas) = repository.get_v3_quote(token_a, token_b, amount, 3000).await?;
    /// println!("Expected output: {}, Gas: {}", amount_out, gas);
    /// ```
    async fn get_v3_quote(
        &self,
        token_in: Address,
        token_out: Address,
        amount_in: U256,
        fee: u32,
    ) -> RepoResult<(U256, u64)>;

    /// Simulates a Uniswap V3 swap transaction using eth_call to estimate gas and validate the swap.
    ///
    /// # Arguments
    ///
    /// * `from` - The sender address
    /// * `token_in` - The input token address
    /// * `token_out` - The output token address
    /// * `amount_in` - The input amount to swap
    /// * `amount_out_min` - The minimum output amount (for slippage protection)
    /// * `fee` - The pool fee tier (500 for 0.05%, 3000 for 0.3%, 10000 for 1%)
    /// * `deadline` - Unix timestamp deadline for the swap
    ///
    /// # Returns
    ///
    /// * `Ok(u64)` - The estimated gas for the swap transaction
    /// * `Err(RepositoryError)` - If the simulation fails
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let gas = repository.simulate_v3_swap(wallet, token_in, token_out, amount_in, min_out, 3000, deadline).await?;
    /// println!("Estimated gas: {}", gas);
    /// ```
    async fn simulate_v3_swap(
        &self,
        from: Address,
        token_in: Address,
        token_out: Address,
        amount_in: U256,
        amount_out_min: U256,
        fee: u32,
        deadline: U256,
    ) -> RepoResult<u64>;
}
