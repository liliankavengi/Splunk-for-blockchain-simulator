use chrono::{DateTime, Utc};
use clickhouse::{Client, Row};
use serde::Deserialize;

pub struct AssertionClient {
    client: Client,
    pub database: String,
}

impl AssertionClient {
    pub fn new(url: &str, database: &str) -> Self {
        let client = Client::default()
            .with_url(url)
            .with_database(database);
        Self {
            client,
            database: database.to_string(),
        }
    }

    /// Fetches all security alerts fired after `after_ts` for the given scenario.
    pub async fn fetch_alerts(
        &self,
        scenario_name: &str,
        after_ts: DateTime<Utc>,
    ) -> anyhow::Result<Vec<AlertRow>> {
        let after_unix = after_ts.timestamp();
        let rows = self
            .client
            .query(
                "SELECT alert_id, scenario_name, \
                 toUnixTimestamp(fired_at) AS fired_at_unix, \
                 toUnixTimestamp(log_timestamp) AS log_ts_unix, \
                 (toUnixTimestamp64Milli(fired_at) - toUnixTimestamp64Milli(log_timestamp)) AS latency_ms, \
                 alert_type \
                 FROM security_alerts \
                 WHERE scenario_name = ? AND toUnixTimestamp(fired_at) > ?",
            )
            .bind(scenario_name)
            .bind(after_unix)
            .fetch_all::<AlertRow>()
            .await
            .map_err(|e| anyhow::anyhow!("fetch_alerts query failed: {}", e))?;
        Ok(rows)
    }

    /// Returns the total number of logs stored for this scenario.
    pub async fn count_logs(&self, scenario_name: &str) -> anyhow::Result<u64> {
        let row: CountRow = self
            .client
            .query("SELECT count() AS n FROM blockchain_logs WHERE scenario_name = ?")
            .bind(scenario_name)
            .fetch_one::<CountRow>()
            .await
            .map_err(|e| anyhow::anyhow!("count_logs query failed: {}", e))?;
        Ok(row.n)
    }

    /// Returns the count of reorg-flagged logs for this scenario.
    pub async fn count_reorg_logs(&self, scenario_name: &str) -> anyhow::Result<u64> {
        let row: CountRow = self
            .client
            .query(
                "SELECT count() AS n FROM blockchain_logs \
                 WHERE scenario_name = ? AND is_reorg = 1",
            )
            .bind(scenario_name)
            .fetch_one::<CountRow>()
            .await
            .map_err(|e| anyhow::anyhow!("count_reorg_logs query failed: {}", e))?;
        Ok(row.n)
    }

    /// Returns compression stats from ClickHouse system tables.
    pub async fn compression_stats(&self) -> anyhow::Result<CompressionStats> {
        let row: CompressionStats = self
            .client
            .query(
                "SELECT \
                   sum(data_compressed_bytes) AS compressed_bytes, \
                   sum(data_uncompressed_bytes) AS uncompressed_bytes \
                 FROM system.parts \
                 WHERE database = ? AND table = 'blockchain_logs' AND active = 1",
            )
            .bind(&self.database)
            .fetch_one::<CompressionStats>()
            .await
            .map_err(|e| anyhow::anyhow!("compression_stats query failed: {}", e))?;
        Ok(row)
    }

    /// Checks for duplicate block hashes between reorg and non-reorg logs.
    pub async fn count_duplicate_block_hashes(&self, scenario_name: &str) -> anyhow::Result<u64> {
        let row: CountRow = self
            .client
            .query(
                "SELECT count() AS n \
                 FROM blockchain_logs AS r \
                 JOIN blockchain_logs AS orig \
                   ON r.block_hash = orig.block_hash \
                  AND r.is_reorg = 1 \
                  AND orig.is_reorg = 0 \
                 WHERE r.scenario_name = ?",
            )
            .bind(scenario_name)
            .fetch_one::<CountRow>()
            .await
            .map_err(|e| anyhow::anyhow!("duplicate_block_hashes query failed: {}", e))?;
        Ok(row.n)
    }
}

#[derive(Debug, Row, Deserialize)]
pub struct AlertRow {
    pub alert_id: String,
    pub scenario_name: String,
    pub fired_at_unix: i64,
    pub log_ts_unix: i64,
    pub latency_ms: f64,
    pub alert_type: String,
}

#[derive(Debug, Row, Deserialize)]
pub struct CountRow {
    pub n: u64,
}

#[derive(Debug, Row, Deserialize)]
pub struct CompressionStats {
    pub compressed_bytes: u64,
    pub uncompressed_bytes: u64,
}

impl CompressionStats {
    pub fn ratio(&self) -> f64 {
        if self.compressed_bytes == 0 {
            return 0.0;
        }
        self.uncompressed_bytes as f64 / self.compressed_bytes as f64
    }

    pub fn cost_reduction_pct(&self) -> f64 {
        let ratio = self.ratio();
        if ratio <= 1.0 {
            return 0.0;
        }
        ((ratio - 1.0) / ratio) * 100.0
    }
}
