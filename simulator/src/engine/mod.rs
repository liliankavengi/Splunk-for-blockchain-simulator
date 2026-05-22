use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

use tokio::sync::Mutex;

use crate::synthesizer::hashes::{deterministic_block_hash, FixedHash32};

pub mod tps_ramp;
pub mod reorg;
pub mod scenario;

// ─── BlockRegistry ────────────────────────────────────────────────────────────

/// Ring-buffer of recent (block_number, block_hash) pairs — used by ReorgTrigger.
pub struct BlockRegistry {
    blocks: VecDeque<(u64, FixedHash32)>,
    capacity: usize,
}

impl BlockRegistry {
    pub fn new(capacity: usize) -> Self {
        let capacity = capacity.max(10);
        Self {
            blocks: VecDeque::with_capacity(capacity),
            capacity,
        }
    }

    pub fn push(&mut self, block_number: u64, hash: FixedHash32) {
        if self.blocks.len() >= self.capacity {
            self.blocks.pop_front();
        }
        self.blocks.push_back((block_number, hash));
    }

    /// Returns the most recent block hash, or a genesis placeholder.
    pub fn current_hash(&self) -> FixedHash32 {
        self.blocks
            .back()
            .map(|(_, h)| h.clone())
            .unwrap_or_else(|| deterministic_block_hash(0, 0))
    }

    /// Returns the last `n` blocks (oldest first), or fewer if unavailable.
    pub fn last_n(&self, n: usize) -> Vec<(u64, FixedHash32)> {
        let skip = self.blocks.len().saturating_sub(n);
        self.blocks.iter().skip(skip).cloned().collect()
    }
}

// ─── EngineState ─────────────────────────────────────────────────────────────

pub struct EngineState {
    pub current_block: AtomicU64,
    /// Counts only successfully published logs. Used for summaries and dashboards.
    pub logs_produced: Arc<AtomicU64>,
    /// Monotonically increasing sequence used to assign log_index in LogBuilder.
    /// Incremented at build-time (before publish), so may exceed logs_produced on failures.
    pub log_sequence: Arc<AtomicU64>,
    pub current_tps: AtomicU64,
    pub start_time: Instant,
    pub block_registry: Arc<Mutex<BlockRegistry>>,
}

impl EngineState {
    pub fn new(initial_block: u64, registry_capacity: usize) -> Arc<Self> {
        let registry = Arc::new(Mutex::new(BlockRegistry::new(registry_capacity)));
        Arc::new(Self {
            current_block: AtomicU64::new(initial_block),
            logs_produced: Arc::new(AtomicU64::new(0)),
            log_sequence: Arc::new(AtomicU64::new(0)),
            current_tps: AtomicU64::new(0),
            start_time: Instant::now(),
            block_registry: registry,
        })
    }

    pub fn elapsed_secs(&self) -> f64 {
        self.start_time.elapsed().as_secs_f64()
    }

    pub async fn current_block_hash(&self) -> FixedHash32 {
        self.block_registry.lock().await.current_hash()
    }

    pub async fn advance_block(&self) -> (u64, FixedHash32) {
        let block_num = self.current_block.fetch_add(1, Ordering::SeqCst) + 1;
        let hash = deterministic_block_hash(block_num, self.logs_produced.load(Ordering::Relaxed));
        self.block_registry.lock().await.push(block_num, hash.clone());
        (block_num, hash)
    }
}
