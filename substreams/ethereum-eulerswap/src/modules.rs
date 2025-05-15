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
use substreams::{hex, pb::substreams::StoreDeltas, prelude::*};
use substreams_ethereum::{
    pb::eth::{self, v2::StorageChange},
    Event,
};
use tycho_substreams::{
    balances::aggregate_balances_changes, contract::extract_contract_changes_builder, prelude::*,
};

pub const EVC_ADDRESS: &[u8] = &hex!("0C9a3dd6b8F28529d72d7f9cE918D493519EE383");
pub const EULERSWAP_PERIPHERY: &[u8] = &hex!("4fE0547e7Be0e9a9cED3aC948B83146996f899aE");
pub const EULERSWAP_IMPLEMENTATION: &[u8] = &hex!("270e7d14f304c0df91e50996072525b24978e17f");
pub const EVK_EVAULT_IMPL: &[u8] = &hex!("8ff1c814719096b61abf00bb46ead0c9a529dd7d");
pub const EVK_VAULT_MODULE_IMPL: &[u8] = &hex!("b4ad4d9c02c01b01cf586c16f01c58c73c7f0188");
pub const EVK_BORROWING_MODULE_IMPL: &[u8] = &hex!("639156f8feb0cd88205e4861a0224ec169605acf");
pub const EVK_GOVERNANCE_MODULE_IMPL: &[u8] = &hex!("a61f5016f2cd5cec12d091f871fce1e1df5f0b67");
pub const EVK_GENERIC_FACTORY: &[u8] = &hex!("29a56a1b8214d9cf7c5561811750d5cbdb45cc8e");
pub const PERMIT_2: &[u8] = &hex!("000000000022D473030F116dDEE9F6B43aC78BA3");
// Store key prefixes and suffixes for consistency
const POOL_PREFIX: &str = "pool:";
const TOKEN_PREFIX: &str = "token:";
const VAULT_PREFIX: &str = "vault:";
const ASSET0_SUFFIX: &str = ":asset0";
const ASSET1_SUFFIX: &str = ":asset1";
const VAULT0_SUFFIX: &str = ":vault0";
const VAULT1_SUFFIX: &str = ":vault1";
const ASSET_SUFFIX: &str = ":asset";

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

/// Format a store key for a pool's vault
fn vault_asset_key(vault_id: &str) -> String {
    format!("{}{}{}", VAULT_PREFIX, vault_id, ASSET_SUFFIX)
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
                    store.set(0, pool_asset_key(pool_id, true), token0_addr);

                    // Add reverse index for token lookup
                    store.set(0, token_key(token0_addr), token0_addr);

                    // Store asset1 (token 1) with consistent formatting
                    let token1_addr = &store_address(&pc.tokens[1]);
                    store.set(0, pool_asset_key(pool_id, false), token1_addr);

                    // Add reverse index for token lookup
                    store.set(0, token_key(token1_addr), token1_addr);

                    // Store vault addresses
                    // Store vault0 (contract 1) with consistent formatting
                    let vault0_addr = &store_address(&pc.contracts[1]);
                    store.set(0, pool_vault_key(pool_id, true), vault0_addr);

                    // Add reverse index for vault lookup
                    store.set(0, vault_key(vault0_addr), vault0_addr);

                    // Store vault0 asset
                    store.set(0, vault_asset_key(vault0_addr), token0_addr);

                    // Store vault1 (contract 2) with consistent formatting
                    let vault1_addr = &store_address(&pc.contracts[2]);
                    store.set(0, pool_vault_key(pool_id, false), vault1_addr);

                    // Add reverse index for vault lookup
                    store.set(0, vault_key(vault1_addr), vault1_addr);

                    // Store vault1 asset
                    store.set(0, vault_asset_key(vault1_addr), token1_addr);
                })
        });
}

// Structure to hold the final balance value for a token in a vault
struct VaultBalance {
    ordinal: u64,
    value: Vec<u8>,
}

