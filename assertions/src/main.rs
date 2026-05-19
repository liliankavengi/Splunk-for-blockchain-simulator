use clap::Parser;

mod checks;
mod cli;
mod clickhouse_client;
mod report;

use checks::{
    alert_latency::check_alert_latency,
    compression_check::check_compression,
    log_completeness::check_log_completeness,
    reorg_handling::check_reorg_handling,
};
use cli::AssertionCli;
use clickhouse_client::AssertionClient;
use report::{print_report, AssertionReport};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = AssertionCli::parse();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("assertions=info,warn")),
        )
        .compact()
        .init();

    let sim_start = chrono::DateTime::parse_from_rfc3339(&cli.sim_start)
        .map_err(|e| anyhow::anyhow!("Invalid --sim-start timestamp: {}", e))?
        .with_timezone(&chrono::Utc);

    tracing::info!(
        scenario = %cli.scenario_name,
        clickhouse = %cli.clickhouse_url,
        database = %cli.database,
        "Running assertion suite"
    );

    let client = AssertionClient::new(&cli.clickhouse_url, &cli.database);

    // Run all 4 checks in parallel
    let (latency, completeness, reorg, compression) = tokio::join!(
        check_alert_latency(&client, &cli.scenario_name, sim_start, cli.alert_latency_threshold),
        check_log_completeness(&client, &cli.scenario_name, cli.expected_logs, 0.001),
        check_reorg_handling(&client, &cli.scenario_name, cli.expected_reorg_logs),
        check_compression(&client, cli.min_compression_ratio),
    );

    let report = AssertionReport {
        scenario_name: cli.scenario_name.clone(),
        latency: latency?,
        completeness: completeness?,
        reorg: reorg?,
        compression: compression?,
    };

    let all_passed = print_report(&report);

    std::process::exit(if all_passed { 0 } else { 1 });
}
