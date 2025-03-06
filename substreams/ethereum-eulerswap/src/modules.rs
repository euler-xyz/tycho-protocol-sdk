//! Template for Protocols with contract factories
//!
//! This template provides foundational maps and store substream modules for indexing a
//! protocol where each component (e.g., pool) is deployed to a separate contract. Each
//! contract is expected to escrow its ERC-20 token balances.
//!
//! If your protocol supports native ETH, you may need to adjust the balance tracking
//! logic in `map_relative_component_balance` to account for native token handling.
//!
//! ## Assumptions
//! - Assumes each pool has a single newly deployed contract linked to it
//! - Assumes pool identifier equals the deployed contract address
//! - Assumes any price or liquidity updated correlates with a pools contract storage update.
//!
//! ## Alternative Module
//! If your protocol uses a vault-like contract to manage balances, or if pools are
//! registered within a singleton contract, refer to the `ethereum-template-singleton`
//! substream for an appropriate alternative.
//!
//! ## Warning
//! This template provides a general framework for indexing a protocol. However, it is
//! likely that you will need to adapt the steps to suit your specific use case. Use the
//! provided code with care and ensure you fully understand each step before proceeding
//! with your implementation.
//!
//! ## Example Use Case
//! For an Uniswap-like protocol where each liquidity pool is deployed as a separate
//! contract, you can use this template to:
//! - Track relative component balances (e.g., ERC-20 token balances in each pool).
//! - Index individual pool contracts as they are created by the factory contract.
//!
//! Adjustments to the template may include:
//! - Handling native ETH balances alongside token balances.
//! - Customizing indexing logic for specific factory contract behavior.
use crate::pool_factories;
use anyhow::Result;
use itertools::Itertools;
use std::collections::HashMap;
use substreams::{pb::substreams::StoreDeltas, prelude::*};
use substreams_ethereum::{pb::eth, Event};
use tycho_substreams::{
    balances::aggregate_balances_changes, contract::extract_contract_changes_builder,
    prelude::*,
};

/// Find and create all relevant protocol components
///
/// This method maps over blocks and instantiates ProtocolComponents with a unique ids
/// as well as all necessary metadata for routing and encoding.
#[substreams::handlers::map]
fn map_protocol_components(block: eth::v2::Block) -> Result<BlockTransactionProtocolComponents> {
    // Gather contract changes by indexing `PoolDeployed` events and analysing the `Create` call
    // We store these as a hashmap by tx hash since we need to agg by tx hash later
    Ok(BlockTransactionProtocolComponents {
        tx_components: block
            .transactions()
            .filter_map(|tx| {
                let components = tx
                    .logs_with_calls()
                    .filter_map(|(log, call)| {
                        // TODO: ensure this method is implemented correctly
                        pool_factories::maybe_create_component(call.call, log, tx)
                    })
                    .collect::<Vec<_>>();

                if !components.is_empty() {
                    Some(TransactionProtocolComponents { tx: Some(tx.into()), components })
                } else {
                    None
                }
            })
            .collect::<Vec<_>>(),
    })
}

/// Stores all protocol components in a store.
///
/// Creates a mapping between pool addresses and their corresponding pool IDs,
/// allowing efficient lookups of pool IDs when only the address is known.
/// The store uses the format "pool:0xADDRESS" as the key.
#[substreams::handlers::store]
fn store_protocol_components(
    map_protocol_components: BlockTransactionProtocolComponents,
    store: StoreSetString,
) {
    map_protocol_components
        .tx_components
        .into_iter()
        .for_each(|tx_pc| {
            tx_pc
                .components
                .into_iter()
                .for_each(|pc| {
                    // Store using format "pool:0xADDRESS" -> full pool ID
                    // For Eulerswap, the id is already the pool address
                    store.set(0, format!("pool:{0}", &pc.id[..42]), &pc.id);
                    
                    // Store token addresses if available (index 0 and 1 in the tokens array)
                    if pc.tokens.len() >= 2 {
                        // Store asset0 (token 0)
                        store.set(
                            0,
                            format!("pool:{}:asset0", &pc.id[..42]),
                            &format!("0x{}", hex::encode(&pc.tokens[0])),
                        );
                        
                        // Store asset1 (token 1)
                        store.set(
                            0,
                            format!("pool:{}:asset1", &pc.id[..42]),
                            &format!("0x{}", hex::encode(&pc.tokens[1])),
                        );
                    }
                })
        });
}

