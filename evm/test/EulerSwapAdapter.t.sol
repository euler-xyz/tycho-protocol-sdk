// SPDX-License-Identifier: AGPL-3.0-or-later
pragma solidity ^0.8.13;

import "./AdapterTest.sol";
import {EulerSwapAdapter, IERC20} from "src/euler-swap/EulerSwapAdapter.sol";
import {FractionMath} from "src/libraries/FractionMath.sol";

contract EulerSwapAdapterTest is AdapterTest {
    using FractionMath for Fraction;

    EulerSwapAdapter adapter;
    address constant WETH = 0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2;
    address constant USDC = 0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48;

    function setUp() public {
        uint256 forkBlock = 21845705;
        vm.createSelectFork(vm.rpcUrl("mainnet"), forkBlock);
        adapter =
            new EulerSwapAdapter(0x5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6f);

        vm.label(address(adapter), "EulerSwapAdapter");
        vm.label(WETH, "WETH");
        vm.label(USDC, "USDC");
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

    // function testGetLimits() public {
    //     bytes32 pair = bytes32(bytes20(USDC_WETH_POOL));
    //     uint256[] memory limits = adapter.getLimits(pair, USDC, WETH);
    //     assertEq(limits.length, 2);
    // }
}