use std::sync::Arc;

use crate::kafka::any_producer::AnyProducer;
use crate::synthesizer::log_builder::BlockchainLog;

/// Publishes a batch of reorg-stamped logs via whichever backend is active.
/// Logs must already have `is_reorg = true` and `original_block_hash` set.
pub async fn publish_reorg_batch(
    producer: &Arc<AnyProducer>,
    logs: Vec<BlockchainLog>,
) -> anyhow::Result<usize> {
    let count = logs.len();
    producer.publish_batch(&logs).await?;
    tracing::info!(count, "Reorg batch published");
    Ok(count)
}
