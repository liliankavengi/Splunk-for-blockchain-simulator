use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use chrono::Utc;
use tokio::task::JoinHandle;

use crate::config::{RuntimeConfig, ScenarioBlueprint};
use crate::dashboard::metrics::SimMetrics;
use crate::engine::{tps_ramp, EngineState};
use crate::engine::reorg::ReorgTrigger;
use crate::kafka::any_producer::AnyProducer;
use crate::synthesizer::log_builder::LogBuilder;

pub struct ScenarioEngine {
    blueprint: Arc<ScenarioBlueprint>,
    producer: Arc<AnyProducer>,
    metrics: Arc<SimMetrics>,
    state: Arc<EngineState>,
}

pub struct RunSummary {
    pub scenario_name: String,
    pub total_logs: u64,
    pub duration_secs: f64,
    pub peak_tps: u64,
    pub reorg_triggered: bool,
}

impl ScenarioEngine {
    pub fn new(
        blueprint: ScenarioBlueprint,
        _config: RuntimeConfig,
        producer: Arc<AnyProducer>,
        metrics: Arc<SimMetrics>,
        state: Arc<EngineState>,
    ) -> Self {
        Self {
            blueprint: Arc::new(blueprint),
            producer,
            metrics,
            state,
        }
    }

    /// Runs the full simulation and returns a summary.
    pub async fn run(&self) -> anyhow::Result<RunSummary> {
        let duration = Duration::from_secs(self.blueprint.duration_seconds);
        let n_workers = tps_ramp::worker_count(self.blueprint.burst_tps);
        let deadline = Instant::now() + duration;

        tracing::info!(
            scenario = %self.blueprint.scenario_name,
            workers = n_workers,
            base_tps = self.blueprint.base_tps,
            burst_tps = self.blueprint.burst_tps,
            duration_secs = self.blueprint.duration_seconds,
            "Simulation starting"
        );

        // Spawn block-advancement task (~12s Ethereum block time)
        let state_for_blocks = Arc::clone(&self.state);
        let block_task: JoinHandle<()> = tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(12)).await;
                let (num, hash) = state_for_blocks.advance_block().await;
                tracing::debug!(block_number = num, hash = %hash, "New block");
            }
        });

        // Spawn worker tasks
        let mut worker_handles: Vec<JoinHandle<u64>> = Vec::with_capacity(n_workers);
        for worker_id in 0..n_workers {
            let blueprint = Arc::clone(&self.blueprint);
            let producer = Arc::clone(&self.producer);
            let state = Arc::clone(&self.state);
            let metrics = Arc::clone(&self.metrics);

            let handle = tokio::spawn(async move {
                worker_loop(worker_id, n_workers, blueprint, producer, state, metrics, deadline).await
            });
            worker_handles.push(handle);
        }

        // Collect results
        for handle in worker_handles {
            match handle.await {
                Ok(_) => {}
                Err(e) => tracing::error!("Worker panicked: {}", e),
            }
        }

        let peak_tps = self.state.current_tps.load(Ordering::Relaxed);
        block_task.abort();

        let actual_total = self.state.logs_produced.load(Ordering::Relaxed);
        let elapsed = self.state.elapsed_secs();

        // Trigger reorg if configured
        let reorg_triggered = if let Some(n) = self.blueprint.reorg_backtrack_blocks {
            let trigger = ReorgTrigger::new(Arc::clone(&self.state), Arc::clone(&self.producer));
            match trigger.trigger(n).await {
                Ok(count) => {
                    tracing::info!(blocks_reorged = count, "Reorg simulation complete");
                    true
                }
                Err(e) => {
                    tracing::error!(error = %e, "Reorg failed");
                    false
                }
            }
        } else {
            false
        };

        Ok(RunSummary {
            scenario_name: self.blueprint.scenario_name.clone(),
            total_logs: actual_total,
            duration_secs: elapsed,
            peak_tps,
            reorg_triggered,
        })
    }
}

async fn worker_loop(
    worker_id: usize,
    n_workers: usize,
    blueprint: Arc<ScenarioBlueprint>,
    producer: Arc<AnyProducer>,
    state: Arc<EngineState>,
    metrics: Arc<SimMetrics>,
    deadline: Instant,
) -> u64 {
    let seed = worker_id as u64 * 31337 + blueprint.base_tps;
    let log_counter = Arc::clone(&state.logs_produced);
    let mut builder = LogBuilder::new(seed, log_counter);
    let mut logs_this_worker = 0u64;
    let mut peak_tps = 0u64;

    loop {
        if Instant::now() >= deadline {
            break;
        }

        let elapsed = state.elapsed_secs();
        let total_tps = tps_ramp::compute_target_tps(&blueprint, elapsed);
        let worker_tps = (total_tps / n_workers as u64).max(1);
        let interval = tps_ramp::tps_to_interval(worker_tps);

        state.current_tps.fetch_max(total_tps, Ordering::Relaxed);
        if total_tps > peak_tps {
            peak_tps = total_tps;
        }

        let block_number = state.current_block.load(Ordering::Relaxed);
        let block_hash = state.current_block_hash().await;
        let block_ts = Utc::now();

        let log = builder.build(block_number, &block_hash, block_ts, &blueprint);

        // Backpressure: throttle if Kafka queue is > 80% full
        if producer.queue_utilization() > 0.80 {
            tokio::time::sleep(Duration::from_millis(10)).await;
            continue;
        }

        match producer.publish(&log).await {
            Ok(()) => {
                logs_this_worker += 1;
                metrics.total_logs_produced.inc();
            }
            Err(e) => {
                tracing::warn!(worker_id, error = %e, "Publish failed — skipping log");
            }
        }

        tokio::time::sleep(interval).await;
    }

    tracing::info!(worker_id, logs = logs_this_worker, "Worker finished");
    logs_this_worker
}