/// Extracts balance changes per component by tracking Swap events
///
/// This function tracks balance changes in EulerSwap pools by monitoring Swap events.
/// When a swap occurs, it records:
/// - Positive deltas for tokens being swapped in (amount0In, amount1In)
/// - Negative deltas for tokens being swapped out (amount0Out, amount1Out)
///
/// The function relies on the store to:
/// 1. Verify the swap event came from a valid EulerSwap pool
/// 2. Look up the token addresses associated with each pool
///
/// The resulting deltas represent the net token movements for each pool component.
/// Note: This does not track direct ERC20 transfers to the pool or changes in the lending vaults,
/// only the token movements from swap events.
#[substreams::handlers::map]
fn map_relative_component_balance(
    block: eth::v2::Block,
    store: StoreGetString,
) -> Result<BlockBalanceDeltas> {
    let deltas = block
        .logs()
        .flat_map(|log| {
            let mut deltas = Vec::new();
            
            // Try to decode the Swap event
            if let Some(swap_event) = crate::abi::eulerswap::events::Swap::match_and_decode(log.log) {
                // Get the pool address from the log emitter
                let pool_address = hex::encode(log.address());
                                
                // Check if the log emitter is a known pool
                if store.get_last(format!("pool:0x{}", pool_address)).is_some() {
                    let component_id = format!("0x{}", pool_address).into_bytes();
                    
                    // Get token addresses from the store and the event
                    if let Some(asset0) = store.get_last(format!("pool:0x{}:asset0", pool_address)) {
                        if let Some(asset1) = store.get_last(format!("pool:0x{}:asset1", pool_address)) {
                            let asset0_bytes = hex::decode(&asset0[2..]).unwrap_or_default();
                            let asset1_bytes = hex::decode(&asset1[2..]).unwrap_or_default();
                            
                            // Add amount0In as a positive delta if > 0
                            if swap_event.amount0_in > substreams::scalar::BigInt::from(0) {
                                deltas.push(BalanceDelta {
                                    ord: log.ordinal(),
                                    tx: Some(log.receipt.transaction.into()),
                                    token: asset0_bytes.clone(),
                                    delta: swap_event.amount0_in.to_signed_bytes_be(),
                                    component_id: component_id.clone(),
                                });
                            }
                            
                            // Add amount1In as a positive delta if > 0
                            if swap_event.amount1_in > substreams::scalar::BigInt::from(0) {
                                deltas.push(BalanceDelta {
                                    ord: log.ordinal(),
                                    tx: Some(log.receipt.transaction.into()),
                                    token: asset1_bytes.clone(),
                                    delta: swap_event.amount1_in.to_signed_bytes_be(),
                                    component_id: component_id.clone(),
                                });
                            }
                            
                            // Add amount0Out as a negative delta if > 0
                            if swap_event.amount0_out > substreams::scalar::BigInt::from(0) {
                                deltas.push(BalanceDelta {
                                    ord: log.ordinal(),
                                    tx: Some(log.receipt.transaction.into()),
                                    token: asset0_bytes.clone(),
                                    delta: swap_event.amount0_out.neg().to_signed_bytes_be(),
                                    component_id: component_id.clone(),
                                });
                            }
                            
                            // Add amount1Out as a negative delta if > 0
                            if swap_event.amount1_out > substreams::scalar::BigInt::from(0) {
                                deltas.push(BalanceDelta {
                                    ord: log.ordinal(),
                                    tx: Some(log.receipt.transaction.into()),
                                    token: asset1_bytes.clone(),
                                    delta: swap_event.amount1_out.neg().to_signed_bytes_be(),
                                    component_id: component_id.clone(),
                                });
                            }
                        }
                    }
                }
            }
            
            deltas
        })
        .collect::<Vec<_>>();

    Ok(BlockBalanceDeltas { balance_deltas: deltas })
}

