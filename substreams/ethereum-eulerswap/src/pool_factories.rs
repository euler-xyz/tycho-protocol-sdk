use substreams::hex;
use substreams_ethereum::pb::eth::v2::{Call, Log, TransactionTrace};
use tycho_substreams::models::{
    ChangeType, FinancialType, ImplementationType, ProtocolComponent, ProtocolType,
};
use substreams_ethereum::{Event, Function};

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
        hex!("04C54FF83e4BC428FD1eDA2f41cdBd583A2e9cF8") => {
            // Try to decode the DeployPool call (not used for now)
            let _deploy_call = crate::abi::eulerswap_factory::functions::DeployPool::match_and_decode(call)?;
            // Try to decode the PoolDeployed event
            let pool_deployed = crate::abi::eulerswap_factory::events::PoolDeployed::match_and_decode(log)?;

            Some(ProtocolComponent {
                id: format!("0x{}", hex::encode(&pool_deployed.pool)),
                tokens: vec![
                    pool_deployed.asset0.clone(),  // First token
                    pool_deployed.asset1.clone(),  // Second token
                ],
                contracts: vec![
                    pool_deployed.pool.clone(),          // The deployed pool contract
                    pool_deployed.vault0.clone(),        // Vault0 contract
                    pool_deployed.vault1.clone(),        // Vault1 contract
                ],
                static_att: vec![
                    tycho_substreams::models::Attribute {
                        name: "pool_type".to_string(),
                        value: "EulerSwap".as_bytes().to_vec(),
                        change: ChangeType::Creation.into(),
                    },
                    tycho_substreams::models::Attribute {
                        name: "swap_account".to_string(),
                        value: pool_deployed.swap_account.clone(),
                        change: ChangeType::Creation.into(),
                    },
                    tycho_substreams::models::Attribute {
                        name: "fee_multiplier".to_string(),
                        value: pool_deployed.fee_multiplier.to_signed_bytes_be(),
                        change: ChangeType::Creation.into(),
                    },
                    tycho_substreams::models::Attribute {
                        name: "price_x".to_string(),
                        value: pool_deployed.price_x.to_signed_bytes_be(),
                        change: ChangeType::Creation.into(),
                    },
                    tycho_substreams::models::Attribute {
                        name: "price_y".to_string(),
                        value: pool_deployed.price_y.to_signed_bytes_be(),
                        change: ChangeType::Creation.into(),
                    },
                    tycho_substreams::models::Attribute {
                        name: "concentration_x".to_string(),
                        value: pool_deployed.concentration_x.to_signed_bytes_be(),
                        change: ChangeType::Creation.into(),
                    },
                    tycho_substreams::models::Attribute {
                        name: "concentration_y".to_string(),
                        value: pool_deployed.concentration_y.to_signed_bytes_be(),
                        change: ChangeType::Creation.into(),
                    },
                ],
                change: ChangeType::Creation.into(),
                protocol_type: Some(ProtocolType {
                    name: "eulerswap".to_string(),
                    financial_type: FinancialType::Swap.into(),
                    attribute_schema: vec![],
                    implementation_type: ImplementationType::Vm.into(),
                }),
            })
        }
        _ => None,
    }
}
