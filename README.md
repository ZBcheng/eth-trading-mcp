# ETH Trading MCP

An Ethereum trading service based on MCP (Model Context Protocol), providing balance queries, token price queries, and token swap functionality.

## Overview

This is an MCP server built with Rust that exposes Ethereum blockchain interaction capabilities through SSE (Server-Sent Events) transport protocol. It supports integration with AI Agents (such as Gemini) to provide intelligent Ethereum trading assistance.

**Core Features:**

- ✅ ETH and ERC20 token balance queries
- ✅ Support for 30+ mainstream tokens (USDT, USDC, UNI, AAVE, etc.)
- ✅ Token price queries and exchange rate calculations
- ✅ Token swap simulation (with slippage protection)
- ✅ MCP protocol compatible, seamlessly integrates with AI Agents

## Architecture

```text
eth-trading-mcp/
├── src/
│   ├── main.rs               # Service entry point
│   ├── lib.rs                # Library exports
│   ├── app.rs                # Application builder and routes
│   ├── config.rs             # Configuration management
│   ├── middleware/           # Middleware layer
│   │   ├── mod.rs
│   │   └── trace.rs          # HTTP tracing
│   ├── repository/           # Data access layer
│   │   ├── mod.rs
│   │   ├── alloy.rs          # Alloy Ethereum client
│   │   ├── contract.rs       # Smart contract interactions
│   │   └── error.rs          # Repository error definitions
│   └── service/              # Business logic layer
│       ├── mod.rs
│       ├── trading.rs        # MCP tool router
│       ├── token_registry.rs # Token symbol to address mapping
│       ├── types.rs          # Request/response types
│       ├── utils.rs          # Utility functions
│       ├── error.rs          # Service error definitions
│       └── tests.rs          # Tests
├── config/
│   ├── default.yaml          # Default configuration
│   └── test.yaml             # Test configuration
└── examples/
    ├── eth_trading_client.rs        # MCP client example
    └── interactive_gemini_agent.rs  # Gemini Agent integration example
```

**Layer Description:**

- **Repository Layer**: Encapsulates Ethereum on-chain data access (Alloy)
  - `AlloyEthereumRepository`: Ethereum RPC call wrapper
  - `ERC20Contract`: ERC20 token standard contract interactions
  - Supports read-only mode and signing mode (requires private key)

- **Service Layer**: Implements business logic and MCP tool registration
  - `EthereumTradingService`: MCP tool router + business implementation
  - `tool_router` macro: Auto-generates MCP tool registration code
  - Unified error handling (`Result<T, ServiceError>`)
  - Token symbol registry (30+ mainstream tokens)

- **Middleware Layer**: HTTP tracing, logging, and cross-cutting concerns
  - `http_trace_layer`: Request tracing based on tower-http
  - Structured logging (tracing)

- **App Layer**: SSE server configuration and route assembly
  - SSE transport layer configuration (Keep-Alive: 15s)
  - Health check endpoint: `/health`
  - MCP endpoint: `/trading/sse`

### Data Flow

```text
AI Agent (Gemini)
      │
      ├─ 1. MCP Request (SSE)
      ↓
MCP Client (rmcp)
      │
      ├─ 2. HTTP POST /trading/message
      ↓
SSE Server (Axum)
      │
      ├─ 3. Tool Router
      ↓
EthereumTradingService
      │
      ├─ 4. Business Logic
      ↓
Repository Layer
      │
      ├─ 5. Alloy RPC Call
      ↓
Ethereum RPC Node
      │
      ├─ 6. On-Chain Data
      ↓
Blockchain
```

### Core Design Patterns

#### 1. Protocol Layer Separation

Separate MCP protocol adaptation from business logic using the `#[tool_router]` macro:

```rust
#[tool_router]  // MCP protocol layer
impl EthereumTradingService {
    #[tool(description = "Query ETH balance")]
    pub async fn get_balance(...) -> Json<Result> {
        // Protocol adaptation: parameter parsing + error wrapping
        match self.get_balance_impl(req).await {
            Ok(resp) => Json(Result::Success(resp)),
            Err(e) => Json(Result::Error { error: e })
        }
    }
}

impl EthereumTradingService {
    async fn get_balance_impl(...) -> ServiceResult<Response> {
        // Pure business logic (protocol-agnostic)
    }
}
```

#### 2. Dependency Injection

Repository layer abstracted through traits, supporting test mocking:

```rust
#[async_trait]
pub trait EthereumRepository: Send + Sync {
    async fn get_balance(&self, address: Address) -> Result<U256>;
    async fn get_token_balance(&self, token: Address, owner: Address) -> Result<U256>;
}
```

#### 3. Unified Error Handling

Three-layer error mapping: `RepositoryError` → `ServiceError` → `MCP Result`

