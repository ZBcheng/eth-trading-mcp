use std::collections::HashMap;

/// Common ERC20 token contract addresses on Ethereum mainnet

// Stablecoins
const USDT_ADDRESS: &str = "0xdac17f958d2ee523a2206206994597c13d831ec7";
const USDC_ADDRESS: &str = "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48";
const DAI_ADDRESS: &str = "0x6b175474e89094c44da98b954eedeac495271d0f";
const BUSD_ADDRESS: &str = "0x4fabb145d64652a948d72533023f6e7a623c7c53";
const FRAX_ADDRESS: &str = "0x853d955acef822db058eb8505911ed77f175b99e";

// Wrapped tokens
const WETH_ADDRESS: &str = "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2";
const WBTC_ADDRESS: &str = "0x2260fac5e5542a773aa44fbcfedf7c193bc2c599";

// DeFi tokens
const UNI_ADDRESS: &str = "0x1f9840a85d5af5bf1d1762f925bdaddc4201f984";
const AAVE_ADDRESS: &str = "0x7fc66500c84a76ad7e9c93437bfc5ac33e2ddae9";
const LINK_ADDRESS: &str = "0x514910771af9ca656af840dff83e8264ecf986ca";
const COMP_ADDRESS: &str = "0xc00e94cb662c3520282e6f5717214004a7f26888";
const MKR_ADDRESS: &str = "0x9f8f72aa9304c8b593d555f12ef6589cc3a579a2";
const SNX_ADDRESS: &str = "0xc011a73ee8576fb46f5e1c5751ca3b9fe0af2a6f";
const CRV_ADDRESS: &str = "0xd533a949740bb3306d119cc777fa900ba034cd52";
const SUSHI_ADDRESS: &str = "0x6b3595068778dd592e39a122f4f5a5cf09c90fe2";
const LDO_ADDRESS: &str = "0x5a98fcbea516cf06857215779fd812ca3bef1b32";

// Layer 2 & Scaling
const MATIC_ADDRESS: &str = "0x7d1afa7b718fb893db30a3abc0cfc608aacfebb0";
const ARB_ADDRESS: &str = "0xb50721bcf8d664c30412cfbc6cf7a15145234ad1";
const OP_ADDRESS: &str = "0x4200000000000000000000000000000000000042";

// Meme tokens
const SHIB_ADDRESS: &str = "0x95ad61b0a150d79219dcf64e1e6cc01f0b64c4ce";
const PEPE_ADDRESS: &str = "0x6982508145454ce325ddbe47a25d4ec3d2311933";
const FLOKI_ADDRESS: &str = "0xcf0c122c6b73ff809c693db761e7baebe62b6a2e";

// Exchange & Utility tokens
const APE_ADDRESS: &str = "0x4d224452801aced8b2f0aebe155379bb5d594381";
const GRT_ADDRESS: &str = "0xc944e90c64b2c07662a292be6244bdf05cda44a7";
const FTM_ADDRESS: &str = "0x4e15361fd6b4bb609fa63c81a2be19d873717870";
const SAND_ADDRESS: &str = "0x3845badade8e6dff049820680d1f14bd3903a5d0";
const MANA_ADDRESS: &str = "0x0f5d2fb29fb7d3cfee444a200298f468908cc942";
const AXS_ADDRESS: &str = "0xbb0e17ef65f82ab018d8edd776e8dd940327b28b";
const ENJ_ADDRESS: &str = "0xf629cbd94d3791c9250152bd8dfbdf380e2a3b9c";
const BAT_ADDRESS: &str = "0x0d8775f648430679a709e98d2b0cb6250d2887ef";
const ZRX_ADDRESS: &str = "0xe41d2489571d322189246dafa5ebde1f4699f498";

/// Token registry for mapping symbols to contract addresses
#[derive(Debug, Clone)]
pub struct TokenRegistry {
    registry: HashMap<String, &'static str>,
}

impl TokenRegistry {
    /// Create a new token registry with all supported tokens
    pub fn new() -> Self {
        Self {
            registry: Self::init_registry(),
        }
    }

    /// Initialize the token registry with common tokens
    fn init_registry() -> HashMap<String, &'static str> {
        let mut registry = HashMap::new();

        // Native & Wrapped tokens
        registry.insert("ETH".to_string(), WETH_ADDRESS);
        registry.insert("WETH".to_string(), WETH_ADDRESS);
        registry.insert("WBTC".to_string(), WBTC_ADDRESS);

        // Stablecoins
        registry.insert("USDT".to_string(), USDT_ADDRESS);
        registry.insert("USDC".to_string(), USDC_ADDRESS);
        registry.insert("DAI".to_string(), DAI_ADDRESS);
        registry.insert("BUSD".to_string(), BUSD_ADDRESS);
        registry.insert("FRAX".to_string(), FRAX_ADDRESS);

