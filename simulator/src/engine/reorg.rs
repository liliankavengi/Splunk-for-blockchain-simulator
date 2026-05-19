use std::sync::Arc;

use sha2::{Digest, Sha256};

use crate::synthesizer::hashes::FixedHash32;
use crate::synthesizer::log_builder::BlockchainLog;
use crate::engine::EngineState;
use crate::kafka::producer::KafkaProducer;

pub struct ReorgTrigger {
    state: Arc<EngineState>,
    producer: Arc<KafkaProducer>,
}

impl ReorgTrigger {
    pub fn new(state: Arc<EngineState>, producer: Arc<KafkaProducer>) -> Self {
        Self { state, producer }
    }

    /// Rewrites the last `backtrack_blocks` block hashes, re-publishes them
    /// with `is_reorg = true`. Returns the number of blocks reorg'd.
    pub async fn trigger(&self, backtrack_blocks: u64) -> anyhow::Result<u64> {
        let n = backtrack_blocks as usize;
        let entries = self.state.block_registry.lock().await.last_n(n);

        if entries.is_empty() {
            tracing::warn!("ReorgTrigger: no blocks in registry to reorg");
            return Ok(0);
        }

        tracing::info!(
            blocks = entries.len(),
            "ReorgTrigger: rewriting {} block(s)",
            entries.len()
        );

        for (block_num, original_hash) in &entries {
            let forked_hash = Self::fork_hash(original_hash, *block_num);

            // Build a representative reorg marker log for this block
            let reorg_log = BlockchainLog {
                log_id: uuid::Uuid::new_v4().to_string(),
                block_number: *block_num,
                block_hash: forked_hash.clone(),
                tx_hash: crate::synthesizer::hashes::deterministic_tx_hash(*block_num, 0),
                tx_index: 0,
                log_index: 0,
                address: "0x0000000000000000000000000000000000000000".to_string(),
                contract_type: "reorg_marker".to_string(),
                data: "0x".to_string(),
                topics: [
                    forked_hash.clone(),
                    original_hash.clone(),
                    FixedHash32::from_bytes(&[0u8; 32]),
                    FixedHash32::from_bytes(&[0u8; 32]),
                ],
                block_timestamp: chrono::Utc::now(),
                simulated_at: chrono::Utc::now(),
                decoded_params: None,
                is_reorg: true,
                original_block_hash: Some(original_hash.clone()),
                scenario_name: "reorg".to_string(),
            };

            self.producer.publish(&reorg_log).await?;
            tracing::debug!(block_num, old = %original_hash, new = %forked_hash, "Block reorg'd");
        }

        Ok(entries.len() as u64)
    }

    /// Derives a forked block hash by hashing the original hash with the block number.
    fn fork_hash(original: &FixedHash32, block_number: u64) -> FixedHash32 {
        let mut hasher = Sha256::new();
        hasher.update(b"reorg:");
        hasher.update(original.as_str().as_bytes());
        hasher.update(block_number.to_be_bytes());
        let result: [u8; 32] = hasher.finalize().into();
        FixedHash32::from_bytes(&result)
    }
}
