use std::io::{self, Write};

use eth_trading_mcp::config::Config;
use rig::agent::stream_to_stdout;
use rig::client::completion::CompletionClientDyn;
use rig::providers::gemini;
use rig::providers::gemini::completion::gemini_api_types::{
    AdditionalParameters, GenerationConfig,
};
use rig::streaming::StreamingPrompt;
use rmcp::ServiceExt;
use rmcp::model::{ClientCapabilities, ClientInfo, Implementation, Tool};
use rmcp::transport::SseClientTransport;

#[tokio::main]
async fn main() {
    let gemini_client = gemini::Client::from_env();

    let gen_cfg = GenerationConfig::default();

    let additional_parameters = AdditionalParameters::default().with_config(gen_cfg);
    let cfg = serde_json::to_value(&additional_parameters)
        .expect("failed to serialize AdditionalParameters");

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
            tracing::error!("client error: {e:?}");
        })
        .expect("Failed to serve client");

    let tools: Vec<Tool> = client
        .list_tools(Default::default())
        .await
        .expect("failed to list mcp tools")
        .tools;

    let agent = gemini_client
        .agent("gemini-2.5-flash")
        .preamble(
            "You are a helpful Ethereum trading assistant with access to blockchain tools.\n\n\
            Guidelines:\n\
            - When calling a tool, always mention its name naturally\n\
            - Explain results in a clear, conversational way\n\
            - Provide context for numbers (e.g., '1000 USDT' not just '1000')\n\
            ",
        )
        .additional_params(cfg)
        .rmcp_tools(tools, client.peer().to_owned())
        .build();

    println!("🤖 Gemini Agent started! Type 'exit' or 'quit' to exit.\n");

    loop {
        print!("💬 Enter your question: ");
        io::stdout().flush().expect("Failed to flush stdout");

        let mut input = String::new();
        io::stdin()
            .read_line(&mut input)
            .expect("Failed to read line");

        let input = input.trim();

        if input.is_empty() {
            continue;
        }

        if input.eq_ignore_ascii_case("exit") || input.eq_ignore_ascii_case("quit") {
            println!("👋 Goodbye!");
            break;
        }

        let mut stream = agent.stream_prompt(input).await;

        let _ = stream_to_stdout(&mut stream)
            .await
            .expect("failed to print stream to stdout");

        println!("\n");
    }
}
