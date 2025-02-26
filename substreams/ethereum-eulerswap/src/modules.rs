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
    abi::erc20, balances::aggregate_balances_changes, contract::extract_contract_changes_builder,
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
                })
        });
}

/// Extracts balance changes per component
///
/// This template function uses ERC20 transfer events to extract balance changes. It
/// assumes that each component is deployed at a dedicated contract address. If a
/// transfer to the component is detected, it's balanced is increased and if a balance
/// from the component is detected its balance is decreased.
///
/// ## Note:
/// Changes are necessary if your protocol uses native ETH, uses a vault contract or if
/// your component burn or mint tokens without emitting transfer events.
///
/// You may want to ignore LP tokens if your protocol emits transfer events for these
/// here.
#[substreams::handlers::map]
fn map_relative_component_balance(
    block: eth::v2::Block,
    store: StoreGetString,
) -> Result<BlockBalanceDeltas> {
    let res = block
        .logs()
        .filter_map(|log| {
            erc20::events::Transfer::match_and_decode(log).map(|transfer| {
                let to_addr_hex = hex::encode(transfer.to.as_slice());
                let from_addr_hex = hex::encode(transfer.from.as_slice());
                let tx = log.receipt.transaction;
                
                // Check if receiver is a known pool
                if let Some(_) = store.get_last(format!("pool:0x{}", to_addr_hex)) {
                    return Some(BalanceDelta {
                        ord: log.ordinal(),
                        tx: Some(tx.into()),
                        token: log.address().to_vec(),
                        delta: transfer.value.to_signed_bytes_be(),
                        component_id: format!("0x{}", to_addr_hex).into_bytes(),
                    });
                } 
                // Check if sender is a known pool
                else if let Some(_) = store.get_last(format!("pool:0x{}", from_addr_hex)) {
                    return Some(BalanceDelta {
                        ord: log.ordinal(),
                        tx: Some(tx.into()),
                        token: log.address().to_vec(),
                        delta: (transfer.value.neg()).to_signed_bytes_be(),
                        component_id: format!("0x{}", from_addr_hex).into_bytes(),
                    });
                }
                None
            })
        })
        .flatten()
        .collect::<Vec<_>>();

    Ok(BlockBalanceDeltas { balance_deltas: res })
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
