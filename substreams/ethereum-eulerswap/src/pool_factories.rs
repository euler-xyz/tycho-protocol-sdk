use crate::modules::{EULERSWAP_PERIPHERY, EVC_ADDRESS, EVK_GENERIC_FACTORY};
use substreams::hex;
use substreams_ethereum::pb::eth::v2::{Call, Log, TransactionTrace};
use substreams_ethereum::{Event, Function};
use tycho_substreams::{
    attributes::json_serialize_bigint_list,
    models::{ImplementationType, ProtocolComponent},
};

/// Format a pool ID consistently
pub fn format_pool_id(pool_address: &[u8]) -> String {
    format!("0x{}", hex::encode(pool_address))
}

/// Attempts to create a new ProtocolComponent from a EulerSwap pool deployment
///
/// This method takes a call, log and transaction trace and checks if they represent
/// a EulerSwap pool deployment by matching against the factory address and decoding
/// the DeployPool call and PoolDeployed event.
///
/// If successful, returns a ProtocolComponent containing:
/// - Pool ID and address
/// - Token pair addresses
/// - Associated contract addresses (pool, vaults)
/// - Pool attributes (type, swap account, fees, prices)
///   Otherwise returns None.
pub fn maybe_create_component(
    call: &Call,
    log: &Log,
    tx: &TransactionTrace,
) -> Option<ProtocolComponent> {
    match *call.address {
        // EulerSwap Factory address
        hex!("a4891c18f036f14d7975b0869d77ea7c7032e0ff") => {
            // Try to decode the DeployPool call (not used for now)
            let _deploy_call =
                crate::abi::eulerswap_factory::functions::DeployPool::match_and_decode(call)?;
            // Try to decode the PoolDeployed event
            let pool_deployed =
                crate::abi::eulerswap_factory::events::PoolDeployed::match_and_decode(log)?;

            // Find the matching PoolConfig event
            let pool_config_log = tx
                .logs_with_calls()
                .find(|(l, _c)| {
                    let pc= crate::abi::eulerswap_factory::events::PoolConfig::match_and_decode(l);
                    pc.is_some() && pc.unwrap().pool == pool_deployed.pool
                }).unwrap().0;

            let pool_config =
                crate::abi::eulerswap_factory::events::PoolConfig::match_and_decode(pool_config_log)?;

            // Format reserves for attributes
            let reserves = vec![pool_config.initial_state.0.clone(), pool_config.initial_state.1.clone()];

            // Decode pool params
            // struct Params {
            //     // Entities
            // 0    address vault0;
            // 1    address vault1;
            // 2    address eulerAccount;
            //     // Curve
            // 3    uint112 equilibriumReserve0;
            // 4    uint112 equilibriumReserve1;
            // 5    uint256 priceX;
            // 6    uint256 priceY;
            // 7    uint256 concentrationX;
            // 8    uint256 concentrationY;
            //     // Fees
            // 9    uint256 fee;
            // 10   uint256 protocolFee;
            // 11   address protocolFeeRecipient;
            // }
            // Format prices
            let prices = vec![pool_config.params.5.clone(), pool_config.params.6.clone()];

            // Format concentrations
            let concentrations =
                vec![pool_config.params.7.clone(), pool_config.params.8.clone()];

            // Create a ProtocolComponent with the proper ID
            let mut component = ProtocolComponent::new(&format_pool_id(&pool_deployed.pool));

            // Add tokens
            component = component.with_tokens(&[
                pool_deployed.asset0.clone(), // First token
                pool_deployed.asset1.clone(), // Second token
            ]);

            // Add contracts
            component = component.with_contracts(&[
                pool_deployed.pool.clone(),        // The deployed pool contract
                pool_config.params.0.clone(),      // Vault0 contract
                pool_config.params.1.clone(),      // Vault1 contract
                EVC_ADDRESS.to_vec(),              // EVC address
                EULERSWAP_PERIPHERY.to_vec(),      // EulerSwap periphery address
                EVK_GENERIC_FACTORY.to_vec(),      // EVK Generic factory address
            ]);

            // Add attributes
            component = component.with_attributes(&[
                ("pool_type", "EulerSwap".as_bytes()),
                ("euler_account", &pool_deployed.euler_account),
                (
                    "fee",
                    &pool_config
                        .params
                        .9
                        .to_signed_bytes_be(),
                ),
                (
                    "protocolFee",
                    &pool_config
                        .params
                        .10
                        .to_signed_bytes_be(),
                ),
                (
                    "protocolFeeRecipient",
                    &pool_config
                        .params
                        .11,
                ),
                ("reserves", &json_serialize_bigint_list(&reserves)),
                ("prices", &json_serialize_bigint_list(&prices)),
                ("concentrations", &json_serialize_bigint_list(&concentrations)),
                ("manual_updates", &[1u8]),
            ]);

            // Set protocol type
            component = component.as_swap_type("eulerswap", ImplementationType::Vm);

            Some(component)
        }
        _ => None,
    }
}
