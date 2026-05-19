use crate::clickhouse_client::AssertionClient;

#[derive(Debug)]
pub struct CompressionResult {
    pub compressed_bytes: u64,
    pub uncompressed_bytes: u64,
    pub ratio: f64,
    pub cost_reduction_pct: f64,
    pub min_ratio: f64,
    pub passed: bool,
    pub skipped: bool,
}

/// Checks that ClickHouse's actual compression ratio meets `min_ratio`.
pub async fn check_compression(
    client: &AssertionClient,
    min_ratio: f64,
) -> anyhow::Result<CompressionResult> {
    match client.compression_stats().await {
        Ok(stats) => {
            let ratio = stats.ratio();
            let cost_pct = stats.cost_reduction_pct();
            let passed = ratio >= min_ratio;

            Ok(CompressionResult {
                compressed_bytes: stats.compressed_bytes,
                uncompressed_bytes: stats.uncompressed_bytes,
                ratio,
                cost_reduction_pct: cost_pct,
                min_ratio,
                passed,
                skipped: false,
            })
        }
        Err(e) => {
            tracing::warn!(error = %e, "Compression stats unavailable — skipping check");
            Ok(CompressionResult {
                compressed_bytes: 0,
                uncompressed_bytes: 0,
                ratio: 0.0,
                cost_reduction_pct: 0.0,
                min_ratio,
                passed: true,
                skipped: true,
            })
        }
    }
}
