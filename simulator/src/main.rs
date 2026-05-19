use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Duration;

use clap::Parser;

mod cli;
mod config;
mod dashboard;
mod engine;
mod kafka;
mod observability;
mod synthesizer;

use cli::Cli;
use config::{load_scenario, RuntimeConfig};
use dashboard::{metrics::SimMetrics, server::DashboardServer};
use engine::{scenario::ScenarioEngine, EngineState};
use kafka::producer::KafkaProducer;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    observability::tracing_setup::init_tracing();

    let blueprint = load_scenario(&cli.scenario)?;
    tracing::info!(
        scenario = %blueprint.scenario_name,
        base_tps = blueprint.base_tps,
        burst_tps = blueprint.burst_tps,
        duration_secs = blueprint.duration_seconds,
        "Loaded scenario blueprint"
    );

    let runtime_config = RuntimeConfig {
        kafka_brokers: cli.brokers.clone(),
        kafka_topic: cli.kafka_topic.clone(),
        dashboard_port: cli.dashboard_port,
        dashboard_password: cli.dashboard_password.clone(),
        clickhouse_url: cli.clickhouse_url.clone(),
        scenario_path: cli.scenario.clone(),
    };

    // Shared metrics
    let metrics = Arc::new(SimMetrics::new()?);

    // Kafka producer
    let producer = Arc::new(
        KafkaProducer::new(
            &runtime_config.kafka_brokers,
            &runtime_config.kafka_topic,
            Arc::clone(&metrics),
        )
        .await?,
    );

    // Engine state — start at a realistic Ethereum block height
    let reorg_cap = blueprint.reorg_backtrack_blocks.unwrap_or(10) * 2;
    let state = EngineState::new(19_500_000u64, reorg_cap as usize);

    // Seed the block registry with the initial block
    state
        .block_registry
        .lock()
        .await
        .push(
            19_500_000u64,
            crate::synthesizer::hashes::deterministic_block_hash(19_500_000, 0),
        );

    // Dashboard server
    let dashboard = DashboardServer::new(
        Arc::clone(&metrics),
        Arc::clone(&state),
        blueprint.scenario_name.clone(),
        runtime_config.dashboard_password.clone(),
        runtime_config.dashboard_port,
    );
    let _dashboard_handle = dashboard.serve().await?;
    tracing::info!(port = runtime_config.dashboard_port, "Dashboard ready");

    // Background metrics updater (every 500ms)
    {
        let metrics_ref = Arc::clone(&metrics);
        let state_ref = Arc::clone(&state);
        let producer_ref = Arc::clone(&producer);
        tokio::spawn(async move {
            let mut last_count = 0u64;
            let mut last_tick = std::time::Instant::now();
            loop {
                tokio::time::sleep(Duration::from_millis(500)).await;
                let now = std::time::Instant::now();
                let current = state_ref.logs_produced.load(Ordering::Relaxed);
                let delta = current.saturating_sub(last_count);
                let elapsed = now.duration_since(last_tick).as_secs_f64();
                let tps = if elapsed > 0.0 { delta as f64 / elapsed } else { 0.0 };

                metrics_ref.logs_per_second.set(tps);
                metrics_ref
                    .kafka_buffer_lag
                    .set(producer_ref.in_flight_count() as f64);

                last_count = current;
                last_tick = now;
            }
        });
    }

    // Run simulation
    let engine = ScenarioEngine::new(
        blueprint,
        runtime_config,
        Arc::clone(&producer),
        Arc::clone(&metrics),
        Arc::clone(&state),
    );

    let summary = engine.run().await?;

    println!();
    println!("╔══════════════════════════════════════════════════╗");
    println!("║          SIMULATION COMPLETE                     ║");
    println!("╠══════════════════════════════════════════════════╣");
    println!("║  Scenario:        {:<31}║", summary.scenario_name);
    println!("║  Total Logs:      {:<31}║", summary.total_logs);
    println!("║  Duration:        {:<31}║", format!("{:.1}s", summary.duration_secs));
    println!("║  Peak TPS:        {:<31}║", summary.peak_tps);
    println!("║  Reorg Triggered: {:<31}║", summary.reorg_triggered);
    println!("╚══════════════════════════════════════════════════╝");
    println!();
    println!("Run the assertions binary to verify ClickHouse ingestion:");
    println!(
        "  ./target/release/assertions \\\n    --clickhouse-url http://localhost:8123 \\\n    --database blockchain_sim \\\n    --scenario-name \"{}\" \\\n    --expected-logs {}",
        summary.scenario_name, summary.total_logs
    );

    Ok(())
}