// Function to extract final balances from EulerSwap vaults by tracking ERC20 storage
fn get_eulerswap_vaults_balances(
    transaction: &eth::v2::TransactionTrace,
    components_store: &StoreGetString,
) -> HashMap<Vec<u8>, HashMap<Vec<u8>, VaultBalance>> {
    // Maps vault address -> (token address -> balance)
    let mut vault_balances: HashMap<Vec<u8>, HashMap<Vec<u8>, VaultBalance>> = HashMap::new();

    // Process all contracts in this transaction and look for vault balance changes
    transaction
        .calls
        .iter()
        .filter(|call| {
            !call.state_reverted
                && (crate::abi::evk_vault::functions::Deposit::match_call(call)
                    || crate::abi::evk_vault::functions::Withdraw::match_call(call)
                    || crate::abi::evk_vault::functions::Borrow::match_call(call)
                    || crate::abi::evk_vault::functions::RepayWithShares::match_call(call))
        })
        .for_each(|call| {
            // Check if this call is directly on a vault that we have in store
            call.storage_changes
                .iter()
                .filter(|sc| {
                    components_store
                        .get_last(vault_key(&store_address(&sc.address)))
                        .is_some()
                })
                .for_each(|sc| {
                    if let Some(asset_address) =
                        components_store.get_last(vault_asset_key(&store_address(&sc.address)))
                    {
                        add_change_if_accounted(
                            &mut vault_balances,
                            sc,
                            &sc.address,
                            &decode_address(&asset_address),
                        );
                    }
                });
        });

    vault_balances
}

fn add_change_if_accounted(
    vault_balances: &mut HashMap<Vec<u8>, HashMap<Vec<u8>, VaultBalance>>,
    change: &StorageChange,
    vault_address: &[u8],
    token_address: &[u8],
) {
    let slot_key = get_storage_key_for_vault_cash();

    // Check if the change is for the first slot of VaultStorage
    // (which contains the cash field among others)
    if change.key == slot_key {
        substreams::log::debug!(
            "Processing call to contract: {} with storage changes for {}",
            store_address(vault_address),
            store_address(&change.address),
        );

        substreams::log::debug!("slot_key {:?}", slot_key);

        substreams::log::debug!("old_value {:?}", &change.old_value);

        // Extract the cash value from the packed slot
        let new_value = &change.new_value;
        substreams::log::debug!("new_value {:?}", new_value);

        // The cash value (Assets type = uint112) is stored after the lastInterestAccumulatorUpdate field
        // lastInterestAccumulatorUpdate is uint48 (6 bytes), so cash starts at bit 48
        // Extract the cash value (uint112 = 14 bytes), starting from byte 12
        //
        // The packed slot contains (starting from least significant bit):
        // - lastInterestAccumulatorUpdate (uint48): 6 bytes
        // - cash (uint112): 14 bytes
        // - remaining fields...
        // We're only interested in the cash field, which is bytes 12-26 of the slot

        let mut cash_value = vec![0u8; 32];
        cash_value[18..].copy_from_slice(&new_value[12..26]);

        // Create a BigInt from bytes vector for logging
        let cash_big_int = substreams::scalar::BigInt::from_unsigned_bytes_be(&cash_value);
        substreams::log::debug!(
            "balance: {} (raw: {})",
            cash_big_int.clone() / substreams::scalar::BigInt::from(1_000_000),
            cash_big_int
        );

        // Store the extracted value
        vault_balances
            .entry(vault_address.to_vec())
            .or_default()
            .entry(token_address.to_vec())
            .and_modify(|v| {
                if v.ordinal < change.ordinal && v.value != cash_value.clone() {
                    v.value = cash_value.clone();
                    v.ordinal = change.ordinal;
                }
            })
            .or_insert(VaultBalance { value: cash_value, ordinal: change.ordinal });
    }
}

