// SPDX-License-Identifier: AGPL-3.0-or-later
pragma solidity ^0.8.13;

import "./AdapterTest.sol";
import {EulerSwapAdapter, IERC20} from "src/eulerswap/EulerSwapAdapter.sol";
import {FractionMath} from "src/libraries/FractionMath.sol";

contract EulerSwapAdapterTest is AdapterTest {
    using FractionMath for Fraction;

    address constant USDC = 0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48;
    address constant USDT = 0xdAC17F958D2ee523a2206206994597C13D831ec7;

    EulerSwapAdapter public adapter;

    function setUp() public {
        uint256 forkBlock = 21915599;
        vm.createSelectFork(vm.rpcUrl("mainnet"), forkBlock);
        adapter = new EulerSwapAdapter(
            0xB6cFe9b23d18A034cE925Ee84b97D20a52Db1940,
            0x7fc1edF54d86DfAA90F1069E81D4B520A2A44d2B
        );

        vm.label(address(adapter), "EulerSwapAdapter");
        vm.label(USDC, "USDC");
        vm.label(USDT, "USDT");
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
