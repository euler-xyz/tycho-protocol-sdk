//! EulerSwap Substreams Module
//!
//! This module provides foundational maps and store substream modules for indexing the
//! EulerSwap protocol. Each pool component is deployed to a separate contract with its
//! own ERC-20 token balances.
//!
//! ## Architecture Overview
//! - Each EulerSwap pool is a separate contract for a token pair
//! - Tokens are stored in separate vault contracts
//! - Each pool has an Euler Account for liquidity management
//! - Pool identifier equals the deployed contract address (bytes20)
//!
//! ## Implementation Notes
//! - Address format is standardized as "0x{hex}" throughout
//! - Store keys follow consistent patterns: "pool:{id}" and "pool:{id}:{property}"
//! - Balance tracking focuses on Swap events and initial deployments
use crate::pool_factories::{self, format_pool_id};
use anyhow::Result;
use itertools::Itertools;
use std::collections::HashMap;
#[allow(unused_imports)]
use substreams::{pb::substreams::StoreDeltas, prelude::*, hex};
use substreams_ethereum::{pb::eth, Event};
use tycho_substreams::{
    balances::aggregate_balances_changes, 
    contract::extract_contract_changes_builder,
    prelude::*,
    abi::erc20,
};

pub const EVC_ADDRESS: &[u8] = &hex!("0C9a3dd6b8F28529d72d7f9cE918D493519EE383");
pub const EULERSWAP_PERIPHERY: &[u8] = &hex!("813d74e832b3d9e9451d8f0e871e877edf2a5a5f");
pub const EVK_EVAULT_IMPL: &[u8] = &hex!("8ff1c814719096b61abf00bb46ead0c9a529dd7d");
pub const EVK_VAULT_MODULE_IMPL: &[u8] = &hex!("b4ad4d9c02c01b01cf586c16f01c58c73c7f0188");
pub const EVK_BORROWING_MODULE_IMPL: &[u8] = &hex!("639156f8feb0cd88205e4861a0224ec169605acf");
pub const EVK_GOVERNANCE_MODULE_IMPL: &[u8] = &hex!("a61f5016f2cd5cec12d091f871fce1e1df5f0b67");
pub const EVK_GENERIC_FACTORY: &[u8] = &hex!("29a56a1b8214d9cf7c5561811750d5cbdb45cc8e");

// Store key prefixes and suffixes for consistency
const POOL_PREFIX: &str = "pool:";
const ASSET0_SUFFIX: &str = ":asset0";
const ASSET1_SUFFIX: &str = ":asset1";
const VAULT0_SUFFIX: &str = ":vault0";
const VAULT1_SUFFIX: &str = ":vault1";
const TOKEN_PREFIX: &str = "token:";
const VAULT_PREFIX: &str = "vault:";

/// Format a store key for a pool
fn pool_key(pool_id: &str) -> String {
    format!("{}{}", POOL_PREFIX, pool_id)
}

/// Format a store key for a pool's asset
fn pool_asset_key(pool_id: &str, is_asset0: bool) -> String {
    if is_asset0 {
        format!("{}{}{}", POOL_PREFIX, pool_id, ASSET0_SUFFIX)
    } else {
        format!("{}{}{}", POOL_PREFIX, pool_id, ASSET1_SUFFIX)
    }
}

/// Format a store key for a pool's vault
fn pool_vault_key(pool_id: &str, is_vault0: bool) -> String {
    if is_vault0 {
        format!("{}{}{}", POOL_PREFIX, pool_id, VAULT0_SUFFIX)
    } else {
        format!("{}{}{}", POOL_PREFIX, pool_id, VAULT1_SUFFIX)
    }
}

/// Format a store key for a token lookup
fn token_key(token_addr: &str) -> String {
    format!("{}{}", TOKEN_PREFIX, token_addr)
}

/// Format a store key for a vault lookup
fn vault_key(vault_addr: &str) -> String {
    format!("{}{}", VAULT_PREFIX, vault_addr)
}

/// Store an address in a consistent format
fn store_address(address: &[u8]) -> String {
    format_pool_id(address)
}

/// Decode an address string back to bytes
fn decode_address(address_str: &str) -> Vec<u8> {
    // Skip the "0x" prefix
    // This function uses the 'hex' module imported above
    hex::decode(&address_str[2..]).unwrap_or_default()
}

