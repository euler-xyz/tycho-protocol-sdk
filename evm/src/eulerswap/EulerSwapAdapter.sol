// SPDX-License-Identifier: AGPL-3.0-or-later
pragma solidity ^0.8.27;

import {
    ISwapAdapter, ISwapAdapterTypes
} from "src/interfaces/ISwapAdapter.sol";
import {IEulerSwap} from "src/eulerswap/IEulerSwap.sol";
import {IEulerSwapFactory} from "src/eulerswap/IEulerSwapFactory.sol";
import {IEulerSwapPeriphery} from "src/eulerswap/IEulerSwapPeriphery.sol";
import {
    IERC20,
    SafeERC20
} from "openzeppelin-contracts/contracts/token/ERC20/utils/SafeERC20.sol";
import {IERC4626} from "openzeppelin-contracts/contracts/interfaces/IERC4626.sol";


contract EulerSwapAdapter is ISwapAdapter {
    using SafeERC20 for IERC20;

    IEulerSwapFactory immutable factory;
    IEulerSwapPeriphery immutable periphery;

    uint32 internal constant POOL_STATUS_UNLOCKED = 1;

    struct Limit {
        uint256 limitIn;
        uint256 limitOut;
    }

    struct PoolCache {
        address token0;
        uint112 reserve0;
        uint112 reserve1;
        Limit limit0to1;
        Limit limit1to0;
        bool initialized;
    }

    mapping(address eulerSwap => PoolCache) internal pools;

    constructor(address factory_, address periphery_) {
        factory = IEulerSwapFactory(factory_);
        periphery = IEulerSwapPeriphery(periphery_);
    }

    /// @inheritdoc ISwapAdapter
    function swap(
        bytes32 poolId,
        address sellToken,
        address buyToken,
        OrderSide side,
        uint256 specifiedAmount
    ) external returns (Trade memory trade) {
        IEulerSwap pool = IEulerSwap(address(bytes20(poolId)));

        uint256 amountIn;
        uint256 amountOut;
        if (side == OrderSide.Buy) {
            amountIn = (
                quoteExactOutput(pool, sellToken, buyToken, specifiedAmount)
                    .denominator
            );
            trade.calculatedAmount = amountOut = specifiedAmount;
        } else {
            trade.calculatedAmount = amountIn = specifiedAmount;
            amountOut = (
                quoteExactInput(pool, sellToken, buyToken, specifiedAmount)
                    .numerator
            );
        }

        trade.gasUsed = 300000; //TODO set correct
        trade.price = Fraction(0, 0);
    }

    /// @inheritdoc ISwapAdapter
    function price(
        bytes32, /*poolId*/
        address, /*sellToken*/
        address, /*buyToken*/
        uint256[] memory /*specifiedAmounts*/
    ) external pure override returns (Fraction[] memory) {
        revert ISwapAdapterTypes.NotImplemented(
            "Price function not implemented"
        );

        // prices = new Fraction[](specifiedAmounts.length);

        // IEulerSwap pool = IEulerSwap(address(bytes20(poolId)));
        // for (uint256 i = 0; i < specifiedAmounts.length; i++) {
        //     prices[i] =
        //         quoteExactInput(pool, sellToken, buyToken,
        // specifiedAmounts[i]);
        // }
    }

    /// @inheritdoc ISwapAdapter
    function getLimits(bytes32 poolId, address sellToken, address buyToken)
        external
        view
        override
        returns (uint256[] memory limits)
    {
        address pool = address(bytes20(poolId));
        PoolCache storage cache = pools[pool];

        limits = new uint256[](2);

        if (cache.initialized) {
            if (cache.token0 == sellToken) {
                (limits[0], limits[1]) = (cache.limit0to1.limitIn, cache.limit0to1.limitOut);
            } else {
                (limits[0], limits[1]) = (cache.limit1to0.limitIn, cache.limit1to0.limitOut);
            }
        } else {
            (limits[0], limits[1]) = periphery.getLimits(pool, sellToken, buyToken);
        }
    }

    /// @inheritdoc ISwapAdapter
    function getTokens(bytes32 poolId)
        external
        view
        override
        returns (address[] memory tokens)
    {
        tokens = new address[](2);
        IEulerSwap pool = IEulerSwap(address(bytes20(poolId)));
        (tokens[0], tokens[1]) = pool.getAssets();
    }

    /// @inheritdoc ISwapAdapter
    function getPoolIds(uint256 offset, uint256 limit)
        external
        view
        override
        returns (bytes32[] memory ids)
    {
        address[] memory allPools = factory.pools();
        uint256 endIdx = offset + limit;
        if (endIdx > allPools.length) {
            endIdx = allPools.length;
        }
        ids = new bytes32[](endIdx - offset);
        for (uint256 i = 0; i < ids.length; i++) {
            ids[i] = bytes20(allPools[offset + i]);
        }
    }

    /// @inheritdoc ISwapAdapter
    function getCapabilities(bytes32, address, address)
        external
        pure
        override
        returns (Capability[] memory capabilities)
    {
        capabilities = new Capability[](3);
        capabilities[0] = Capability.SellOrder;
        capabilities[1] = Capability.BuyOrder;
        capabilities[2] = Capability.TokenBalanceIndependent;
    }

    function quoteExactInput(
        IEulerSwap pool,
        address tokenIn,
        address tokenOut,
        uint256 amountIn
    ) internal returns (Fraction memory calculatedPrice) {
        PoolCache storage cache = loadPoolCache(address(pool));

        uint256 amountOut =
        periphery.quoteExactInputWithReserves(
            address(pool),
            tokenIn,
            tokenOut,
            amountIn,
            cache.reserve0,
            cache.reserve1
        );

        updatePoolCache(cache, amountIn, amountOut, tokenIn);

        calculatedPrice = Fraction(amountOut, amountIn);
    }

    /// @dev for testing only
    function getPoolCache(address pool) public view returns (PoolCache memory) {
        return pools[pool];
    }

    function quoteExactOutput(
        IEulerSwap pool,
        address tokenIn,
        address tokenOut,
        uint256 amountOut
    ) internal returns (Fraction memory calculatedPrice) {
        PoolCache storage cache = loadPoolCache(address(pool));

        uint256 amountIn = periphery
            .quoteExactOutputWithReserves(
            address(pool),
            tokenIn,
            tokenOut,
            amountOut,
            cache.reserve0,
            cache.reserve1
        );

        updatePoolCache(cache, amountIn, amountOut, tokenIn);

        calculatedPrice = Fraction(amountOut, amountIn);
    }

    function loadPoolCache(address pool) internal returns (PoolCache storage) {
        PoolCache storage cache = pools[pool];

        if (!cache.initialized) {
            initializeCache(pool);
        }

        return cache;
    }

    /// @dev Function is public for testing
    function initializeCache(address pool) public {
        PoolCache storage cache = pools[pool];

        (uint112 reserve0, uint112 reserve1, uint32 status) =
            IEulerSwap(pool).getReserves();
        if (status != POOL_STATUS_UNLOCKED) revert("Invalid pool state");

        cache.reserve0 = reserve0;
        cache.reserve1 = reserve1;

        IEulerSwap.Params memory p = IEulerSwap(pool).getParams();

        address token0 = IERC4626(p.vault0).asset();
        address token1 = IERC4626(p.vault1).asset();

        cache.token0 = token0;

        (uint256 limitIn, uint256 limitOut) =
            periphery.getLimits(pool, token0, token1);
        cache.limit0to1 = Limit(limitIn, limitOut);
        (limitIn, limitOut) = periphery.getLimits(pool, token1, token0);
        cache.limit1to0 = Limit(limitIn, limitOut);

        cache.initialized = true;
    }

    function updatePoolCache(
        PoolCache storage cache,
        uint256 amountIn,
        uint256 amountOut,
        address tokenIn
    ) internal {
        uint256 amount0In;
        uint256 amount0Out;
        uint256 amount1In;
        uint256 amount1Out;

        if (cache.token0 == tokenIn) {
            amount0In = amountIn;
            amount1Out = amountOut;
        } else {
            amount0Out = amountOut;
            amount1In = amountIn;
        }

        uint256 newReserve0 = cache.reserve0 + amount0In - amount0Out;
        uint256 newReserve1 = cache.reserve1 + amount1In - amount1Out;

        cache.reserve0 = uint112(newReserve0);
        cache.reserve1 = uint112(newReserve1);

        if (cache.token0 == tokenIn) {
            require(cache.limit0to1.limitIn > amountIn, LimitExceeded(cache.limit0to1.limitIn));
            require(cache.limit0to1.limitOut > amountOut, LimitExceeded(cache.limit0to1.limitOut));
            cache.limit0to1.limitIn -= amountIn;
            cache.limit0to1.limitOut -= amountOut;
            cache.limit1to0.limitIn += amountOut;
            cache.limit1to0.limitOut += amountIn;
        } else {
            require(cache.limit1to0.limitIn > amountIn, LimitExceeded(cache.limit1to0.limitIn));
            require(cache.limit1to0.limitOut > amountOut, LimitExceeded(cache.limit1to0.limitOut));
            cache.limit1to0.limitIn -= amountIn;
            cache.limit1to0.limitOut -= amountOut;
            cache.limit0to1.limitIn += amountOut;
            cache.limit0to1.limitOut += amountIn;
        }
    }
}
