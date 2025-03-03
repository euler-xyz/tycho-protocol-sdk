// SPDX-License-Identifier: AGPL-3.0-or-later
pragma solidity ^0.8.13;

import "./AdapterTest.sol";
import {
    IERC20,
    EulerSwapAdapter,
    IEulerSwap
} from "src/eulerswap/EulerSwapAdapter.sol";
import {FractionMath} from "src/libraries/FractionMath.sol";

contract EulerSwapAdapterTest is AdapterTest {
    using FractionMath for Fraction;

    address constant USDC = 0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48;
    address constant USDT = 0xdAC17F958D2ee523a2206206994597C13D831ec7;
    address constant USDC_USDT_POOL = 0x67C30405250e395f31d661274352dA404e624682;

    EulerSwapAdapter public adapter;

    function setUp() public {
        uint256 forkBlock = 21923492;
        vm.createSelectFork(vm.rpcUrl("mainnet"), forkBlock);
        adapter = new EulerSwapAdapter(
            0x04C54FF83e4BC428FD1eDA2f41cdBd583A2e9cF8,
            0x64A8410D7D2ecF3Aaf32b6C3932e4586f3C42ecE
        );

        vm.label(address(adapter), "EulerSwapAdapter");
        vm.label(USDC, "USDC");
        vm.label(USDT, "USDT");
    }

    function testSwap() public {
        bytes32 poolId = bytes32(bytes20(USDC_USDT_POOL));
        IEulerSwap pool = IEulerSwap(address(bytes20(poolId)));

        address swapper = pool.myAccount();
        uint256 amountIn = 20e6;
        uint256 usdcBalanceBefore = IERC20(USDC).balanceOf(swapper);
        uint256 usdtBalanceBefore = IERC20(USDT).balanceOf(swapper);

        vm.startPrank(swapper);
        IERC20(USDC).approve(address(adapter), amountIn);
        adapter.swap(poolId, USDC, USDT, OrderSide.Sell, amountIn);
        vm.stopPrank();

        assertGt(IERC20(USDT).balanceOf(address(swapper)), usdtBalanceBefore);
        assertLt(IERC20(USDC).balanceOf(address(swapper)), usdcBalanceBefore);
    }

    function testPrice() public view {
        bytes32 poolId = bytes32(bytes20(USDC_USDT_POOL));

        uint256[] memory specifiedAmounts = new uint256[](3);
        specifiedAmounts[0] = 100e6;
        specifiedAmounts[0] = 200e6;
        specifiedAmounts[0] = 300e6;

        Fraction[] memory prices =
            adapter.price(poolId, USDC, USDT, specifiedAmounts);

        assertEq(prices.length, specifiedAmounts.length);

        assertApproxEqAbs(prices[0].numerator, specifiedAmounts[0], 10e6);
        assertEq(prices[0].denominator, specifiedAmounts[0]);
        assertApproxEqAbs(prices[1].numerator, specifiedAmounts[1], 10e6);
        assertEq(prices[1].denominator, specifiedAmounts[1]);
        assertApproxEqAbs(prices[2].numerator, specifiedAmounts[2], 10e6);
        assertEq(prices[2].denominator, specifiedAmounts[2]);
    }

    function testFuzzPrice(uint256 specifiedAmount) public view {
        // Assume OrderSide.Sell
        uint256[] memory limits =
            adapter.getLimits(bytes32(bytes20(USDC_USDT_POOL)), USDC, USDT);

        vm.assume(specifiedAmount > 0);
        vm.assume(specifiedAmount < limits[0]);

        console2.log("specifiedAmount", specifiedAmount);

        uint256[] memory specifiedAmounts = new uint256[](1);
        specifiedAmounts[0] = specifiedAmount;

        // Fraction[] memory prices = adapter.price(
        //     bytes32(bytes20(USDC_USDT_POOL)), USDC, USDT, specifiedAmounts
        // );

        // assertGt(prices[0].numerator, 0);
        // assertGt(prices[0].denominator, 0);
    }


    function testGetLimits() public view {
        bytes32 poolId = bytes32(bytes20(USDC_USDT_POOL));
        uint256[] memory limits = adapter.getLimits(poolId, USDC, USDT);

        assertEq(limits.length, 2);
        assertGt(limits[0], 0);
        assertGt(limits[1], 0);
    }

    function testGetCapabilities(
        bytes32 poolId,
        address sellToken,
        address buyToken
    ) public view {
        Capability[] memory res =
            adapter.getCapabilities(poolId, sellToken, buyToken);

        assertEq(res.length, 3);
    }

    function testGetTokens() public view {
        bytes32 poolId = bytes32(bytes20(USDC_USDT_POOL));
        address[] memory tokens = adapter.getTokens(poolId);

        assertEq(tokens.length, 2);
        assertEq(tokens[0], USDC);
        assertEq(tokens[1], USDT);
    }

    function testGetPoolIds() public view {
        bytes32[] memory ids = adapter.getPoolIds(0, 1);

        assertEq(ids.length, 1);
        assertEq(ids[0], bytes32(bytes20(USDC_USDT_POOL)));
    }
}
