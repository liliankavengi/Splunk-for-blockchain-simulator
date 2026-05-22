use crate::clickhouse_client::AssertionClient;

#[derive(Debug)]
pub struct CompletenessResult {
    pub expected: u64,
    pub actual: u64,
    pub loss_pct: f64,
    pub tolerance_pct: f64,
    pub passed: bool,
}

/// Asserts that the number of logs in ClickHouse is within `tolerance_pct` of `expected`.
pub async fn check_log_completeness(
    client: &AssertionClient,
    scenario_name: &str,
    expected: u64,
    tolerance_pct: f64,
) -> anyhow::Result<CompletenessResult> {
    let actual = client.count_logs(scenario_name).await?;

    let loss_pct = if expected == 0 {
        0.0
    } else {
        // Use absolute difference so over-ingestion (actual > expected) is also flagged.
        let diff = (expected as i64 - actual as i64).unsigned_abs();
        (diff as f64 / expected as f64) * 100.0
    };

    let passed = loss_pct <= tolerance_pct * 100.0;

    Ok(CompletenessResult {
        expected,
        actual,
        loss_pct,
        tolerance_pct: tolerance_pct * 100.0,
        passed,
    })
}
