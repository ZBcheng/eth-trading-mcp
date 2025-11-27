use rmcp::handler::server::wrapper::Parameters;
use tokio::time::{Duration, sleep};

use crate::config::Config;
use crate::service::trading::EthereumTradingService;
use crate::service::types::{
    GetBalanceRequest, GetBalanceResult, GetTokenPriceRequest, GetTokenPriceResult,
};

// Vitalik Buterin's address
const WALLET_ADDRESS: &str = "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045";

// ERC20 Token Contract Addresses (Ethereum Mainnet)
const USDT_CONTRACT_ADDRESS: &str = "0xdac17f958d2ee523a2206206994597c13d831ec7";

/// Helper function to load test configuration
async fn get_test_config() -> Config {
    Config::from_yaml("config/test.yaml").await
}

/// Add delay between tests to avoid rate limiting
async fn avoid_rate_limit() {
    sleep(Duration::from_millis(500)).await;
}

#[tokio::test]
#[serial_test::serial]
#[ignore]
async fn test_get_balance_with_eth_should_work() {
    avoid_rate_limit().await;
    let config = get_test_config().await;
    let service = EthereumTradingService::new(&config);
    let params = Parameters(GetBalanceRequest {
        wallet_address: WALLET_ADDRESS.to_string(),
        token_contract_address: None,
    });

    let result = service.get_balance(params).await.0;
    match result {
        GetBalanceResult::Success(resp) => {
            println!("✅ ETH Balance Response:");
            println!("   Address: {}", WALLET_ADDRESS);
            println!("   Balance: {} wei", resp.balance);
            println!("   Formatted: {} ETH", resp.formatted_balance);
            println!("   Decimals: {}", resp.decimals);
            println!("   Symbol: {}", resp.symbol);

            // Verify it's real data (not mock)
            assert_eq!(resp.decimals, 18);
            assert_eq!(resp.symbol, "ETH");
        }
        GetBalanceResult::Error { error } => {
            panic!("Expected success but got error: {}", error);
        }
    }
}

#[tokio::test]
#[serial_test::serial]
#[ignore]
async fn test_get_balance_with_erc20_token_should_work() {
    avoid_rate_limit().await;
    let config = get_test_config().await;
    let service = EthereumTradingService::new(&config);
    let params = Parameters(GetBalanceRequest {
        wallet_address: WALLET_ADDRESS.to_string(),
        token_contract_address: Some(USDT_CONTRACT_ADDRESS.to_string()),
    });

    let result = service.get_balance(params).await.0;
    match result {
        GetBalanceResult::Success(resp) => {
            println!("✅ USDT Balance Response:");
            println!("   Address: {}", WALLET_ADDRESS);
            println!("   Token: {} ({})", resp.symbol, USDT_CONTRACT_ADDRESS);
            println!("   Balance: {} (raw)", resp.balance);
            println!("   Formatted: {} {}", resp.formatted_balance, resp.symbol);
            println!("   Decimals: {}", resp.decimals);

            // Verify it's real USDT data
            assert_eq!(resp.symbol, "USDT");
            assert_eq!(resp.decimals, 6); // USDT uses 6 decimals
        }
        GetBalanceResult::Error { error } => {
            panic!("Expected success but got error: {}", error);
        }
    }
}

#[tokio::test]
#[serial_test::serial]
#[ignore]
async fn test_get_balance_with_invalid_address_should_return_error() {
    avoid_rate_limit().await;
    let config = get_test_config().await;
    let service = EthereumTradingService::new(&config);
    let params = Parameters(GetBalanceRequest {
        wallet_address: "invalid_address".to_string(),
        token_contract_address: None,
    });

    let result = service.get_balance(params).await.0;
    match result {
        GetBalanceResult::Success(_) => {
            panic!("Expected error but got success");
        }
        GetBalanceResult::Error { error } => {
            println!("✅ Got expected error: {}", error);
            // Verify it's an InvalidWalletAddress error
            match error {
                super::error::ServiceError::InvalidWalletAddress(_) => {
                    println!("   Error type: InvalidWalletAddress ✓");
                }
                _ => panic!("Expected InvalidWalletAddress error, got: {:?}", error),
            }
        }
    }
}

#[tokio::test]
#[serial_test::serial]
#[ignore]
async fn test_get_token_price_usdc_should_work() {
    avoid_rate_limit().await;
    let config = get_test_config().await;
    let service = EthereumTradingService::new(&config);
    let params = Parameters(GetTokenPriceRequest::Symbol {
        symbol: "USDC".to_string(),
    });

    let result = service.get_token_price(params).await.0;
    match result {
        GetTokenPriceResult::Success(resp) => {
            println!("✅ USDC Price Response:");
            println!("   Symbol: {}", resp.symbol);
            println!("   Address: {}", resp.address);
            println!("   Price in USD: ${}", resp.price_usd);
            println!("   Price in ETH: {} ETH", resp.price_eth);
            println!("   Timestamp: {}", resp.timestamp);
            println!();
            println!("💡 Usage Examples:");
            println!(
                "   - 'What's the current price of USDC in USD?' → ${}",
                resp.price_usd
            );
            println!(
                "   - 'What's the current price of USDC in ETH?' → {} ETH",
                resp.price_eth
            );
            println!("   - To convert: 1 USDC = {} ETH", resp.price_eth);

            assert_eq!(resp.symbol, "USDC");
            // USDC should be close to $1
            let price_usd: f64 = resp.price_usd.parse().unwrap_or(0.0);
            assert!(
                price_usd > 0.9 && price_usd < 1.1,
                "USDC price should be close to $1"
            );
        }
        GetTokenPriceResult::Error { error } => {
            panic!("Expected success but got error: {}", error);
        }
    }
}

#[tokio::test]
#[serial_test::serial]
#[ignore]
async fn test_get_token_price_eth_should_work() {
    avoid_rate_limit().await;
    let config = get_test_config().await;
    let service = EthereumTradingService::new(&config);
    let params = Parameters(GetTokenPriceRequest::Symbol {
        symbol: "ETH".to_string(),
    });

    let result = service.get_token_price(params).await.0;
    match result {
        GetTokenPriceResult::Success(resp) => {
            println!("✅ ETH Price Response:");
            println!("   Symbol: {}", resp.symbol);
            println!("   Address: {}", resp.address);
            println!("   Price in USD: ${}", resp.price_usd);
            println!("   Price in ETH: {} ETH", resp.price_eth);
            println!("   Timestamp: {}", resp.timestamp);
            println!();
            println!("💡 ETH is the base currency, so price_eth = 1.0");

            assert_eq!(resp.symbol, "ETH");
            assert_eq!(resp.price_eth, "1.0");
            // ETH price should be reasonable (between $500 and $10000)
            let price_usd: f64 = resp.price_usd.parse().unwrap_or(0.0);
            assert!(
                price_usd > 500.0 && price_usd < 10000.0,
                "ETH price should be reasonable"
            );
        }
        GetTokenPriceResult::Error { error } => {
            panic!("Expected success but got error: {}", error);
        }
    }
}