/// Compute storage slot for vault's internal 'cash' field
///
/// Based on the provided storage layout:
/// - The vaultStorage field is at slot 2 in the Storage contract
/// - Within vaultStorage struct, the cash field is in the first packed slot
/// - Cash is an Assets type (uint112) at offset 6 bytes (after lastInterestAccumulatorUpdate which is uint48)
///
/// This function returns slot 2 where vaultStorage is stored.
fn get_storage_key_for_vault_cash() -> Vec<u8> {
    // Vault storage is at slot 2 in the Storage contract
    let mut slot_bytes: [u8; 32] = [0u8; 32];
    slot_bytes[31] = 2u8; // Set the last byte to 2

    // Return slot 2 directly (no hashing needed for direct struct fields)
    slot_bytes.to_vec()
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
            if let Some(deploy_event) =
                crate::abi::eulerswap_factory::events::PoolDeployed::match_and_decode(log.log)
            {
                // Format the pool ID consistently
                let pool_id = format_pool_id(&deploy_event.pool);

                // Check if the pool is already in the store
                if store
                    .get_last(pool_key(&pool_id))
                    .is_some()
                {
                    // Get token addresses from the event
                    let asset0_bytes = deploy_event.asset0.clone();
                    let asset1_bytes = deploy_event.asset1.clone();

                                // Find the matching PoolConfig event
                    let pool_config_log = block
                        .logs()
                        .find(|l| {
                            let pc= crate::abi::eulerswap_factory::events::PoolConfig::match_and_decode(l);
                            pc.is_some() && pc.unwrap().pool == deploy_event.pool
                        }).unwrap();

                    if let Some(pool_config) =
                        crate::abi::eulerswap_factory::events::PoolConfig::match_and_decode(pool_config_log)
                    {
                        // Add reserve0 as the initial balance for asset0
                        if pool_config.initial_state.0 > substreams::scalar::BigInt::from(0) {
                            deltas.push(BalanceDelta {
                                ord: log.ordinal(),
                                tx: Some(log.receipt.transaction.into()),
                                token: asset0_bytes.clone(),
                                component_id: pool_id.clone().into_bytes(),
                                delta: pool_config
                                    .initial_state
                                    .0
                                    .to_signed_bytes_be(),
                            });
                        }

                        // Add reserve1 as the initial balance for asset1
                        if pool_config.initial_state.1 > substreams::scalar::BigInt::from(0) {
                            deltas.push(BalanceDelta {
                                ord: log.ordinal(),
                                tx: Some(log.receipt.transaction.into()),
                                token: asset1_bytes.clone(),
                                component_id: pool_id.clone().into_bytes(),
                                delta: pool_config
                                    .initial_state
                                    .1
                                    .to_signed_bytes_be(),
                            });
                        }
                    }
                }
            }

            // Try to decode the Swap event
            if let Some(swap_event) = crate::abi::eulerswap::events::Swap::match_and_decode(log.log)
            {
                // Format the pool ID consistently
                let pool_id = format_pool_id(log.address());

                // Check if the log emitter is a known pool
                if store
                    .get_last(pool_key(&pool_id))
                    .is_some()
                {
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
                                    delta: swap_event
                                        .amount0_in
                                        .to_signed_bytes_be(),
                                });
                            }

                            // Add amount1In as a positive delta if > 0
                            if swap_event.amount1_in > substreams::scalar::BigInt::from(0) {
                                deltas.push(BalanceDelta {
                                    ord: log.ordinal(),
                                    tx: Some(log.receipt.transaction.into()),
                                    token: asset1_bytes.clone(),
                                    component_id: pool_id.clone().into_bytes(),
                                    delta: swap_event
                                        .amount1_in
                                        .to_signed_bytes_be(),
                                });
                            }

                            // Add amount0Out as a negative delta if > 0
                            if swap_event.amount0_out > substreams::scalar::BigInt::from(0) {
                                deltas.push(BalanceDelta {
                                    ord: log.ordinal(),
                                    tx: Some(log.receipt.transaction.into()),
                                    token: asset0_bytes.clone(),
                                    component_id: pool_id.clone().into_bytes(),
                                    delta: swap_event
                                        .amount0_out
                                        .neg()
                                        .to_signed_bytes_be(),
                                });
                            }

                            // Add amount1Out as a negative delta if > 0
                            if swap_event.amount1_out > substreams::scalar::BigInt::from(0) {
                                deltas.push(BalanceDelta {
                                    ord: log.ordinal(),
                                    tx: Some(log.receipt.transaction.into()),
                                    token: asset1_bytes.clone(),
                                    component_id: pool_id.clone().into_bytes(),
                                    delta: swap_event
                                        .amount1_out
                                        .neg()
                                        .to_signed_bytes_be(),
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

/// Converts address bytes into a Vec<u8> containing a leading `0x`.
fn address_to_bytes_with_0x(address: &[u8]) -> Vec<u8> {
    address_to_string_with_0x(address).into_bytes()
}

/// Converts address bytes into a string containing a leading `0x`.
fn address_to_string_with_0x(address: &[u8]) -> String {
    format!("0x{}", hex::encode(address))
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
            name: "update_marker".to_string(),
            value: vec![1u8],
            change: ChangeType::Creation.into(),
        },
        Attribute {
            name: "stateless_contract_addr_0".to_string(),
            value: address_to_bytes_with_0x(EVK_EVAULT_IMPL),
            change: ChangeType::Creation.into(),
        },
        Attribute {
            name: "stateless_contract_addr_1".to_string(),
            value: address_to_bytes_with_0x(EVK_VAULT_MODULE_IMPL),
            change: ChangeType::Creation.into(),
        },
        Attribute {
            name: "stateless_contract_addr_2".to_string(),
            value: address_to_bytes_with_0x(EVK_BORROWING_MODULE_IMPL),
            change: ChangeType::Creation.into(),
        },
        Attribute {
            name: "stateless_contract_addr_3".to_string(),
            value: address_to_bytes_with_0x(EVK_GOVERNANCE_MODULE_IMPL),
            change: ChangeType::Creation.into(),
        },
        Attribute {
            name: "stateless_contract_addr_4".to_string(),
            value: address_to_bytes_with_0x(PERMIT_2),
            change: ChangeType::Creation.into(),
        },
        Attribute {
            name: "stateless_contract_addr_5".to_string(),
            value: address_to_bytes_with_0x(EULERSWAP_IMPLEMENTATION),
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
            let is_known_fixed_address = addr.eq(EVC_ADDRESS)
                || addr.eq(EULERSWAP_PERIPHERY)
                || addr.eq(EULERSWAP_IMPLEMENTATION)
                || addr.eq(EVK_EVAULT_IMPL)
                || addr.eq(EVK_VAULT_MODULE_IMPL)
                || addr.eq(EVK_BORROWING_MODULE_IMPL)
                || addr.eq(EVK_GOVERNANCE_MODULE_IMPL)
                || addr.eq(EVK_GENERIC_FACTORY);

            is_pool || is_vault || is_known_fixed_address
        },
        &mut transaction_changes,
    );

    // Extract token balances for EulerSwap vaults using storage tracking
    block
        .transaction_traces
        .iter()
        .for_each(|tx| {
            let vault_balances = get_eulerswap_vaults_balances(tx, &components_store);

            if !vault_balances.is_empty() {
                substreams::log::debug!(
                    "vault_balances.is_empty() {:?}",
                    vault_balances.is_empty()
                );

                let tycho_tx = Transaction::from(tx);
                let builder = transaction_changes
                    .entry(tycho_tx.index)
                    .or_insert_with(|| TransactionChangesBuilder::new(&tycho_tx));

                // Process each vault's final balances
                for (vault_address, token_balances) in vault_balances {
                    substreams::log::debug!("vault_address {:?}", store_address(&vault_address));

                    let mut vault_contract_change =
                        InterimContractChange::new(&vault_address, false);

                    for (token_addr, balance) in token_balances {
                        substreams::log::debug!("token_addr {:?}", store_address(&token_addr));

                        substreams::log::debug!("balance {:?}", balance.value.as_slice());

                        // Convert to human-readable format
                        let big_int =
                            substreams::scalar::BigInt::from_unsigned_bytes_be(&balance.value);
                        substreams::log::debug!(
                            "balance (human readable): {} (raw: {})",
                            big_int.clone() / substreams::scalar::BigInt::from(1_000_000), // Divided by 10^6 for 6 decimals
                            big_int
                        );

                        vault_contract_change
                            .upsert_token_balance(&token_addr, balance.value.as_slice());
                    }

                    builder.add_contract_changes(&vault_contract_change);
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
                    if components_store
                        .get_last(vault_key(&address_str))
                        .is_none()
                    {
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
