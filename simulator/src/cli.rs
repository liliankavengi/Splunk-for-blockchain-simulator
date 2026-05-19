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

    /// Kafka topic to publish logs to
    #[arg(long, default_value = "blockchain-logs-sim")]
    pub kafka_topic: String,

    /// Admin dashboard HTTP port
    #[arg(long, default_value_t = 8080)]
    pub dashboard_port: u16,

    /// Admin dashboard Bearer token password
    #[arg(long, env = "DASHBOARD_PASSWORD", default_value = "secret")]
    pub dashboard_password: String,

    /// ClickHouse URL (used for post-run assertion hints)
    #[arg(long, default_value = "http://localhost:8123")]
    pub clickhouse_url: String,
}