/// Find and create all relevant protocol components
///
/// This method maps over blocks and instantiates ProtocolComponents with unique ids
/// as well as all necessary metadata for routing and encoding.
#[substreams::handlers::map]
fn map_protocol_components(block: eth::v2::Block) -> Result<BlockTransactionProtocolComponents> {
    // Gather contract changes by indexing `PoolDeployed` events and analyzing the `Create` call
    // We store these as a hashmap by tx hash since we need to agg by tx hash later
    Ok(BlockTransactionProtocolComponents {
        tx_components: block
            .transactions()
            .filter_map(|tx| {
                let components = tx
                    .logs_with_calls()
                    .filter_map(|(log, call)| {
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
/// The store uses the format "pool:{ID}" as the key.
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
                    // Extract the pool ID (should already be in "0x{hex}" format)
                    let pool_id = &pc.id;
                    
                    // Store using consistent format "pool:{ID}" -> full pool ID
                    store.set(0, pool_key(pool_id), pool_id);
                    
                    // Store token addresses
                    // Store asset0 (token 0) with consistent formatting
                    let token0_addr = &store_address(&pc.tokens[0]);
                    store.set(
                        0,
                        pool_asset_key(pool_id, true),
                        token0_addr,
                    );
                    
                    // Add reverse index for token lookup
                    store.set(0, token_key(token0_addr), token0_addr);
                    
                    // Store asset1 (token 1) with consistent formatting
                    let token1_addr = &store_address(&pc.tokens[1]);
                    store.set(
                        0,
                        pool_asset_key(pool_id, false),
                        token1_addr,
                    );
                    
                    // Add reverse index for token lookup
                    store.set(0, token_key(token1_addr), token1_addr);

                    // Store vault addresses
                    // Store vault0 (contract 1) with consistent formatting
                    let vault0_addr = &store_address(&pc.contracts[1]);
                    store.set(
                        0,
                        pool_vault_key(pool_id, true),
                        vault0_addr,
                    );
                    
                    // Add reverse index for vault lookup
                    store.set(0, vault_key(vault0_addr), vault0_addr);
                    
                    // Store vault1 (contract 2) with consistent formatting
                    let vault1_addr = &store_address(&pc.contracts[2]);
                    store.set(
                        0,
                        pool_vault_key(pool_id, false),
                        vault1_addr,
                    );
                    
                    // Add reverse index for vault lookup
                    store.set(0, vault_key(vault1_addr), vault1_addr);
                })
        });
}

/// Maps token balance deltas for each EulerSwap pool component in a block
///
/// This function tracks:
/// - Initial pool balances from PoolDeployed events (reserve0 and reserve1)
/// - Positive deltas for tokens being swapped in (amount0In, amount1In)
/// - Negative deltas for tokens being swapped out (amount0Out, amount1Out)
///
/// The function relies on the store to:
/// 1. Verify the swap event came from a valid EulerSwap pool
/// 2. Look up the token addresses associated with each pool
#[substreams::handlers::map]
fn map_relative_component_balance(
    block: eth::v2::Block,
    store: StoreGetString,
) -> Result<BlockBalanceDeltas> {
    let deltas = block
        .logs()
        .flat_map(|log| {
            let mut deltas = Vec::new();
            
            // Try to decode the PoolDeployed event from the factory
            if let Some(deploy_event) = crate::abi::eulerswap_factory::events::PoolDeployed::match_and_decode(log.log) {
                // Format the pool ID consistently
                let pool_id = format_pool_id(&deploy_event.pool);
                
                // Check if the pool is already in the store
                if store.get_last(pool_key(&pool_id)).is_some() {
                    // Get token addresses from the event
                    let asset0_bytes = deploy_event.asset0.clone();
                    let asset1_bytes = deploy_event.asset1.clone();
                    
                    // Add reserve0 as the initial balance for asset0
                    if deploy_event.reserve0 > substreams::scalar::BigInt::from(0) {
                        deltas.push(BalanceDelta {
                            ord: log.ordinal(),
                            tx: Some(log.receipt.transaction.into()),
                            token: asset0_bytes.clone(),
                            component_id: pool_id.clone().into_bytes(),
                            delta: deploy_event.reserve0.to_signed_bytes_be(),
                        });
                    }
                    
                    // Add reserve1 as the initial balance for asset1
                    if deploy_event.reserve1 > substreams::scalar::BigInt::from(0) {
                        deltas.push(BalanceDelta {
                            ord: log.ordinal(),
                            tx: Some(log.receipt.transaction.into()),
                            token: asset1_bytes.clone(),
                            component_id: pool_id.clone().into_bytes(),
                            delta: deploy_event.reserve1.to_signed_bytes_be(),
                        });
                    }
                }
            }
            
            // Try to decode the Swap event
            if let Some(swap_event) = crate::abi::eulerswap::events::Swap::match_and_decode(log.log) {
                // Format the pool ID consistently
                let pool_id = format_pool_id(log.address());
                
                // Check if the log emitter is a known pool
                if store.get_last(pool_key(&pool_id)).is_some() {
                    // Get token addresses from the store
                    if let Some(asset0) = store.get_last(pool_asset_key(&pool_id, true)) {
                        if let Some(asset1) = store.get_last(pool_asset_key(&pool_id, false)) {
                            let asset0_bytes = decode_address(&asset0);
                            let asset1_bytes = decode_address(&asset1);
                            
                            // Add amount0In as a positive delta if > 0
                            if swap_event.amount0_in > substreams::scalar::BigInt::from(0) {
                                deltas.push(BalanceDelta {
                                    ord: log.ordinal(),
                                    tx: Some(log.receipt.transaction.into()),
                                    token: asset0_bytes.clone(),
                                    component_id: pool_id.clone().into_bytes(),
                                    delta: swap_event.amount0_in.to_signed_bytes_be(),
                                });
                            }
                            
                            // Add amount1In as a positive delta if > 0
                            if swap_event.amount1_in > substreams::scalar::BigInt::from(0) {
                                deltas.push(BalanceDelta {
                                    ord: log.ordinal(),
                                    tx: Some(log.receipt.transaction.into()),
                                    token: asset1_bytes.clone(),
                                    component_id: pool_id.clone().into_bytes(),
                                    delta: swap_event.amount1_in.to_signed_bytes_be(),
                                });
                            }
                            
                            // Add amount0Out as a negative delta if > 0
                            if swap_event.amount0_out > substreams::scalar::BigInt::from(0) {
                                deltas.push(BalanceDelta {
                                    ord: log.ordinal(),
                                    tx: Some(log.receipt.transaction.into()),
                                    token: asset0_bytes.clone(),
                                    component_id: pool_id.clone().into_bytes(),
                                    delta: swap_event.amount0_out.neg().to_signed_bytes_be(),
                                });
                            }
                            
                            // Add amount1Out as a negative delta if > 0
                            if swap_event.amount1_out > substreams::scalar::BigInt::from(0) {
                                deltas.push(BalanceDelta {
                                    ord: log.ordinal(),
                                    tx: Some(log.receipt.transaction.into()),
                                    token: asset1_bytes.clone(),
                                    component_id: pool_id.clone().into_bytes(),
                                    delta: swap_event.amount1_out.neg().to_signed_bytes_be(),
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
#[substreams::handlers::store]
pub fn store_balances(deltas: BlockBalanceDeltas, store: StoreAddBigInt) {
    tycho_substreams::balances::store_balance_changes(deltas, store);
}

/// Aggregates protocol components and balance changes by transaction.
///
/// This is the main method that will aggregate all changes as well as extract all
/// relevant contract storage deltas.
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

    // Default attributes to add to all pool components
    let default_attributes = vec![
        Attribute {
            name: "balance_owner".to_string(),
            // Use the pool address as the balance owner
            value: vec![], // We'll fill this in for each component
            change: ChangeType::Creation.into(),
        },
        Attribute {
            name: "update_marker".to_string(),
            value: vec![1u8],
            change: ChangeType::Creation.into(),
        },
    ];

    // Aggregate newly created components per tx
    new_components
    .tx_components
    .iter()
    .for_each(|tx_component| {
        // Initialize builder if not yet present for this tx
        let tx = tx_component.tx.as_ref().unwrap();
        let builder = transaction_changes
            .entry(tx.index)
            .or_insert_with(|| TransactionChangesBuilder::new(tx));

        // Iterate over individual components created within this tx
        tx_component
            .components
            .iter()
            .for_each(|component| {
                // Add the component to the builder
                builder.add_protocol_component(component);
                
                // Create attributes with the correct balance owner
                let mut component_attributes = default_attributes.clone();
                // Set the balance owner to the pool address
                component_attributes[0].value = decode_address(&component.id);
                
                // Add entity changes with the attributes
                builder.add_entity_change(&EntityChanges {
                    component_id: component.id.clone(),
                    attributes: component_attributes,
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
            let addr_str = store_address(addr);
            
            // Check if this address belongs to a known pool using consistent formatting
            let is_pool = components_store
                .get_last(pool_key(&addr_str))
                .is_some();
            
            // Check if address is any known token
            // let is_token: bool = components_store
            //     .get_last(token_key(&addr_str))
            //     .is_some();
                
            // Check if address is any known vault
            let is_vault = components_store
                .get_last(vault_key(&addr_str))
                .is_some();
                
            // Check if this address is one of the known fixed addresses
            let is_known_fixed_address = addr.eq(EVC_ADDRESS) ||
                addr.eq(EULERSWAP_PERIPHERY) ||
                addr.eq(EVK_EVAULT_IMPL) ||
                addr.eq(EVK_VAULT_MODULE_IMPL) ||
                addr.eq(EVK_BORROWING_MODULE_IMPL) ||
                addr.eq(EVK_GOVERNANCE_MODULE_IMPL) ||
                addr.eq(EVK_GENERIC_FACTORY);
            
            is_pool || is_vault || is_known_fixed_address
        },
        &mut transaction_changes,
    );

    // Track ERC20 Transfer events for vault balances
    // Process all Transfer events in the block related to vaults
    block
        .logs()
        .for_each(|log| {
            // Look for Transfer events from ERC20 tokens
            if let Some(transfer) = erc20::events::Transfer::match_and_decode(log.log) {
                let from_str = store_address(&transfer.from);
                let to_str = store_address(&transfer.to);
                let token_address = log.address().to_vec();
                
                // Check if sender is a known vault
                if components_store.get_last(vault_key(&from_str)).is_some() {
                    // This is a transfer out of a vault
                    let vault_address = decode_address(&from_str);
                    let mut vault_change = InterimContractChange::new(&vault_address, true);
                    
                    // Add token balance delta (negative since tokens are leaving)
                    vault_change.upsert_token_balance(
                        &token_address, 
                        &transfer.value.neg().to_signed_bytes_be()
                    );
                    
                    // Add the contract change to the builder
                    let tycho_tx: Transaction = Transaction::from(log.receipt.transaction);
                    let builder = transaction_changes
                        .entry(tycho_tx.index.into())
                        .or_insert_with(|| TransactionChangesBuilder::new(&tycho_tx));
                        
                    builder.add_contract_changes(&vault_change);
                }
                
                // Check if receiver is a known vault
                if components_store.get_last(vault_key(&to_str)).is_some() {
                    // This is a transfer into a vault
                    let vault_address = decode_address(&to_str);
                    let mut vault_change = InterimContractChange::new(&vault_address, true);
                    
                    // Add token balance delta (positive since tokens are entering)
                    vault_change.upsert_token_balance(
                        &token_address,
                        &transfer.value.to_signed_bytes_be()
                    );
                    
                    let tycho_tx: Transaction = Transaction::from(log.receipt.transaction);
                    let builder = transaction_changes
                        .entry(tycho_tx.index.into())
                        .or_insert_with(|| TransactionChangesBuilder::new(&tycho_tx));

                    builder.add_contract_changes(&vault_change);
                }
            }
        });

    // Update components that had contract changes
    transaction_changes
        .iter_mut()
        .for_each(|(_, change)| {
            // This indirection is necessary due to borrowing rules
            let addresses = change
                .changed_contracts()
                .map(|e| e.to_vec())
                .collect::<Vec<_>>();
                
            addresses
                .into_iter()
                .for_each(|address: Vec<u8>| {
                    let address_str = store_address(&address);
                    if !components_store.get_last(vault_key(&address_str)).is_some() {
                        // We reconstruct the component_id from the address here
                        let pool_id = format_pool_id(&address);
                        if let Some(component_id) = components_store.get_last(pool_key(&pool_id)) {
                            change.mark_component_as_updated(&component_id);
                        }
                    }
                });
        });

    // Process all `transaction_changes` for final output in the `BlockChanges`,
    // sorted by transaction index (the key).
    Ok(BlockChanges {
        block: Some((&block).into()),
        changes: transaction_changes
            .drain()
            .sorted_unstable_by_key(|(index, _)| *index)
            .filter_map(|(_, builder)| builder.build())
            .collect::<Vec<_>>(),
    })
}
