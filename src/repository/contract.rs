use alloy::sol;

// Smart contract ABI definitions for Ethereum blockchain interactions
sol! {
    /// ERC20 token standard interface.
    ///
    /// Provides methods to query ERC20 token balances and metadata.
    /// This is a minimal interface containing only the view functions needed for balance queries.
    #[sol(rpc)]
    interface IERC20 {
        /// Returns the token balance of the specified account.
        ///
        /// # Arguments
        /// * `account` - The address to query the balance of
        ///
        /// # Returns
        /// The balance in the token's smallest unit (considering decimals)
        function balanceOf(address account) external view returns (uint256);

        /// Returns the number of decimals used by the token.
        ///
        /// # Returns
        /// The number of decimals (e.g., 18 for most tokens, 6 for USDT/USDC)
        function decimals() external view returns (uint8);

        /// Returns the token symbol.
        ///
        /// # Returns
        /// The token symbol as a string (e.g., "ETH", "USDT", "DAI")
        function symbol() external view returns (string memory);
    }

    /// Uniswap V2 Pair interface for liquidity pool interactions.
    ///
    /// Provides methods to query reserves and token addresses from Uniswap V2 pairs.
    /// Used for calculating token prices based on liquidity pool reserves.
    #[sol(rpc)]
    interface IUniswapV2Pair {
        /// Returns the reserves of both tokens in the pair and the last block timestamp.
        ///
        /// # Returns
        /// * `reserve0` - The reserve amount of token0
        /// * `reserve1` - The reserve amount of token1
        /// * `blockTimestampLast` - The timestamp of the last reserve update
        function getReserves() external view returns (uint112 reserve0, uint112 reserve1, uint32 blockTimestampLast);

        /// Returns the address of the first token in the pair.
        ///
        /// # Returns
        /// The contract address of token0
        function token0() external view returns (address);

        /// Returns the address of the second token in the pair.
        ///
        /// # Returns
        /// The contract address of token1
        function token1() external view returns (address);
    }

    /// Uniswap V2 Factory interface for pair discovery.
    ///
    /// Used to find the pair contract address for any two tokens.
    #[sol(rpc)]
    interface IUniswapV2Factory {
        /// Returns the pair address for two tokens, or zero address if no pair exists.
        ///
        /// # Arguments
        /// * `tokenA` - The address of the first token
        /// * `tokenB` - The address of the second token
        ///
        /// # Returns
        /// The address of the pair contract, or 0x0 if the pair doesn't exist
        function getPair(address tokenA, address tokenB) external view returns (address pair);
    }

    /// Uniswap V2 Router02 interface for token swaps.
    ///
    /// Provides methods to query swap amounts and execute token swaps.
    #[sol(rpc)]
    interface IUniswapV2Router02 {
        /// Given an input amount and token pair, returns the maximum output amount for the swap.
        ///
        /// # Arguments
        /// * `amountIn` - The input amount
        /// * `path` - Array of token addresses representing the swap path
        ///
        /// # Returns
        /// Array of amounts where the last element is the output amount
        function getAmountsOut(uint256 amountIn, address[] calldata path) external view returns (uint256[] memory amounts);

        /// Swaps an exact amount of input tokens for as many output tokens as possible.
        ///
        /// # Arguments
        /// * `amountIn` - The exact amount of input tokens to swap
        /// * `amountOutMin` - The minimum amount of output tokens to receive
        /// * `path` - Array of token addresses representing the swap path
        /// * `to` - Recipient address of the output tokens
        /// * `deadline` - Unix timestamp after which the transaction will revert
        ///
        /// # Returns
        /// Array of amounts swapped at each step
        function swapExactTokensForTokens(
            uint256 amountIn,
            uint256 amountOutMin,
            address[] calldata path,
            address to,
            uint256 deadline
        ) external returns (uint256[] memory amounts);

        /// Returns the factory address.
        function factory() external view returns (address);

        /// Returns the WETH address.
        function WETH() external view returns (address);
    }

    /// Uniswap V3 QuoterV2 interface for getting swap quotes.
    ///
    /// Provides methods to simulate swaps and get exact output amounts without executing the swap.
    #[sol(rpc)]
    interface IQuoterV2 {
        /// QuoteExactInputSingle parameters struct
        struct QuoteExactInputSingleParams {
            address tokenIn;
            address tokenOut;
            uint256 amountIn;
            uint24 fee;
            uint160 sqrtPriceLimitX96;
        }

        /// Returns the amount out for a single-hop exact input swap without executing the swap.
        ///
        /// # Arguments
        /// * `params` - The parameters for the quote
        ///
        /// # Returns
        /// * `amountOut` - The expected output amount
        /// * `sqrtPriceX96After` - The sqrt price after the swap
        /// * `initializedTicksCrossed` - The number of ticks crossed
        /// * `gasEstimate` - The estimated gas usage
        function quoteExactInputSingle(QuoteExactInputSingleParams calldata params)
            external
            returns (
                uint256 amountOut,
                uint160 sqrtPriceX96After,
                uint32 initializedTicksCrossed,
                uint256 gasEstimate
            );

        /// QuoteExactInput parameters for multi-hop swaps
        struct QuoteExactInputParams {
            bytes path;
            uint256 amountIn;
        }

        /// Returns the amount out for a multi-hop exact input swap without executing the swap.
        ///
        /// # Arguments
        /// * `params` - The parameters containing encoded path and input amount
        ///
        /// # Returns
        /// * `amountOut` - The expected output amount
        /// * `sqrtPriceX96AfterList` - Array of sqrt prices after each hop
        /// * `initializedTicksCrossedList` - Array of ticks crossed in each hop
        /// * `gasEstimate` - The estimated gas usage
        function quoteExactInput(QuoteExactInputParams calldata params)
            external
            returns (
                uint256 amountOut,
                uint160[] memory sqrtPriceX96AfterList,
                uint32[] memory initializedTicksCrossedList,
                uint256 gasEstimate
            );
    }

    /// Uniswap V3 SwapRouter interface for executing swaps.
    ///
    /// Provides methods to execute token swaps on Uniswap V3 with single or multi-hop routes.
    #[sol(rpc)]
    interface ISwapRouter {
        /// ExactInputSingle parameters struct
        struct ExactInputSingleParams {
            address tokenIn;
            address tokenOut;
            uint24 fee;
            address recipient;
            uint256 deadline;
            uint256 amountIn;
            uint256 amountOutMinimum;
            uint160 sqrtPriceLimitX96;
        }

        /// Swaps `amountIn` of one token for as much as possible of another token.
        ///
        /// # Arguments
        /// * `params` - The parameters necessary for the swap
        ///
        /// # Returns
        /// The amount of the received token
        function exactInputSingle(ExactInputSingleParams calldata params)
            external
            payable
            returns (uint256 amountOut);

        /// ExactInput parameters for multi-hop swaps
        struct ExactInputParams {
            bytes path;
            address recipient;
            uint256 deadline;
            uint256 amountIn;
            uint256 amountOutMinimum;
        }

        /// Swaps `amountIn` of one token for as much as possible of another along the specified path.
        ///
        /// # Arguments
        /// * `params` - The parameters necessary for the multi-hop swap
        ///
        /// # Returns
        /// The amount of the received token
        function exactInput(ExactInputParams calldata params)
            external
            payable
            returns (uint256 amountOut);
    }
}
