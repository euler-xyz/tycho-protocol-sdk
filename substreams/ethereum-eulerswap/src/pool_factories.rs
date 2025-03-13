use substreams::hex;
use substreams_ethereum::pb::eth::v2::{Call, Log, TransactionTrace};
use tycho_substreams::{
    attributes::json_serialize_bigint_list,
    models::{ImplementationType, ProtocolComponent},
};
use substreams_ethereum::{Event, Function};
use crate::modules::{
    EVC_ADDRESS, EVK_EVAULT_IMPL, 
    EVK_VAULT_MODULE_IMPL, EVK_BORROWING_MODULE_IMPL,
    EVK_GOVERNANCE_MODULE_IMPL, EVK_GENERIC_FACTORY
};

/// Format a component ID consistently
pub fn format_component_id(address: &[u8]) -> String {
    format!("0x{}", hex::encode(address))
}

/// Attempts to create a new ProtocolComponent from a EulerSwap pool or vault deployment
///
/// This method takes a call, log and transaction trace and checks if they represent
/// a EulerSwap pool or vault deployment by matching against the factory addresses and decoding
/// the appropriate deployment calls and events.
///
/// For pools, it matches against the pool factory address and decodes the DeployPool call 
/// and PoolDeployed event.
///
/// For vaults, it matches against the vault factory address and decodes the corresponding events.
///
/// If successful, returns a ProtocolComponent containing:
/// For pools:
///   - Pool ID and address
///   - Token pair addresses
///   - Associated contract addresses (pool, vaults, EVC)
///   - Pool attributes (type, swap account, fees, prices, concentrations, reserves)
/// For vaults:
///   - Vault ID and address
///   - Token address
///   - Associated contract addresses (vault, implementation contracts, EVC)
///   - Vault attributes (type, implementation addresses, module addresses)
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
            let _deploy_call = crate::abi::eulerswap_factory::functions::DeployPool::match_and_decode(call)?;
            // Try to decode the PoolDeployed event
            let pool_deployed = crate::abi::eulerswap_factory::events::PoolDeployed::match_and_decode(log)?;
            
            // Format reserves for attributes
            let reserves = vec![
                pool_deployed.reserve0.clone(),
                pool_deployed.reserve1.clone()
            ];
            
            // Format prices
            let prices = vec![
                pool_deployed.price_x.clone(),
                pool_deployed.price_y.clone()
            ];
            
            // Format concentrations
            let concentrations = vec![
                pool_deployed.concentration_x.clone(),
                pool_deployed.concentration_y.clone()
            ];

            // Create a ProtocolComponent with the proper ID
            let mut component = ProtocolComponent::new(&format_component_id(&pool_deployed.pool));
            
            // Add tokens
            component = component.with_tokens(&[
                pool_deployed.asset0.clone(),  // First token
                pool_deployed.asset1.clone(),  // Second token
            ]);
            
            // Add contracts
            component = component.with_contracts(&[
                pool_deployed.pool.clone(),     // The deployed pool contract
                pool_deployed.vault0.clone(),   // Vault0 contract
                pool_deployed.vault1.clone(),   // Vault1 contract
                EVC_ADDRESS.to_vec()            // EVC address
            ]);
            
            // Add attributes
            component = component.with_attributes(&[
                ("pool_type", "EulerSwap".as_bytes()),
                ("euler_account", &pool_deployed.euler_account),
                ("fee_multiplier", &pool_deployed.fee_multiplier.to_signed_bytes_be()),
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
        },
        // EVK Generic Factory for vault deployments
        hex!("29a56a1b8214D9Cf7c5561811750D5cBDb45CC8e") => {
            // Try to decode the ProxyCreated event
            let proxy_created = crate::abi::evk_generic_factory::events::ProxyCreated::match_and_decode(log)?;
            
            // Create a ProtocolComponent with the vault address as the ID
            let mut component = ProtocolComponent::new(&format_component_id(&proxy_created.proxy));
            
            // Add contracts
            component = component.with_contracts(&[
                proxy_created.proxy.clone(),            // The deployed vault contract
                proxy_created.implementation.clone(),   // Implementation contract
                EVC_ADDRESS.to_vec()                    // EVC address
            ]);
            
            // Add attributes
            component = component.with_attributes(&[
                ("vault_type", "EulerLending".as_bytes()),
                ("upgradeable", if proxy_created.upgradeable { &[1u8] } else { &[0u8] }),
                ("trailing_data_length", &(proxy_created.trailing_data.len() as u32).to_be_bytes()),
                // Add stateless contract address
                ("stateless_contract_addr_0", &EVK_EVAULT_IMPL),
                ("stateless_contract_addr_1", &EVK_VAULT_MODULE_IMPL),
                ("stateless_contract_addr_2", &EVK_BORROWING_MODULE_IMPL),
                ("stateless_contract_addr_3", &EVK_GOVERNANCE_MODULE_IMPL),
                ("stateless_contract_addr_4", &EVK_GENERIC_FACTORY),
                ("manual_updates", &[1u8]),
            ]);
            
            // Set protocol type as Lend instead of Swap for vaults
            component.protocol_type = Some(tycho_substreams::models::ProtocolType {
                name: "eulerswap".to_string(),
                financial_type: tycho_substreams::models::FinancialType::Lend.into(),
                attribute_schema: vec![],
                implementation_type: ImplementationType::Vm.into(),
            });
            
            Some(component)
        }
        _ => None,
    }
}
