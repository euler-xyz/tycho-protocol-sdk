// SPDX-License-Identifier: AGPL-3.0-or-later
pragma solidity ^0.8.13;

import {ISwapAdapter} from "src/interfaces/ISwapAdapter.sol";
import {
    IERC20,
    SafeERC20
} from "openzeppelin-contracts/contracts/token/ERC20/utils/SafeERC20.sol";

contract EulerSwapAdapter is ISwapAdapter {
    using SafeERC20 for IERC20;

    IMaglevEulerSwapFactory immutable factory;

    constructor(address factory_) {
        factory = IMaglevEulerSwapFactory(factory_);
    }

    /// @inheritdoc ISwapAdapter
    function price(
        bytes32 poolId,
        address sellToken,
        address buyToken,
        uint256[] memory specifiedAmounts
    ) external view override returns (Fraction[] memory prices) {
        prices = new Fraction[](specifiedAmounts.length);
        
        IMaglevEulerSwap pool = IMaglevEulerSwap(address(bytes20(poolId)));
        for (uint256 i = 0; i < specifiedAmounts.length; i++) {
            prices[i] = quoteExactInput(pool, sellToken, buyToken, specifiedAmounts[i]);
        }
    }

    /// @inheritdoc ISwapAdapter
    function swap(
        bytes32 poolId,
        address sellToken,
        address buyToken,
        OrderSide side,
        uint256 specifiedAmount
    ) external returns (Trade memory trade) {
        IMaglevEulerSwap pool = IMaglevEulerSwap(address(bytes20(poolId)));

        bool isAmountOutAsset0 = buyToken == pool.asset0();
        uint256 amountIn;
        uint256 amountOut;
        if (side == OrderSide.Buy) {
            amountIn = (quoteExactOutput(pool, sellToken, buyToken, specifiedAmount).denominator);
            trade.calculatedAmount = amountOut = specifiedAmount;
        } else {
            trade.calculatedAmount = amountIn = specifiedAmount;
            amountOut =
                (quoteExactInput(pool, sellToken, buyToken, specifiedAmount).numerator);
        }

        IERC20(sellToken).safeTransferFrom(
            msg.sender, address(pool), amountIn
        );

        uint256 gasBefore = gasleft();
        (isAmountOutAsset0) ? pool.swap(amountOut, 0, msg.sender, "") : pool.swap(0, amountOut, msg.sender, "");
        trade.gasUsed = gasBefore - gasleft();

        if (side == OrderSide.Buy) {
            trade.price = quoteExactOutput(pool, sellToken, buyToken, specifiedAmount);
        } else {
            trade.price =
                quoteExactInput(pool, sellToken, buyToken, specifiedAmount);
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

    /// @inheritdoc ISwapAdapter
    function getTokens(bytes32 poolId)
        external
        view
        override
        returns (address[] memory tokens)
    {
        tokens = new address[](2);
        IMaglevEulerSwap pool = IMaglevEulerSwap(address(bytes20(poolId)));
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

    /// @notice Calculates pool prices for specified amounts
    function quoteExactInput(
        IMaglevEulerSwap pool,
        address tokenIn,
        address tokenOut,
        uint256 amountIn
    ) internal view returns (Fraction memory calculatedPrice) {
        calculatedPrice =
            Fraction(pool.quoteExactInput(tokenIn, tokenOut, amountIn), amountIn);
    }

    function quoteExactOutput(
        IMaglevEulerSwap pool,
        address tokenIn,
        address tokenOut,
        uint256 amountOut
    ) internal view returns (Fraction memory calculatedPrice) {
        calculatedPrice =
            Fraction(amountOut, pool.quoteExactOutput(tokenIn, tokenOut, amountOut));
    }
}

interface IMaglevEulerSwapFactory {
    event PoolDeployed(address indexed asset0, address indexed asset1, uint256 indexed feeMultiplier, address pool);
    
    function deployPool(
        address vault0,
        address vault1,
        address holder,
        uint112 debtLimit0,
        uint112 debtLimit1,
        uint256 fee,
        uint256 priceX,
        uint256 priceY,
        uint256 concentrationX,
        uint256 concentrationY
    ) external returns (address);

    function evc() external view returns (address);
    function allPools(uint256 index) external view returns (address);
    function getPool(address assetA, address assetB, uint256 fee) external view returns (address);
    function allPoolsLength() external view returns (uint256);
    function getAllPoolsListSlice(uint256 start, uint256 end) external view returns (address[] memory);
}

interface IMaglevEulerSwap {
    // IMaglevBase
    function configure() external;
    function swap(uint256 amount0Out, uint256 amount1Out, address to, bytes calldata data) external;
    function quoteExactInput(address tokenIn, address tokenOut, uint256 amountIn) external view returns (uint256);
    function quoteExactOutput(address tokenIn, address tokenOut, uint256 amountOut) external view returns (uint256);

    function vault0() external view returns (address);
    function vault1() external view returns (address);
    function asset0() external view returns (address);
    function asset1() external view returns (address);
    function myAccount() external view returns (address);
    function debtLimit0() external view returns (uint112);
    function debtLimit1() external view returns (uint112);
    function feeMultiplier() external view returns (uint256);
    function getReserves() external view returns (uint112, uint112, uint32);

    // IMaglevEulerSwap
    function priceX() external view returns (uint256);
    function priceY() external view returns (uint256);
    function concentrationX() external view returns (uint256);
    function concentrationY() external view returns (uint256);
    function initialReserve0() external view returns (uint112);
    function initialReserve1() external view returns (uint112);
}
