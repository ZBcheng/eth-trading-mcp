//! Utilities for precise decimal arithmetic with U256 values
//!
//! This module provides conversion between U256 (blockchain integers) and Decimal
//! for accurate financial calculations without floating-point precision loss.

use alloy::primitives::U256;
use rust_decimal::Decimal;
use std::str::FromStr;

use super::ServiceResult;
use super::error::ServiceError;

/// Convert U256 to Decimal with proper decimal scaling
///
/// # Arguments
/// * `value` - The U256 value to convert
/// * `decimals` - Number of decimal places (e.g., 18 for ETH, 6 for USDC)
///
/// # Returns
/// A Decimal representing the actual value (e.g., 1.5 ETH instead of 1500000000000000000 wei)
pub fn u256_to_decimal(value: U256, decimals: u8) -> ServiceResult<Decimal> {
    // Convert U256 to string
    let value_str = value.to_string();

    // Parse to Decimal
    let mut decimal = Decimal::from_str(&value_str).map_err(|e| {
        ServiceError::InvalidAmount(format!("Failed to parse U256 to Decimal: {}", e))
    })?;

    // Adjust for decimals by dividing by 10^decimals
    if decimals > 0 {
        let divisor = Decimal::from(10u64.pow(decimals as u32));
        decimal /= divisor;
    }

    // Normalize to remove trailing zeros
    Ok(decimal.normalize())
}

/// Convert Decimal to U256 with proper decimal scaling
///
/// # Arguments
/// * `value` - The Decimal value to convert
/// * `decimals` - Number of decimal places to scale to
///
/// # Returns
/// A U256 representing the raw blockchain value (e.g., wei instead of ETH)
pub fn decimal_to_u256(value: Decimal, decimals: u8) -> ServiceResult<U256> {
    // Scale up by multiplying by 10^decimals
    let scaled = if decimals > 0 {
        let multiplier = Decimal::from(10u64.pow(decimals as u32));
        value * multiplier
    } else {
        value
    };

    // Convert to string and remove decimal point
    let scaled_str = scaled.to_string();
    let integer_str = scaled_str.split('.').next().unwrap_or(&scaled_str);

    // Parse to U256
    U256::from_str(integer_str)
        .map_err(|e| ServiceError::InvalidAmount(format!("Failed to parse Decimal to U256: {}", e)))
}

/// Calculate price with precise decimal arithmetic
///
/// # Arguments
/// * `numerator` - Reserve of output token
/// * `denominator` - Reserve of input token
/// * `numerator_decimals` - Decimals for numerator token
/// * `denominator_decimals` - Decimals for denominator token
///
/// # Returns
/// Price as a Decimal
pub fn calculate_price(
    numerator: U256,
    denominator: U256,
    numerator_decimals: u8,
    denominator_decimals: u8,
) -> ServiceResult<Decimal> {
    if denominator.is_zero() {
        return Err(ServiceError::InvalidAmount("Division by zero".to_string()));
    }

    let num_decimal = u256_to_decimal(numerator, numerator_decimals)?;
    let den_decimal = u256_to_decimal(denominator, denominator_decimals)?;

    if den_decimal.is_zero() {
        return Err(ServiceError::InvalidAmount("Division by zero".to_string()));
    }

    Ok(num_decimal / den_decimal)
}

/// Calculate percentage with precise decimal arithmetic
///
/// # Arguments
/// * `value` - The value to calculate percentage of
/// * `percentage` - The percentage (e.g., 0.5 for 0.5%)
///
/// # Returns
/// Result value
pub fn apply_percentage(value: U256, percentage: Decimal) -> ServiceResult<U256> {
    let value_decimal = Decimal::from_str(&value.to_string())
        .map_err(|e| ServiceError::InvalidAmount(format!("Failed to parse value: {}", e)))?;

    let result_decimal = value_decimal * percentage / Decimal::from(100);

    let result_str = result_decimal.to_string();
    let integer_str = result_str.split('.').next().unwrap_or(&result_str);

    U256::from_str(integer_str)
        .map_err(|e| ServiceError::InvalidAmount(format!("Failed to parse result: {}", e)))
}

