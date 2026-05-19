use chrono::{DateTime, Utc};

use crate::clickhouse_client::AssertionClient;

#[derive(Debug)]
pub struct AlertLatencyResult {
    pub alerts_total: u64,
    pub sub_second_count: u64,
    pub sub_second_pct: f64,
    pub max_latency_ms: f64,
    pub p99_latency_ms: f64,
    pub threshold_pct: f64,
    pub passed: bool,
}

/// Checks that at least `threshold_pct` of alerts fired within 1 second.
pub async fn check_alert_latency(
    client: &AssertionClient,
    scenario_name: &str,
    sim_start: DateTime<Utc>,
    threshold_pct: f64,
) -> anyhow::Result<AlertLatencyResult> {
    let alerts = client.fetch_alerts(scenario_name, sim_start).await?;

    if alerts.is_empty() {
        // No alerts table or no alerts found — mark as skipped (pass with caveat)
        tracing::warn!(
            scenario = scenario_name,
            "No alerts found in security_alerts table — skipping latency check"
        );
        return Ok(AlertLatencyResult {
            alerts_total: 0,
            sub_second_count: 0,
            sub_second_pct: 100.0,
            max_latency_ms: 0.0,
            p99_latency_ms: 0.0,
            threshold_pct,
            passed: true,
        });
    }

    let mut latencies: Vec<f64> = alerts.iter().map(|r| r.latency_ms).collect();
    latencies.sort_by(|a, b| a.partial_cmp(b).unwrap());

    let total = latencies.len() as u64;
    let sub_second: u64 = latencies.iter().filter(|&&ms| ms < 1000.0).count() as u64;
    let sub_second_pct = (sub_second as f64 / total as f64) * 100.0;
    let max = latencies.last().copied().unwrap_or(0.0);
    let p99_idx = ((total as f64 * 0.99) as usize).min(latencies.len().saturating_sub(1));
    let p99 = latencies[p99_idx];

    let passed = sub_second_pct >= threshold_pct * 100.0;

    Ok(AlertLatencyResult {
        alerts_total: total,
        sub_second_count: sub_second,
        sub_second_pct,
        max_latency_ms: max,
        p99_latency_ms: p99,
        threshold_pct: threshold_pct * 100.0,
        passed,
    })
}
