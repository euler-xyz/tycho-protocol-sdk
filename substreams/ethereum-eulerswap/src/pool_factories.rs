use crate::modules::{
    EVC_ADDRESS, EVK_BORROWING_MODULE_IMPL, EVK_EVAULT_IMPL, EVK_GENERIC_FACTORY,
    EVK_GOVERNANCE_MODULE_IMPL, EVK_VAULT_MODULE_IMPL,
};
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
/// Otherwise returns None.
pub fn maybe_create_component(
    call: &Call,
    log: &Log,
    _tx: &TransactionTrace,
) -> Option<ProtocolComponent> {
    match *call.address {
        // EulerSwap Factory address
        hex!("F75548aF02f1928CbE9015985D4Fcbf96d728544") => {
            // Try to decode the DeployPool call (not used for now)
            let _deploy_call =
                crate::abi::eulerswap_factory::functions::DeployPool::match_and_decode(call)?;
            // Try to decode the PoolDeployed event
            let pool_deployed =
                crate::abi::eulerswap_factory::events::PoolDeployed::match_and_decode(log)?;

            // Format reserves for attributes
            let reserves = vec![pool_deployed.reserve0.clone(), pool_deployed.reserve1.clone()];

            // Format prices
            let prices = vec![pool_deployed.price_x.clone(), pool_deployed.price_y.clone()];

            // Format concentrations
            let concentrations =
                vec![pool_deployed.concentration_x.clone(), pool_deployed.concentration_y.clone()];

            // Create a ProtocolComponent with the proper ID
            let mut component = ProtocolComponent::new(&format_pool_id(&pool_deployed.pool));

            // Add tokens
            component = component.with_tokens(&[
                pool_deployed.asset0.clone(), // First token
                pool_deployed.asset1.clone(), // Second token
            ]);

            // Add contracts
            component = component.with_contracts(&[
                pool_deployed.pool.clone(),   // The deployed pool contract
                pool_deployed.vault0.clone(), // Vault0 contract
                pool_deployed.vault1.clone(), // Vault1 contract
                EVC_ADDRESS.to_vec(),         // EVC address
            ]);

            // Add attributes
            component = component.with_attributes(&[
                ("pool_type", "EulerSwap".as_bytes()),
                ("euler_account", &pool_deployed.euler_account),
                (
                    "fee_multiplier",
                    &pool_deployed
                        .fee_multiplier
                        .to_signed_bytes_be(),
                ),
                ("reserves", &json_serialize_bigint_list(&reserves)),
                ("prices", &json_serialize_bigint_list(&prices)),
                ("concentrations", &json_serialize_bigint_list(&concentrations)),
                // Add stateless contract address
                ("stateless_contract_addr_0", &EVK_EVAULT_IMPL),
                ("stateless_contract_addr_1", &EVK_VAULT_MODULE_IMPL),
                ("stateless_contract_addr_2", &EVK_BORROWING_MODULE_IMPL),
                ("stateless_contract_addr_3", &EVK_GOVERNANCE_MODULE_IMPL),
                ("stateless_contract_addr_4", &EVK_GENERIC_FACTORY),
                ("manual_updates", &[1u8]),
            ]);

            // Set protocol type
            component = component.as_swap_type("eulerswap", ImplementationType::Vm);

            Some(component)
        }
        _ => None,
    }
}