/// Parse human-readable amount (e.g., "1.5") to smallest unit based on decimals
///
/// # Arguments
/// * `amount` - Human-readable amount as string (e.g., "1.5" for 1.5 ETH)
/// * `decimals` - Number of decimal places for the token
///
/// # Examples
/// - "1" with 18 decimals -> 1000000000000000000 (1 ETH in wei)
/// - "100" with 6 decimals -> 100000000 (100 USDC in smallest unit)
///
/// # Returns
/// U256 value in smallest unit
pub fn parse_amount(amount: &str, decimals: u8) -> Result<U256, String> {
    // Try to parse as Decimal first for human-readable amounts
    if let Ok(decimal_amount) = Decimal::from_str(amount) {
        // Multiply by 10^decimals to get the smallest unit
        // Build multiplier: 10^decimals
        let mut multiplier = Decimal::from(1);
        for _ in 0..decimals {
            multiplier *= Decimal::from(10);
        }

        let smallest_unit = decimal_amount * multiplier;

        // Convert to string and parse as U256 (remove decimal point if any)
        let amount_str = smallest_unit.to_string();
        let integer_part = amount_str.split('.').next().unwrap_or("0");

        U256::from_str(integer_part).map_err(|e| format!("Failed to parse amount: {}", e))
    } else {
        // If not a decimal, try parsing directly as U256 (assume already in smallest unit)
        U256::from_str(amount).map_err(|e| format!("Invalid amount format: {}", e))
    }
}

/// Format balance from smallest unit to human-readable format
///
/// # Arguments
/// * `balance` - Balance in smallest unit (e.g., wei for ETH)
/// * `decimals` - Number of decimal places for the token
///
/// # Returns
/// Formatted balance as string with trailing zeros removed
pub fn format_balance(balance: U256, decimals: u8) -> String {
    let divisor = U256::from(10u64).pow(U256::from(decimals));
    let whole = balance / divisor;
    let remainder = balance % divisor;

    if remainder.is_zero() {
        whole.to_string()
    } else {
        let decimal_part = remainder.to_string();
        let padded = format!("{:0>width$}", decimal_part, width = decimals as usize);
        let trimmed = padded.trim_end_matches('0');
        if trimmed.is_empty() {
            whole.to_string()
        } else {
            format!("{whole}.{trimmed}")
        }
    }
}

/// Calculate price impact percentage for a swap
///
/// # Arguments
/// * `amount_in` - Input amount
/// * `amount_out` - Output amount
/// * `reserve_in` - Input token reserve in the pool
/// * `reserve_out` - Output token reserve in the pool
///
/// # Returns
/// Price impact as a percentage string
pub fn calculate_price_impact(
    amount_in: U256,
    amount_out: U256,
    reserve_in: U256,
    reserve_out: U256,
) -> String {
    if reserve_in.is_zero() || reserve_out.is_zero() || amount_in.is_zero() {
        return "0".to_string();
    }

    // Price before = reserve_out / reserve_in
    // Price after = (reserve_out - amount_out) / (reserve_in + amount_in)
    // Impact = |1 - (price_after / price_before)| * 100

    // Use Decimal for precise calculation
    let price_before = match calculate_price(reserve_out, reserve_in, 18, 18) {
        Ok(p) => p,
        Err(_) => return "0".to_string(),
    };

    let new_reserve_out = reserve_out.saturating_sub(amount_out);
    let new_reserve_in = reserve_in + amount_in;

    let price_after = match calculate_price(new_reserve_out, new_reserve_in, 18, 18) {
        Ok(p) => p,
        Err(_) => return "0".to_string(),
    };

    let impact = (Decimal::from(1) - (price_after / price_before)).abs() * Decimal::from(100);
    impact.to_string()
}

/// Calculate exchange rate between tokens with different decimals
///
/// # Arguments
/// * `amount_in` - Input amount
/// * `amount_out` - Output amount
/// * `decimals_in` - Decimals for input token
/// * `decimals_out` - Decimals for output token
///
/// # Returns
/// Exchange rate as a string
pub fn calculate_exchange_rate(
    amount_in: U256,
    amount_out: U256,
    decimals_in: u8,
    decimals_out: u8,
) -> String {
    if amount_in > U256::ZERO {
        match calculate_price(amount_out, amount_in, decimals_out, decimals_in) {
            Ok(rate) => rate.to_string(),
            Err(_) => "0".to_string(),
        }
    } else {
        "0".to_string()
    }
}

