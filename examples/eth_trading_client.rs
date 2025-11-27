use eth_trading_mcp::config::Config;
use eth_trading_mcp::{GetBalanceRequest, GetTokenPriceRequest, SwapTokensRequest};
use rmcp::ServiceExt;
use rmcp::model::{CallToolRequestParam, ClientCapabilities, ClientInfo, Implementation};
use rmcp::transport::SseClientTransport;

/// Example wallet address (Vitalik's address)
const VITALIK_ADDRESS: &str = "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045";

/// USDT token contract address on Ethereum mainnet
const USDT_ADDRESS: &str = "0xdac17f958d2ee523a2206206994597c13d831ec7";

/// USDC token contract address on Ethereum mainnet
const USDC_ADDRESS: &str = "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48";

/// Example of calling the MCP server using rmcp's SSE client
///
/// This example demonstrates:
/// 1. Connecting to the MCP server via SSE transport
/// 2. Listing available tools
/// 3. Calling the get_balance tool
/// 4. Calling the get_token_price tool
/// 5. Calling the swap_tokens tool (V2)
/// 6. Calling the swap_tokens tool (V3)
/// 7. Comparing V2 vs V3 swap results
#[tokio::main]
async fn main() {
    let config = Config::from_yaml("config/default.yaml").await;
    let uri = format!("http://localhost:{}/trading/sse", config.server.port);

    let transport = SseClientTransport::start(uri.as_str())
        .await
        .expect("Failed to start SSE client transport");

    let client_info = ClientInfo {
        protocol_version: Default::default(),
        capabilities: ClientCapabilities::default(),
        client_info: Implementation {
            name: "eth-mcp-trading-client".to_string(),
            version: "0.1.0".to_string(),
            ..Default::default()
        },
    };

    let client = client_info
        .serve(transport)
        .await
        .inspect_err(|e| {
            eprintln!("client error: {e:?}");
        })
        .expect("Failed to serve client");

    println!("✓ Connected to MCP server at {}\n", uri);

    // 1. List available tools
    println!("=== Listing available tools ===");
    let tools_response = client.list_tools(None).await.expect("failed to list tools");

    println!("Available tools:");
    for tool in &tools_response.tools {
        let desc = tool
            .description
            .as_ref()
            .map(|s| s.as_ref())
            .unwrap_or("No description");

        println!("  - {}: {}", tool.name, desc);
    }
    println!();

    // 2. Get ETH balance
    println!("=== Getting ETH balance ===");
    let get_eth_balance_request = GetBalanceRequest {
        wallet_address: VITALIK_ADDRESS.to_string(),
        token_contract_address: None,
    };

    let arguments = serde_json::to_value(&get_eth_balance_request)
        .expect("failed to serialize get_eth_balance_request")
        .as_object()
        .cloned();

    let balance_result = client
        .call_tool(CallToolRequestParam {
            name: "get_balance".into(),
            arguments,
        })
        .await
        .expect("failed to call `get_balance`");

    println!("Balance result:");
    println!(
        "{}\n",
        serde_json::to_string_pretty(&balance_result).unwrap()
    );

    // 3. Get ERC20 token balance (USDT)
    println!("=== Getting USDT balance ===");
    let get_usdt_balance_request = GetBalanceRequest {
        wallet_address: VITALIK_ADDRESS.to_string(),
        token_contract_address: Some(USDT_ADDRESS.to_string()),
    };

    let arguments = serde_json::to_value(&get_usdt_balance_request)
        .expect("failed to serialize get_usdt_balance_request")
        .as_object()
        .cloned();

    let usdt_balance_result = client
        .call_tool(CallToolRequestParam {
            name: "get_balance".into(),
            arguments,
        })
        .await
        .expect("failed to call `get_balance`");

    println!("USDT balance result:");
    println!(
        "{}\n",
        serde_json::to_string_pretty(&usdt_balance_result).unwrap()
    );

    // 4. Get token price by symbol
    println!("=== Getting USDT price ===");
    let get_token_price_by_symbol_request = GetTokenPriceRequest::symbol("USDT");

    let arguments = serde_json::to_value(&get_token_price_by_symbol_request)
        .expect("failed to serialize get_token_price_by_symbol_request")
        .as_object()
        .cloned();

    let price_result = client
        .call_tool(CallToolRequestParam {
            name: "get_token_price".into(),
            arguments,
        })
        .await
        .expect("failed to call `get_token_price`");

    println!("Price result:");
    println!("{}\n", serde_json::to_string_pretty(&price_result).unwrap());

    // 5. Get token price by contract address
    println!("=== Getting USDC price by contract address ===");
    let get_token_price_by_contract_address_request =
        GetTokenPriceRequest::contract_address(USDC_ADDRESS);

    let arguments = serde_json::to_value(&get_token_price_by_contract_address_request)
        .expect("failed to serialize get_token_price_by_contract_address_request")
        .as_object()
        .cloned();

    let usdc_price_result = client
        .call_tool(CallToolRequestParam {
            name: "get_token_price".into(),
            arguments,
        })
        .await
        .expect("failed to call `get_token_price`");

    println!("USDC price result:");
    println!(
        "{}\n",
        serde_json::to_string_pretty(&usdc_price_result).unwrap()
    );

    // 6. Simulate a token swap
    println!("=== Simulating token swap ===");
    let swap_tokens_request = SwapTokensRequest {
        from_token: USDT_ADDRESS.to_string(),
        to_token: "ETH".to_string(),           // Use ETH symbol for WETH
        amount: "100".to_string(),             // 100 USDT (within balance)
        slippage_tolerance: "0.5".to_string(), // 0.5% slippage tolerance
        uniswap_version: Some("v2".to_string()),
        from_address: Some(VITALIK_ADDRESS.to_string()),
    };

    let arguments = serde_json::to_value(&swap_tokens_request)
        .expect("failed to serialize swap_tokens_request")
        .as_object()
        .cloned();

    let swap_result = client
        .call_tool(CallToolRequestParam {
            name: "swap_tokens".into(),
            arguments,
        })
        .await
        .expect("failed to call `swap_tokens`");

    println!("Swap simulation result:");
    println!("{}\n", serde_json::to_string_pretty(&swap_result).unwrap());

    // 7. Simulate a V3 swap
    println!("=== Simulating Uniswap V3 swap ===");
    let swap_v3_request = SwapTokensRequest {
        from_token: "USDC".to_string(),          // Use USDC symbol
        to_token: "WETH".to_string(),            // Swap to WETH
        amount: "1000".to_string(),              // 1000 USDC
        slippage_tolerance: "0.5".to_string(),   // 0.5% slippage tolerance
        uniswap_version: Some("v3".to_string()), // Use V3
        from_address: Some(VITALIK_ADDRESS.to_string()),
    };

    let arguments = serde_json::to_value(&swap_v3_request)
        .expect("failed to serialize swap_v3_request")
        .as_object()
        .cloned();

    let swap_v3_result = client
        .call_tool(CallToolRequestParam {
            name: "swap_tokens".into(),
            arguments,
        })
        .await
        .expect("failed to call `swap_tokens` with v3");

    println!("V3 Swap simulation result:");
    println!(
        "{}\n",
        serde_json::to_string_pretty(&swap_v3_result).unwrap()
    );

    // 8. Compare V2 vs V3 for the same swap
    println!("=== Comparing V2 vs V3 for USDC -> WETH swap ===");

    // V2 swap
    let swap_v2_compare = SwapTokensRequest {
        from_token: "USDC".to_string(),
        to_token: "WETH".to_string(),
        amount: "1000".to_string(),
        slippage_tolerance: "0.5".to_string(),
        uniswap_version: Some("v2".to_string()),
        from_address: None, // No simulation address for faster response
    };

    let arguments_v2 = serde_json::to_value(&swap_v2_compare)
        .expect("failed to serialize swap_v2_compare")
        .as_object()
        .cloned();

    let swap_v2_result = client
        .call_tool(CallToolRequestParam {
            name: "swap_tokens".into(),
            arguments: arguments_v2,
        })
        .await
        .expect("failed to call `swap_tokens` v2");

    // V3 swap
    let swap_v3_compare = SwapTokensRequest {
        from_token: "USDC".to_string(),
        to_token: "WETH".to_string(),
        amount: "1000".to_string(),
        slippage_tolerance: "0.5".to_string(),
        uniswap_version: Some("v3".to_string()),
        from_address: None,
    };

    let arguments_v3 = serde_json::to_value(&swap_v3_compare)
        .expect("failed to serialize swap_v3_compare")
        .as_object()
        .cloned();

    let swap_v3_result_compare = client
        .call_tool(CallToolRequestParam {
            name: "swap_tokens".into(),
            arguments: arguments_v3,
        })
        .await
        .expect("failed to call `swap_tokens` v3");

    println!("V2 Result:");
    println!("{}", serde_json::to_string_pretty(&swap_v2_result).unwrap());
    println!("\nV3 Result:");
    println!(
        "{}",
        serde_json::to_string_pretty(&swap_v3_result_compare).unwrap()
    );

    println!("\n=== All operations completed successfully ===");
}