> 🔧 **Architecture Extensibility**:
> When protocol extension is needed, `*_impl` methods can be extracted into independent Services, implementing higher-level abstraction over the protocol layer, supporting MCP, gRPC, and REST simultaneously

## Quick Start

### Prerequisites

- Rust 1.70+
- Environment variable configuration

### Configuration

#### 1. Create Environment Variable File

Generate `.env` file from example:

```bash
cp .env.example .env
```

Edit `.env` file to configure log level and private key:

```dotenv
# Log level configuration (highest priority, overrides code defaults)
# Format: RUST_LOG=<global_level>,<module>=<level>
RUST_LOG=debug,alloy=info,rmcp=info

# Optional: Wallet private key for signing transactions
# Warning: Never commit real private keys to version control!
WALLET_PRIVATE_KEY=0x1234...
```

> 📋 **Log Level Settings**:
>
> - **Priority 1**: `RUST_LOG` environment variable in `.env` file (recommended)
> - **Priority 2**: Default `env_filter` value in `src/main.rs` at startup (`"debug,alloy=info,rmcp=info"`)
> - Recommended to configure in `.env` for different log levels in different environments
>

#### 2. Configuration File Description

Configuration items in `config/default.yaml` support environment variable injection (via `${VAR_NAME}` syntax):

```yaml
server:
  host: 0.0.0.0
  port: 8000

rpc:
  url: https://eth.llamarpc.com  # Ethereum RPC node

wallet:
  private_key: ${WALLET_PRIVATE_KEY}  # Injected from .env
```

> 💡 Environment variables in `.env` file are automatically injected into configuration files for easier sensitive information management.
> 🔗 **Changing RPC Node**: Directly modify the `rpc.url` field in `config/default.yaml`. Common nodes:
>
> - LlamaRPC: `https://eth.llamarpc.com` (default)
> - Ankr: `https://rpc.ankr.com/eth`
>

### Start Server

```bash
# Development mode
cargo run

# Release mode
cargo run --release
```

Server will start at `http://0.0.0.0:8000`, MCP SSE endpoint is `/trading/sse`.

## API Reference

The service exposes three MCP tools through the `/trading/sse` endpoint.

### 1. get_balance

**Description:** Query ETH and ERC20 token balances

**Request:**

