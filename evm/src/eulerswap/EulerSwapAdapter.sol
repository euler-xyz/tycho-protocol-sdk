// SPDX-License-Identifier: AGPL-3.0-or-later
pragma solidity ^0.8.13;

import {ISwapAdapter} from "src/interfaces/ISwapAdapter.sol";
import {IEulerSwap} from "src/eulerswap/IEulerSwap.sol";
import {IEulerSwapFactory} from "src/eulerswap/IEulerSwapFactory.sol";
import {IEulerSwapPeriphery} from "src/eulerswap/IEulerSwapPeriphery.sol";
import {
    IERC20,
    SafeERC20
} from "openzeppelin-contracts/contracts/token/ERC20/utils/SafeERC20.sol";

contract EulerSwapAdapter is ISwapAdapter {
    using SafeERC20 for IERC20;

    IEulerSwapFactory immutable factory;
    IEulerSwapPeriphery immutable periphery;

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

        bool isAmountOutAsset0 = buyToken == pool.asset0();
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

        IERC20(sellToken).safeTransferFrom(msg.sender, address(pool), amountIn);

        uint256 gasBefore = gasleft();
        (isAmountOutAsset0)
            ? pool.swap(amountOut, 0, msg.sender, "")
            : pool.swap(0, amountOut, msg.sender, "");
        trade.gasUsed = gasBefore - gasleft();
    }

    /// @inheritdoc ISwapAdapter
    function price(
        bytes32 poolId,
        address sellToken,
        address buyToken,
        uint256[] memory specifiedAmounts
    ) external view override returns (Fraction[] memory prices) {
        prices = new Fraction[](specifiedAmounts.length);

        IEulerSwap pool = IEulerSwap(address(bytes20(poolId)));
        for (uint256 i = 0; i < specifiedAmounts.length; i++) {
            prices[i] =
                quoteExactInput(pool, sellToken, buyToken, specifiedAmounts[i]);
        }
    }

    /// @inheritdoc ISwapAdapter
    function getLimits(bytes32 poolId, address sellToken, address buyToken)
        external
        view
        override
        returns (uint256[] memory limits)
    {
        limits = new uint256[](2);
        address pool = address(bytes20(poolId));

        (limits[0], limits[1]) = periphery.getLimits(pool, sellToken, buyToken);
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
        tokens[0] = address(pool.asset0());
        tokens[1] = address(pool.asset1());
    }

    /// @inheritdoc ISwapAdapter
    function getPoolIds(uint256 offset, uint256 limit)
        external
        view
        override
        returns (bytes32[] memory ids)
    {
        uint256 endIdx = offset + limit;
        if (endIdx > factory.allPoolsLength()) {
            endIdx = factory.allPoolsLength();
        }
        ids = new bytes32[](endIdx - offset);
        for (uint256 i = 0; i < ids.length; i++) {
            ids[i] = bytes20(factory.allPools(offset + i));
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
        capabilities[2] = Capability.PriceFunction;
    }

    /// @notice Calculates pool prices for specified amounts
    function quoteExactInput(
        IEulerSwap pool,
        address tokenIn,
        address tokenOut,
        uint256 amountIn
    ) internal view returns (Fraction memory calculatedPrice) {
        calculatedPrice = Fraction(
            periphery.quoteExactInput(
                address(pool), tokenIn, tokenOut, amountIn
            ),
            amountIn
        );
    }

    function quoteExactOutput(
        IEulerSwap pool,
        address tokenIn,
        address tokenOut,
        uint256 amountOut
    ) internal view returns (Fraction memory calculatedPrice) {
        calculatedPrice = Fraction(
            amountOut,
            periphery.quoteExactOutput(
                address(pool), tokenIn, tokenOut, amountOut
            )
        );
    }
}
