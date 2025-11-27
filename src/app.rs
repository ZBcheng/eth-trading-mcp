use std::time::Duration;

use axum::Router;
use axum::http::StatusCode;
use axum::routing::get;
use rmcp::transport::SseServer;
use rmcp::transport::sse_server::SseServerConfig;
use tokio_util::sync::CancellationToken;

use crate::config::Config;
use crate::middleware::trace::http_trace_layer;
use crate::service::EthereumTradingService;

pub fn build_app(cancellation_token: CancellationToken, config: Config) -> anyhow::Result<Router> {
    let addr = config.server_uri().parse()?;

    let sse_config = SseServerConfig {
        bind: addr,
        sse_path: "/sse".to_string(),
        post_path: "/message".to_string(),
        ct: cancellation_token,
        sse_keep_alive: Some(Duration::from_secs(15)),
    };

    let (sse_server, sse_router) = SseServer::new(sse_config);

    let eth_service = move || EthereumTradingService::new(&config);

    sse_server.with_service(eth_service);

    let app = Router::new()
        .route("/health", get(|| async move { StatusCode::OK }))
        .nest("/trading", sse_router)
        .layer(http_trace_layer());

    Ok(app)
}