```json
{
  "wallet_address": "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045",
  "token_contract_address": "0xdAC17F958D2ee523a2206206994597C13D831ec7"
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `wallet_address` | string | ✅ | Wallet address to query balance for |
| `token_contract_address` | string | ❌ | Optional ERC20 token contract address. If not provided, returns ETH balance |

**Response (Success):**

```json
{
  "balance": "1000000000",
  "formatted_balance": "1000.0",
  "decimals": 6,
  "symbol": "USDT"
}
```

| Field | Type | Description |
|-------|------|-------------|
| `balance` | string | Raw balance value |
| `formatted_balance` | string | Balance formatted with proper decimals |
| `decimals` | u8 (number) | Token decimals |
| `symbol` | string | Token symbol (ETH or token symbol) |

**Response (Error):**

```json
{
  "error": {
    // ServiceError object (see Error Types section)
  }
}
```

---

### 2. get_token_price

**Description:** Get current token price in USD or ETH

**Request (by symbol):**

```json
{
  "symbol": "USDC"
}
```

**Request (by contract address):**

```json
{
  "contract_address": "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `symbol` | string | One of | Query by token symbol (e.g., "ETH", "USDT", "BTC") |
| `contract_address` | string | the two | Query by token contract address |

**Response (Success):**

```json
{
  "symbol": "USDC",
  "address": "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48",
  "price_usd": "0.9998",
  "price_eth": "0.0003305",
  "timestamp": 1705315800
}
```

| Field | Type | Description |
|-------|------|-------------|
| `symbol` | string | Token symbol |
| `address` | string | Token contract address |
| `price_usd` | string | Price in USD |
| `price_eth` | string | Price in ETH |
| `timestamp` | i64 (number) | Unix timestamp of the price data |

**Response (Error):**

```json
{
  "error": {
    // ServiceError object (see Error Types section)
  }
}
```

---

### 3. swap_tokens

**Description:** Execute a token swap simulation on Uniswap V2 or V3.

**Request:**

```json
{
  "from_token": "USDC",
  "to_token": "WETH",
  "amount": "1000",
  "slippage_tolerance": "0.5",
  "uniswap_version": "v2",
  "from_address": "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045"
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `from_token` | string | ✅ | Source token symbol or address (e.g., "ETH", "WETH", or "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2") |
| `to_token` | string | ✅ | Destination token symbol or address (e.g., "USDC", "DAI", or "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48") |
| `amount` | string | ✅ | Amount to swap in human-readable format (e.g., "1" for 1 ETH, "100.5" for 100.5 USDC). This will be automatically converted to the token's smallest unit based on its decimals |
| `slippage_tolerance` | string | ✅ | Slippage tolerance in percentage (e.g., "0.5" for 0.5%, "2" for 2%) |
| `uniswap_version` | string | ❌ | Optional: Uniswap version to use ("v2" or "v3", defaults to "v2") |
| `from_address` | string | ❌ | Optional: Wallet address for simulation (defaults to a standard address) |

**Response (Success):**

```json
{
  "estimated_output": "0.3305",
  "estimated_output_raw": "330500000000000000",
  "minimum_output": "0.3288",
  "estimated_gas": "150000",
  "estimated_gas_eth": "0.003825",
  "price_impact": "0.12",
  "exchange_rate": "0.0003305",
  "transaction_data": "Swap simulation (V2): 0xA0b8... -> 0xC02a..."
}
```

| Field | Type | Description |
|-------|------|-------------|
| `estimated_output` | string | Estimated output amount (formatted with decimals) |
| `estimated_output_raw` | string | Estimated output amount (raw) |
| `minimum_output` | string | Minimum output amount after slippage (formatted) |
| `estimated_gas` | string | Estimated gas cost in wei |
| `estimated_gas_eth` | string | Estimated gas cost in ETH |
| `price_impact` | string | Price impact percentage |
| `exchange_rate` | string | Exchange rate (from_token per to_token) |
| `transaction_data` | string | Transaction data (for reference, not for execution) |

**Response (Error):**

```json
{
  "error": {
    // ServiceError object (see Error Types section)
  }
}
```

## Testing

Project contains unit tests and integration tests. Tests that interact with the blockchain are marked with `#[ignore]` by default.

```bash
# Run non-ignored tests only (unit tests without blockchain interaction)
cargo test

# Run all tests including ignored blockchain interaction tests
cargo test -- --ignored

# Run a specific ignored test
cargo test test_get_balance_with_eth_should_work -- --ignored --nocapture

# Run all tests (both ignored and non-ignored)
cargo test -- --include-ignored
```

> ⚠️ **Why are blockchain tests ignored?**
>
> - Tests that interact with Ethereum RPC nodes are marked with `#[ignore]` to prevent rate limiting issues during normal test runs
> - Free RPC providers (like LlamaRPC) have strict rate limits, and running all tests simultaneously may trigger HTTP 429 errors
> - You need to manually run these tests individually or with delays to avoid hitting rate limits
>
> **Running blockchain interaction tests:**
>
> ```bash
> # Run all ignored tests (includes automatic 1-second delays between tests)
> cargo test -- --ignored --test-threads=1
>
> # Run a specific blockchain test
> cargo test repository::alloy::tests::test_get_token_metadata_dai_should_work -- --ignored --nocapture
>
> # Run all service layer blockchain tests
> cargo test service::tests -- --ignored --nocapture
> ```
>
> **Tips:**
>
> - Use `--test-threads=1` to run tests sequentially and avoid rate limiting
> - Tests include automatic delays (configurable via `TEST_DELAY_MS` in `src/repository/alloy.rs`)
> - If you still encounter rate limiting, wait a few minutes before retrying
> - Consider using a paid RPC provider for extensive testing

## Examples

> ⚠️ **Important**: Before running examples, start the server in another terminal: `cargo run`

### 1. MCP Client Example

**Step 1**: Start the server

```bash
# Terminal 1: Start MCP server
cargo run
```

**Step 2**: Run client example

```bash
# Terminal 2: Run client example
cargo run --example eth_trading_client
```

This example demonstrates:

- Connecting to MCP SSE server
- Querying Vitalik's ETH balance
- Querying USDT/USDC price
- Simulating token swaps

### 2. Gemini Agent Integration

**Step 1**: Start the server (if not already running)

```bash
# Terminal 1: Start MCP server
cargo run
```

**Step 2**: Run interactive AI Agent

```bash
# Terminal 2: Set Gemini API Key and run
export GEMINI_API_KEY=your_api_key
cargo run --example interactive_gemini_agent
```

Example interaction:

```text
💬 Enter your question: Check the ETH balance of this wallet: 0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045
🤖 Response: The wallet `0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045` holds 6.7625 ETH.

💬 Enter your question: What's the current price of ETH in USD?
🤖 Response: The current price of ETH is $3024.58.

💬 Enter your question: What's the USDT to ETH exchange rate?
🤖 Response: The current exchange rate for USDT to ETH is approximately 0.0003305 ETH per USDT.
```

## Tech Stack

- **Alloy**: Ethereum interaction library
- **Axum**: HTTP server framework
- **RMCP**: MCP protocol implementation
- **Tokio**: Async runtime
- **Rig**: AI Agent framework (examples)

## License

MIT