/// Calculate minimum output amount with slippage tolerance using precise decimal arithmetic
///
/// # Arguments
/// * `amount_out` - Expected output amount
/// * `slippage` - Slippage tolerance as a percentage (e.g., 0.5 for 0.5%)
///
/// # Returns
/// Minimum acceptable output amount
pub fn calculate_minimum_output(amount_out: U256, slippage: Decimal) -> U256 {
    // Calculate (100 - slippage) as a percentage
    let percentage = Decimal::from(100) - slippage;

    // Convert amount to Decimal
    let amount_decimal = match Decimal::from_str(&amount_out.to_string()) {
        Ok(d) => d,
        Err(_) => return U256::ZERO,
    };

    // Calculate minimum: amount * (100 - slippage) / 100
    let minimum = amount_decimal * percentage / Decimal::from(100);

    // Convert back to U256
    match U256::from_str(&minimum.to_string().split('.').next().unwrap_or("0")) {
        Ok(result) => result,
        Err(_) => U256::ZERO,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_u256_to_decimal_eth_should_work() {
        // 1.5 ETH = 1500000000000000000 wei
        let wei = U256::from_str("1500000000000000000").unwrap();
        let eth = u256_to_decimal(wei, 18).unwrap();
        assert_eq!(eth.to_string(), "1.5"); // Decimal removes trailing zeros
    }

    #[test]
    fn test_u256_to_decimal_usdc_should_work() {
        // 1000.5 USDC = 1000500000 (6 decimals)
        let raw = U256::from(1000500000u64);
        let usdc = u256_to_decimal(raw, 6).unwrap();
        assert_eq!(usdc.to_string(), "1000.5"); // Decimal removes trailing zeros
    }

    #[test]
    fn test_decimal_to_u256_eth_should_work() {
        let eth = Decimal::from_str("1.5").unwrap();
        let wei = decimal_to_u256(eth, 18).unwrap();
        assert_eq!(wei, U256::from_str("1500000000000000000").unwrap());
    }

    #[test]
    fn test_calculate_price_should_work() {
        // Price: 2000 USDC / 1 WETH = 2000 USD per ETH
        let usdc_reserve = U256::from(2000000000u64); // 2000 USDC (6 decimals)
        let weth_reserve = U256::from_str("1000000000000000000").unwrap(); // 1 WETH (18 decimals)

        let price = calculate_price(usdc_reserve, weth_reserve, 6, 18).unwrap();
        assert_eq!(price.to_string(), "2000");
    }

    #[test]
    fn test_apply_percentage_should_work() {
        let value = U256::from(1000u64);
        let percentage = Decimal::from_str("0.5").unwrap(); // 0.5%
        let result = apply_percentage(value, percentage).unwrap();
        assert_eq!(result, U256::from(5u64)); // 1000 * 0.5% = 5
    }

    #[test]
    fn test_parse_amount_et_should_work() {
        // Parse 1.5 ETH
        let amount = parse_amount("1.5", 18).unwrap();
        assert_eq!(amount, U256::from_str("1500000000000000000").unwrap());
    }

    #[test]
    fn test_parse_amount_usdc_should_work() {
        // Parse 100.5 USDC (6 decimals)
        let amount = parse_amount("100.5", 6).unwrap();
        assert_eq!(amount, U256::from(100500000u64));
    }

    #[test]
    fn test_format_balance_eth_should_work() {
        let wei = U256::from_str("1500000000000000000").unwrap();
        let formatted = format_balance(wei, 18);
        assert_eq!(formatted, "1.5");
    }

    #[test]
    fn test_format_balance_usdc_should_work() {
        let amount = U256::from(100500000u64);
        let formatted = format_balance(amount, 6);
        assert_eq!(formatted, "100.5");
    }

    #[test]
    fn test_format_balance_whole_number_should_work() {
        let wei = U256::from_str("1000000000000000000").unwrap();
        let formatted = format_balance(wei, 18);
        assert_eq!(formatted, "1");
    }

    #[test]
    fn test_calculate_price_impact_zero_input_should_work() {
        let result = calculate_price_impact(
            U256::ZERO,
            U256::from(1000u64),
            U256::from(10000u64),
            U256::from(10000u64),
        );
        assert_eq!(result, "0");
    }

    #[test]
    fn test_calculate_price_impact_normal() {
        // Test a small trade with minimal impact
        let amount_in = U256::from_str("1000000000000000000").unwrap(); // 1 ETH
        let amount_out = U256::from_str("2000000000").unwrap(); // ~2000 USDC
        let reserve_in = U256::from_str("1000000000000000000000").unwrap(); // 1000 ETH
        let reserve_out = U256::from_str("2000000000000").unwrap(); // 2M USDC

        let impact = calculate_price_impact(amount_in, amount_out, reserve_in, reserve_out);
        // Should be a very small impact for 1 ETH in a 1000 ETH pool
        assert_ne!(impact, "0");
    }

    #[test]
    fn test_calculate_exchange_rate_should_work() {
        // 1 ETH = 2000 USDC
        let amount_in = U256::from_str("1000000000000000000").unwrap(); // 1 ETH (18 decimals)
        let amount_out = U256::from(2000000000u64); // 2000 USDC (6 decimals)

        let rate = super::calculate_exchange_rate(amount_in, amount_out, 18, 6);
        assert_eq!(rate, "2000");
    }

    #[test]
    fn test_calculate_minimum_output_should_work() {
        // 1000 tokens with 0.5% slippage = 995 minimum
        let amount_out = U256::from(1000u64);
        let slippage = Decimal::from_str("0.5").unwrap();

        let minimum = super::calculate_minimum_output(amount_out, slippage);
        assert_eq!(minimum, U256::from(995u64));
    }
}
