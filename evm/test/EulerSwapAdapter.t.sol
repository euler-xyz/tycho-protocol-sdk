// SPDX-License-Identifier: AGPL-3.0-or-later
pragma solidity ^0.8.13;

import "./AdapterTest.sol";
import {
    IERC20,
    EulerSwapAdapter,
    IEulerSwap,
    IEulerSwapPeriphery,
    SafeERC20
} from "src/eulerswap/EulerSwapAdapter.sol";
import {FractionMath} from "src/libraries/FractionMath.sol";
import {IERC4626} from "openzeppelin-contracts/contracts/interfaces/IERC4626.sol";
import "forge-std/Test.sol";

address constant EULER_SWAP_FACTORY = 0xa4891c18f036F14d7975b0869D77eA7c7032e0fF;
address constant EULER_SWAP_PERIPHERY = 0xb653fb145B2EC8412E74eaB1a48756c54B083A0E;
address constant USDC = 0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48;
address constant USDT = 0xdAC17F958D2ee523a2206206994597C13D831ec7;
address constant USDC_USDT_POOL = 0x93839Ae283b2A827210586DEbA7b1dE50A122888;

uint256 constant FORK_BLOCK = 22431196;


contract EulerSwapAdapterTest is AdapterTest {
    using FractionMath for Fraction;

    EulerSwapAdapter public adapter;

    function setUp() public {
        vm.createSelectFork(vm.rpcUrl("mainnet"), FORK_BLOCK);
        adapter = new EulerSwapAdapter(
            EULER_SWAP_FACTORY,
            EULER_SWAP_PERIPHERY
        );

        vm.label(address(adapter), "EulerSwapAdapter");
        vm.label(USDC, "USDC");
        vm.label(USDT, "USDT");
    }

    function testEulerSwap_Swap() public {
        bytes32 poolId = bytes32(bytes20(USDC_USDT_POOL));

        address swapper = makeAddr("swapper");
        deal(USDC, swapper, 10e6);

        uint256 amountIn = 5e6;

        vm.startPrank(swapper);
        IERC20(USDC).approve(address(adapter), amountIn);
        Trade memory trade = adapter.swap(poolId, USDC, USDT, OrderSide.Sell, amountIn);
        vm.stopPrank();

        assertEq(trade.calculatedAmount, 5e6);
        assertEq(trade.gasUsed, 750000);
    }

    function testEulerSwap_Price() public {
        bytes32 poolId = bytes32(bytes20(USDC_USDT_POOL));

        uint256[] memory specifiedAmounts = new uint256[](3);
        specifiedAmounts[0] = 100e6;
        specifiedAmounts[0] = 200e6;
        specifiedAmounts[0] = 300e6;

        vm.expectRevert(
            abi.encodeWithSelector(
                ISwapAdapterTypes.NotImplemented.selector,
                "Price function not implemented"
            )
        );
        adapter.price(poolId, USDC, USDT, specifiedAmounts);
    }

    // function testEulerSwap_Price() public view {
    //     bytes32 poolId = bytes32(bytes20(USDC_USDT_POOL));

    //     uint256[] memory specifiedAmounts = new uint256[](3);
    //     specifiedAmounts[0] = 100e6;
    //     specifiedAmounts[0] = 200e6;
    //     specifiedAmounts[0] = 300e6;

    //     Fraction[] memory prices =
    //         adapter.price(poolId, USDC, USDT, specifiedAmounts);

    //     assertEq(prices.length, specifiedAmounts.length);

    //     assertApproxEqAbs(prices[0].numerator, specifiedAmounts[0], 10e6);
    //     assertEq(prices[0].denominator, specifiedAmounts[0]);
    //     assertApproxEqAbs(prices[1].numerator, specifiedAmounts[1], 10e6);
    //     assertEq(prices[1].denominator, specifiedAmounts[1]);
    //     assertApproxEqAbs(prices[2].numerator, specifiedAmounts[2], 10e6);
    //     assertEq(prices[2].denominator, specifiedAmounts[2]);
    // }

    function testEulerSwap_GetLimits() public view {
        bytes32 poolId = bytes32(bytes20(USDC_USDT_POOL));
        uint256[] memory limits01 = adapter.getLimits(poolId, USDC, USDT);

        assertEq(limits01.length, 2);
        assertGt(limits01[0], 0);
        assertGt(limits01[1], 0);

        uint256[] memory limits10 = adapter.getLimits(poolId, USDT, USDC);

        assertEq(limits10.length, 2);
        assertGt(limits10[0], 0);
        assertGt(limits10[1], 0);

    }

    function testEulerSwap_GetCapabilities(
        bytes32 poolId,
        address sellToken,
        address buyToken
    ) public view {
        Capability[] memory res =
            adapter.getCapabilities(poolId, sellToken, buyToken);

        assertEq(res.length, 3);
    }

    function testEulerSwap_GetTokens() public view {
        bytes32 poolId = bytes32(bytes20(USDC_USDT_POOL));
        address[] memory tokens = adapter.getTokens(poolId);

        assertEq(tokens.length, 2);
        assertEq(tokens[0], USDC);
        assertEq(tokens[1], USDT);
    }

    function testEulerSwap_GetPoolIds() public view {
        bytes32[] memory ids = adapter.getPoolIds(0, 1);

        assertEq(ids.length, 1);
        assertEq(ids[0], bytes32(bytes20(USDC_USDT_POOL)));
    }
}