/// Aggregates relative balances values into absolute values
///
/// Aggregate the relative balances in an additive store since tycho-indexer expects
/// absolute balance inputs.
///
/// ## Note:
/// This method should usually not require any changes.
#[substreams::handlers::store]
pub fn store_balances(deltas: BlockBalanceDeltas, store: StoreAddBigInt) {
    tycho_substreams::balances::store_balance_changes(deltas, store);
}

/// Aggregates protocol components and balance changes by transaction.
///
/// This is the main method that will aggregate all changes as well as extract all
/// relevant contract storage deltas.
///
/// ## Note:
/// You may have to change this method if your components have any default dynamic
/// attributes, or if you need any additional static contracts indexed.
#[substreams::handlers::map]
fn map_protocol_changes(
    block: eth::v2::Block,
    new_components: BlockTransactionProtocolComponents,
    components_store: StoreGetString,
    balance_store: StoreDeltas,
    deltas: BlockBalanceDeltas,
) -> Result<BlockChanges, substreams::errors::Error> {
    // We merge contract changes by transaction (identified by transaction index)
    // making it easy to sort them at the very end.
    let mut transaction_changes: HashMap<_, TransactionChangesBuilder> = HashMap::new();

    // Aggregate newly created components per tx
    new_components
        .tx_components
        .iter()
        .for_each(|tx_component| {
            // initialise builder if not yet present for this tx
            let tx = tx_component.tx.as_ref().unwrap();
            let builder = transaction_changes
                .entry(tx.index)
                .or_insert_with(|| TransactionChangesBuilder::new(tx));

            // iterate over individual components created within this tx
            tx_component
                .components
                .iter()
                .for_each(|component| {
                    builder.add_protocol_component(component);
                    // Add default attributes for Eulerswap pools
                    builder.add_entity_change(&EntityChanges {
                        component_id: component.id.clone(),
                        attributes: vec![
                            // TODO: check this, as tokens are held in vaults, and vault shares are held by swap account
                            Attribute {
                                name: "balance_owner".to_string(),
                                value: component.id[2..42].as_bytes().to_vec(),
                                change: ChangeType::Creation.into(),
                            },
                            Attribute {
                                name: "update_marker".to_string(),
                                value: vec![1u8],
                                change: ChangeType::Creation.into(),
                            },
                        ],
                    });
                });
        });

    // Aggregate absolute balances per transaction.
    aggregate_balances_changes(balance_store, deltas)
        .into_iter()
        .for_each(|(_, (tx, balances))| {
            let builder = transaction_changes
                .entry(tx.index)
                .or_insert_with(|| TransactionChangesBuilder::new(&tx));
            balances
                .values()
                .for_each(|token_bc_map| {
                    token_bc_map
                        .values()
                        .for_each(|bc| builder.add_balance_change(bc))
                });
        });

    // Extract and insert any storage changes that happened for any of the components.
    extract_contract_changes_builder(
        &block,
        |addr| {
            // Check if this address belongs to a known pool
            components_store
                .get_last(format!("pool:0x{}", hex::encode(addr)))
                .is_some()
        },
        &mut transaction_changes,
    );

    // Process all `transaction_changes` for final output in the `BlockChanges`,
    //  sorted by transaction index (the key).
    Ok(BlockChanges {
        block: Some((&block).into()),
        changes: transaction_changes
            .drain()
            .sorted_unstable_by_key(|(index, _)| *index)
            .filter_map(|(_, builder)| builder.build())
            .collect::<Vec<_>>(),
    })
}
