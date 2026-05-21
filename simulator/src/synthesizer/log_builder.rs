use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use chrono::{DateTime, Utc};
use rand::Rng;
use rand::SeedableRng;
use rand::rngs::StdRng;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::config::{Complexity, ContractType, ScenarioBlueprint};
use crate::synthesizer::defi_params::{
    gen_amm_swap, gen_bridge_transfer, gen_flash_loan, gen_nft_sale, DecodedParams,
};
use crate::synthesizer::evm::generate_evm_data;
use crate::synthesizer::hashes::{
    deterministic_tx_hash, event_signature_topic, random_address, random_indexed_topic,
    FixedHash32,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockchainLog {
    pub log_id: String,
    pub block_number: u64,
    pub block_hash: FixedHash32,
    pub tx_hash: FixedHash32,
    pub tx_index: u32,
    pub log_index: u32,
    pub address: String,
    pub contract_type: String,
    pub data: String,
    pub topics: [FixedHash32; 4],
    pub block_timestamp: DateTime<Utc>,
    pub simulated_at: DateTime<Utc>,
    pub decoded_params: Option<DecodedParams>,
    /// Flat Map(String, String) projection of decoded_params for ClickHouse Schema-on-Read.
    /// Handles deeply nested / recursive JSON without requiring a fixed schema.
    pub contract_args: HashMap<String, String>,
    pub is_reorg: bool,
    pub original_block_hash: Option<FixedHash32>,
    pub scenario_name: String,
}

pub struct LogBuilder {
    rng: StdRng,
    log_counter: Arc<AtomicU64>,
}

impl LogBuilder {
    pub fn new(seed: u64, log_counter: Arc<AtomicU64>) -> Self {
        Self {
            rng: StdRng::seed_from_u64(seed),
            log_counter,
        }
    }

    /// Builds one complete `BlockchainLog` for the given block context.
    pub fn build(
        &mut self,
        block_number: u64,
        block_hash: &FixedHash32,
        block_timestamp: DateTime<Utc>,
        blueprint: &ScenarioBlueprint,
    ) -> BlockchainLog {
        let log_index = self.log_counter.fetch_add(1, Ordering::Relaxed) as u32;
        let tx_index = (log_index / 3).wrapping_add(1); // ~3 logs per tx

        let evm_bytes = generate_evm_data(
            &blueprint.target_contract_type,
            &blueprint.nested_fields_complexity,
            &mut self.rng,
        );

        let variant: u8 = self.rng.gen_range(0u8..4);
        let topics = [
            event_signature_topic(&blueprint.target_contract_type, variant),
            random_indexed_topic(&mut self.rng),
            random_indexed_topic(&mut self.rng),
            random_indexed_topic(&mut self.rng),
        ];

        let decoded_params = if blueprint.nested_fields_complexity == Complexity::High {
            Some(self.build_decoded_params(&blueprint.target_contract_type))
        } else {
            None
        };

        let contract_args = flatten_params(decoded_params.as_ref());

        BlockchainLog {
            log_id: Uuid::new_v4().to_string(),
            block_number,
            block_hash: block_hash.clone(),
            tx_hash: deterministic_tx_hash(block_number, tx_index),
            tx_index,
            log_index,
            address: random_address(&mut self.rng),
            contract_type: blueprint.target_contract_type.to_string(),
            data: format!("0x{}", hex::encode(&evm_bytes)),
            topics,
            block_timestamp,
            simulated_at: Utc::now(),
            decoded_params,
            contract_args,
            is_reorg: false,
            original_block_hash: None,
            scenario_name: blueprint.scenario_name.clone(),
        }
    }

    fn build_decoded_params(&mut self, contract_type: &ContractType) -> DecodedParams {
        match contract_type {
            ContractType::DeFiLendingPool => DecodedParams::FlashLoan(gen_flash_loan(&mut self.rng)),
            ContractType::AmmSwapPool => DecodedParams::AmmSwap(gen_amm_swap(&mut self.rng)),
            ContractType::NftMarketplace => DecodedParams::NftSale(gen_nft_sale(&mut self.rng)),
            ContractType::Bridge | ContractType::Governance => {
                DecodedParams::BridgeTransfer(gen_bridge_transfer(&mut self.rng))
            }
        }
    }

    /// Re-stamps a log as a reorg clone: new block hash, `is_reorg = true`.
    pub fn reorg_clone(original: &BlockchainLog, new_block_hash: FixedHash32) -> BlockchainLog {
        BlockchainLog {
            log_id: Uuid::new_v4().to_string(),
            block_hash: new_block_hash.clone(),
            is_reorg: true,
            original_block_hash: Some(original.block_hash.clone()),
            simulated_at: Utc::now(),
            ..original.clone()
        }
    }
}

/// Flattens `DecodedParams` into a `Map(String, String)` compatible with ClickHouse.
/// Top-level scalar fields are emitted directly; nested objects are dot-prefixed
/// (e.g. `protocol_fees.token0`). Arrays are serialised as compact JSON strings
/// so the map remains flat regardless of nesting depth.
///
/// Uses a text round-trip (to_string → from_str) to avoid serde_json's
/// `to_value` failure on u128 values that exceed u64::MAX.
fn flatten_params(params: Option<&DecodedParams>) -> HashMap<String, String> {
    let Some(p) = params else {
        return HashMap::new();
    };
    let json_str = match serde_json::to_string(p) {
        Ok(s) => s,
        Err(_) => return HashMap::new(),
    };
    let value: serde_json::Value = match serde_json::from_str(&json_str) {
        Ok(v) => v,
        Err(_) => return HashMap::new(),
    };
    let mut map = HashMap::new();
    flatten_json("", &value, &mut map);
    map
}

fn flatten_json(prefix: &str, value: &serde_json::Value, out: &mut HashMap<String, String>) {
    match value {
        serde_json::Value::Object(obj) => {
            for (k, v) in obj {
                let key = if prefix.is_empty() {
                    k.clone()
                } else {
                    format!("{}.{}", prefix, k)
                };
                flatten_json(&key, v, out);
            }
        }
        serde_json::Value::Array(_) => {
            // Arrays (swap_path, collateral_assets, merkle_proof, traits) are
            // stored as compact JSON strings to keep the map flat.
            out.insert(prefix.to_string(), value.to_string());
        }
        serde_json::Value::Null => {}
        other => {
            out.insert(prefix.to_string(), other.to_string().trim_matches('"').to_string());
        }
    }
}
