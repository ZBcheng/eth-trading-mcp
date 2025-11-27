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
