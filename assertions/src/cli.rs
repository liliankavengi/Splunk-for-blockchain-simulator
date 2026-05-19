use clap::Parser;

#[derive(Parser, Debug)]
#[command(
    name = "assertions",
    about = "Post-simulation validation suite — queries ClickHouse and verifies ingestion correctness",
    version
)]
pub struct AssertionCli {
    /// ClickHouse HTTP URL
    #[arg(long, default_value = "http://localhost:8123")]
    pub clickhouse_url: String,

    /// ClickHouse database name
    #[arg(long, default_value = "blockchain_sim")]
    pub database: String,

    /// Scenario name to validate (must match the one used during simulation)
    #[arg(long)]
    pub scenario_name: String,

    /// Expected total log count from the simulation run summary
    #[arg(long)]
    pub expected_logs: u64,

    /// Expected count of reorg-flagged logs (0 if no reorg was triggered)
    #[arg(long, default_value_t = 0)]
    pub expected_reorg_logs: u64,

    /// RFC3339 timestamp of when the simulation started
    #[arg(long, default_value = "1970-01-01T00:00:00Z")]
    pub sim_start: String,

    /// Minimum fraction of alerts that must have fired within 1 second (0.0–1.0)
    #[arg(long, default_value_t = 0.95)]
    pub alert_latency_threshold: f64,

    /// Minimum acceptable compression ratio (default 3.0 = 3:1)
    #[arg(long, default_value_t = 3.0)]
    pub min_compression_ratio: f64,
}
