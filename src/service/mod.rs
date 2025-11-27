pub mod error;
pub mod token_registry;
pub mod trading;
pub mod types;
pub mod utils;

#[cfg(test)]
mod tests;

pub use error::ServiceError;
pub use token_registry::TokenRegistry;
pub use trading::EthereumTradingService;
pub use types::*;

pub(crate) type ServiceResult<T> = std::result::Result<T, ServiceError>;