contract EulerSwapAdapterInvariants is Test {
    EulerSwapAdapter internal adapter;
    EulerSwapAdapterHandler internal handler;

    function setUp() public {
        vm.createSelectFork(vm.rpcUrl("mainnet"), FORK_BLOCK);
        adapter = new EulerSwapAdapter(
            EULER_SWAP_FACTORY,
            EULER_SWAP_PERIPHERY
        );

        handler = new EulerSwapAdapterHandler(adapter, USDC_USDT_POOL);
        targetContract(address(handler));
    }

    /// @dev The handler will fuzz swaps and simulate them through quoting only adapter and 
    /// through actual swap simulation. The invariant makes sure the outcomes are the same.
    function invariant_AdapterStateMatchesPool() external {
        vm.skip(true);

        IEulerSwap.Params memory p = IEulerSwap(USDC_USDT_POOL).getParams();
        address asset0 = IERC4626(p.vault0).asset();
        address asset1 = IERC4626(p.vault1).asset();

        EulerSwapAdapter.PoolCache memory cache = adapter.getPoolCache(USDC_USDT_POOL);
        
        (uint112 reserve0, uint112 reserve1,) = IEulerSwap(USDC_USDT_POOL).getReserves();
        assertApproxEqAbs(reserve0, cache.reserve0, 1);
        assertApproxEqAbs(reserve1, cache.reserve1, 1);

        (uint256 limit0, uint256 limit1) = IEulerSwap(USDC_USDT_POOL).getLimits(asset0, asset1);
        assertApproxEqAbs(cache.limit0to1.limitIn, limit0, 1);
        assertApproxEqAbs(cache.limit0to1.limitOut, limit1, 1);

        (limit0, limit1) = IEulerSwap(USDC_USDT_POOL).getLimits(asset1, asset0);
        assertApproxEqAbs(cache.limit1to0.limitIn, limit0, 1);
        assertApproxEqAbs(cache.limit1to0.limitOut, limit1, 1);
    }
}

contract EulerSwapAdapterHandler is Test {
    using SafeERC20 for IERC20;

    EulerSwapAdapter internal adapter;
    IEulerSwap pool;
    bytes32 poolId;
    address asset0;
    address asset1;

    constructor(EulerSwapAdapter _adapter, address _pool) {
        adapter = _adapter;
        pool = IEulerSwap(_pool);

        IEulerSwap.Params memory p = pool.getParams();
        asset0 = IERC4626(p.vault0).asset();
        asset1 = IERC4626(p.vault1).asset();
        poolId = bytes20(_pool);

        IERC20(asset0).safeIncreaseAllowance(EULER_SWAP_PERIPHERY, type(uint256).max);
        IERC20(asset1).safeIncreaseAllowance(EULER_SWAP_PERIPHERY, 0);
        deal(asset0, address(this), 1e25);
        deal(asset1, address(this), 1e25);

        adapter.initializeCache(_pool);
    }

    function swap(ISwapAdapter.OrderSide side, bool isAsset0In, uint256 amount) public {
        // 1 wei amounts will hit zero shares error on deposit, ignore
        if (amount <= 1) return;

        (address assetIn, address assetOut) = isAsset0In ? (asset0, asset1) : (asset1, asset0);
        (uint256 limitIn, uint256 limitOut) = pool.getLimits(assetIn, assetOut);

        if (side == ISwapAdapterTypes.OrderSide.Sell && amount > limitIn) return; 
        if (side == ISwapAdapterTypes.OrderSide.Buy && amount > limitOut) return; 

        // quote swap through the adapter, remembering the results
        adapter.swap(poolId, assetIn, assetOut, side, amount);


        // simulate actual swap
        if (side == ISwapAdapterTypes.OrderSide.Sell) {
            IEulerSwapPeriphery(EULER_SWAP_PERIPHERY).swapExactIn(
                address(pool),
                assetIn,
                assetOut,
                amount,
                address(this),
                0,
                type(uint256).max
            );
        } else {
            IEulerSwapPeriphery(EULER_SWAP_PERIPHERY).swapExactOut(
                address(pool),
                assetIn,
                assetOut,
                amount,
                address(this),
                type(uint256).max,
                type(uint256).max
            );
        }
    }
}