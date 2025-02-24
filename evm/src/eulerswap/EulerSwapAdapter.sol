// SPDX-License-Identifier: AGPL-3.0-or-later
pragma solidity ^0.8.13;

import {ISwapAdapter} from "src/interfaces/ISwapAdapter.sol";
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
    function getLimits(bytes32 poolId, address sellToken, address buyToken)
        external
        view
        override
        returns (uint256[] memory limits)
    {
        limits = new uint256[](2);
        IEulerSwap pool = IEulerSwap(address(bytes20(poolId)));
        address swapAccount = pool.myAccount();

        IEVC evc = IEVC(IEVault(pool.vault0()).EVC());
        if (!evc.isAccountOperatorAuthorized(swapAccount, address(pool))) {
            return limits;
        }

        (uint256 r0, uint256 r1,) = pool.getReserves();
        if (sellToken < buyToken) {
            limits[0] = r0;
            limits[1] = r1;
        } else {
            limits[0] = r1;
            limits[1] = r0;
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

    function vaultBalance(address vault, address swapAccount)
        internal
        view
        returns (uint256)
    {
        uint256 shares = IEVault(vault).balanceOf(swapAccount);

        return shares == 0 ? 0 : IEVault(vault).convertToAssets(shares);
    }
}

interface IEulerSwapFactory {
    struct DeployParams {
        address vault0;
        address vault1;
        address swapAccount;
        uint256 fee;
        uint256 priceX;
        uint256 priceY;
        uint256 concentrationX;
        uint256 concentrationY;
        uint112 debtLimit0;
        uint112 debtLimit1;
    }

    function deployPool(DeployParams memory params)
        external
        returns (address);

    function allPools(uint256 index) external view returns (address);
    function getPool(bytes32 poolKey) external view returns (address);
    function allPoolsLength() external view returns (uint256);
    function getAllPoolsListSlice(uint256 start, uint256 end)
        external
        view
        returns (address[] memory);
}

interface IEulerSwap {
    struct Params {
        address vault0;
        address vault1;
        address myAccount;
        uint112 debtLimit0;
        uint112 debtLimit1;
        uint256 fee;
    }

    struct CurveParams {
        uint256 priceX;
        uint256 priceY;
        uint256 concentrationX;
        uint256 concentrationY;
    }

    function swap(
        uint256 amount0Out,
        uint256 amount1Out,
        address to,
        bytes calldata data
    ) external;
    function activate() external;

    function verify(uint256 newReserve0, uint256 newReserve1)
        external
        view
        returns (bool);
    function curve() external view returns (bytes32);
    function vault0() external view returns (address);
    function vault1() external view returns (address);
    function asset0() external view returns (address);
    function asset1() external view returns (address);
    function myAccount() external view returns (address);
    function initialReserve0() external view returns (uint112);
    function initialReserve1() external view returns (uint112);
    function feeMultiplier() external view returns (uint256);
    function getReserves()
        external
        view
        returns (uint112 reserve0, uint112 reserve1, uint32 status);
    function priceX() external view returns (uint256);
    function priceY() external view returns (uint256);
    function concentrationX() external view returns (uint256);
    function concentrationY() external view returns (uint256);
}

interface IEulerSwapPeriphery {
    /// @notice How much `tokenOut` can I get for `amountIn` of `tokenIn`?
    function quoteExactInput(
        address eulerSwap,
        address tokenIn,
        address tokenOut,
        uint256 amountIn
    ) external view returns (uint256);

    /// @notice How much `tokenIn` do I need to get `amountOut` of `tokenOut`?
    function quoteExactOutput(
        address eulerSwap,
        address tokenIn,
        address tokenOut,
        uint256 amountOut
    ) external view returns (uint256);
}

interface IEVault {
    function EVC() external view returns (address);
    function balanceOf(address account) external view returns (uint256);
    function convertToAssets(uint256 shares) external view returns (uint256);
    function maxWithdraw(address owner) external view returns (uint256);
}

interface IEVC {
    function isAccountOperatorAuthorized(address account, address operator)
        external
        view
        returns (bool authorized);
}
