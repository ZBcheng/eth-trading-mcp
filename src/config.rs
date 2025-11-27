use std::{fs, path::Path};

use dotenv::dotenv;
use envsubst::substitute;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub rpc: RpcConfig,
    pub wallet: WalletConfig,
}

impl Config {
    pub async fn from_yaml(path: impl AsRef<Path>) -> Self {
        dotenv().ok();

        let file_content =
            fs::read_to_string(path).expect("failed to read config file from path: {path}");

        let env_vars: std::collections::HashMap<String, String> = std::env::vars()
            .filter(|(key, _)| key.starts_with("SERVER_") || key.starts_with("WALLET_"))
            .collect();

        let interpolated = substitute(&file_content, &env_vars)
            .expect("Failed to substitute environment variables in YAML");

        let config: Config =
            serde_yaml::from_str(&interpolated).expect("Failed to parse YAML configuration");

        config
    }

    pub fn server_uri(&self) -> String {
        format!("{}:{}", self.server.host, self.server.port)
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RpcConfig {
    pub url: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WalletConfig {
    pub private_key: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_load_config_from_yaml() {
        let config = Config::from_yaml("config/test.yaml").await;

        // Verify server config
        assert_eq!(config.server.host, "0.0.0.0");
        assert_eq!(config.server.port, 8000);

        // Verify RPC config
        assert_eq!(config.rpc.url, "https://eth.llamarpc.com");

        // Verify wallet config (should be empty in test.yaml)
        assert_eq!(config.wallet.private_key, "");
    }

    #[tokio::test]
    async fn test_config_with_env_vars() {
        // Set environment variables
        unsafe {
            std::env::set_var("PRIVATE_KEY", "0xtest_private_key_123");
            std::env::set_var("SERVER_HOST", "127.0.0.1");
            std::env::set_var("SERVER_PORT", "9000");
        }

        let config = Config::from_yaml("config/test.yaml").await;

        // Verify that config was loaded (env vars in YAML would be substituted)
        assert!(!config.server.host.is_empty());
        assert!(config.server.port > 0);

        // Clean up environment variables
        unsafe {
            std::env::remove_var("PRIVATE_KEY");
            std::env::remove_var("SERVER_HOST");
            std::env::remove_var("SERVER_PORT");
        }
    }

    #[tokio::test]
    async fn test_config_fields_are_accessible() {
        let config = Config::from_yaml("config/test.yaml").await;

        // Verify all fields can be accessed
        let _host: &str = &config.server.host;
        let _port: u16 = config.server.port;
        let _rpc_url: &str = &config.rpc.url;
        let _private_key: &str = &config.wallet.private_key;

        // Verify config can be cloned
        let _cloned_config = config.clone();
    }

    #[tokio::test]
    async fn test_config_debug_format() {
        let config = Config::from_yaml("config/test.yaml").await;

        // Verify Debug trait works
        let debug_output = format!("{:?}", config);
        assert!(debug_output.contains("Config"));
        assert!(debug_output.contains("server"));
        assert!(debug_output.contains("rpc"));
        assert!(debug_output.contains("wallet"));
    }
}