        // DeFi tokens
        registry.insert("UNI".to_string(), UNI_ADDRESS);
        registry.insert("AAVE".to_string(), AAVE_ADDRESS);
        registry.insert("LINK".to_string(), LINK_ADDRESS);
        registry.insert("COMP".to_string(), COMP_ADDRESS);
        registry.insert("MKR".to_string(), MKR_ADDRESS);
        registry.insert("SNX".to_string(), SNX_ADDRESS);
        registry.insert("CRV".to_string(), CRV_ADDRESS);
        registry.insert("SUSHI".to_string(), SUSHI_ADDRESS);
        registry.insert("LDO".to_string(), LDO_ADDRESS);

        // Layer 2 & Scaling
        registry.insert("MATIC".to_string(), MATIC_ADDRESS);
        registry.insert("ARB".to_string(), ARB_ADDRESS);
        registry.insert("OP".to_string(), OP_ADDRESS);

        // Meme tokens
        registry.insert("SHIB".to_string(), SHIB_ADDRESS);
        registry.insert("PEPE".to_string(), PEPE_ADDRESS);
        registry.insert("FLOKI".to_string(), FLOKI_ADDRESS);

        // Exchange & Utility tokens
        registry.insert("APE".to_string(), APE_ADDRESS);
        registry.insert("GRT".to_string(), GRT_ADDRESS);
        registry.insert("FTM".to_string(), FTM_ADDRESS);
        registry.insert("SAND".to_string(), SAND_ADDRESS);
        registry.insert("MANA".to_string(), MANA_ADDRESS);
        registry.insert("AXS".to_string(), AXS_ADDRESS);
        registry.insert("ENJ".to_string(), ENJ_ADDRESS);
        registry.insert("BAT".to_string(), BAT_ADDRESS);
        registry.insert("ZRX".to_string(), ZRX_ADDRESS);

        registry
    }

    /// Lookup token address by symbol (case-insensitive)
    ///
    /// Returns the contract address if found, None otherwise
    pub fn lookup(&self, symbol: &str) -> Option<&str> {
        let symbol_upper = symbol.to_uppercase();
        self.registry.get(&symbol_upper).copied()
    }

    /// Get list of all supported token symbols (sorted alphabetically)
    pub fn supported_tokens(&self) -> Vec<String> {
        let mut tokens: Vec<String> = self.registry.keys().cloned().collect();
        tokens.sort();
        tokens
    }

    /// Check if a token symbol is supported
    pub fn contains(&self, symbol: &str) -> bool {
        let symbol_upper = symbol.to_uppercase();
        self.registry.contains_key(&symbol_upper)
    }

    /// Get the number of registered tokens
    pub fn len(&self) -> usize {
        self.registry.len()
    }

    /// Check if the registry is empty
    pub fn is_empty(&self) -> bool {
        self.registry.is_empty()
    }

    /// Get WETH address
    pub fn weth_address() -> &'static str {
        WETH_ADDRESS
    }
}

impl Default for TokenRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lookup_existing_token() {
        let registry = TokenRegistry::new();

        assert_eq!(registry.lookup("USDT"), Some(USDT_ADDRESS));
        assert_eq!(registry.lookup("usdt"), Some(USDT_ADDRESS));
        assert_eq!(registry.lookup("ETH"), Some(WETH_ADDRESS));
        assert_eq!(registry.lookup("WETH"), Some(WETH_ADDRESS));
    }

    #[test]
    fn test_lookup_non_existing_token() {
        let registry = TokenRegistry::new();

        assert_eq!(registry.lookup("UNKNOWN"), None);
        assert_eq!(registry.lookup("xyz"), None);
    }

    #[test]
    fn test_contains() {
        let registry = TokenRegistry::new();

        assert!(registry.contains("USDT"));
        assert!(registry.contains("usdt"));
        assert!(registry.contains("ETH"));
        assert!(!registry.contains("UNKNOWN"));
    }

    #[test]
    fn test_supported_tokens() {
        let registry = TokenRegistry::new();
        let tokens = registry.supported_tokens();

        assert!(!tokens.is_empty());
        assert!(tokens.contains(&"USDT".to_string()));
        assert!(tokens.contains(&"ETH".to_string()));

        // Check if sorted
        for i in 1..tokens.len() {
            assert!(tokens[i - 1] <= tokens[i]);
        }
    }

    #[test]
    fn test_len() {
        let registry = TokenRegistry::new();
        assert!(registry.len() > 0);
        assert!(!registry.is_empty());
    }

    #[test]
    fn test_weth_address() {
        assert_eq!(TokenRegistry::weth_address(), WETH_ADDRESS);
    }
}
