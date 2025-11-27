pub mod app;
pub mod config;
pub mod middleware;
pub mod repository;
pub mod service;

pub use app::build_app;

// Re-export commonly used types for tests
pub use service::{
    EthereumTradingService, GetBalanceRequest, GetBalanceResponse, GetTokenPriceRequest,
    GetTokenPriceResponse, SwapTokensRequest, SwapTokensResponse,
};
