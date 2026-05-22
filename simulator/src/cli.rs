use std::path::PathBuf;
use clap::Parser;

#[derive(Parser, Debug)]
#[command(
    name = "simulator",
    about = "High-speed blockchain log simulator for stress testing ingestion pipelines",
    version
)]
pub struct Cli {
    /// Path to scenario JSON/YAML blueprint
    #[arg(long, short = 's')]
    pub scenario: PathBuf,

    /// Kafka broker list (comma-separated)
    #[arg(long, default_value = "localhost:9092")]
    pub brokers: String,

    /// Kafka topic to publish logs to (overridden by ACTIVE_INGEST_TOPIC env var)
    #[arg(long, env = "ACTIVE_INGEST_TOPIC", default_value = "blockchain-logs-sim")]
    pub kafka_topic: String,

    /// Admin dashboard HTTP port (Render sets $PORT automatically)
    #[arg(long, env = "PORT", default_value_t = 8080)]
    pub dashboard_port: u16,

    /// Admin dashboard Bearer token password
    #[arg(long, env = "DASHBOARD_PASSWORD", default_value = "secret")]
    pub dashboard_password: String,

    /// ClickHouse URL (used for post-run assertion hints)
    #[arg(long, default_value = "http://localhost:8123")]
    pub clickhouse_url: String,

    /// Write logs as NDJSON to this file instead of Kafka ("-" for stdout).
    /// When set, Kafka is not used even if the kafka feature is compiled in.
    #[arg(long, env = "SIM_OUTPUT_FILE")]
    pub output_file: Option<String>,
}
