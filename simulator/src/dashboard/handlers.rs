use std::sync::Arc;
use std::sync::atomic::Ordering;

use axum::{extract::State, response::IntoResponse, Json};
use serde::Serialize;

use crate::dashboard::metrics::SimMetrics;
use crate::engine::EngineState;

#[derive(Clone)]
pub struct DashboardCtx {
    pub metrics: Arc<SimMetrics>,
    pub state: Arc<EngineState>,
    pub scenario_name: String,
}

#[derive(Serialize)]
pub struct DashboardSnapshot {
    pub scenario_name: String,
    pub logs_per_second: f64,
    pub kafka_buffer_lag: f64,
    pub total_logs_produced: u64,
    pub compression_ratio: f64,
    pub cold_storage_cost_reduction_pct: f64,
    pub current_block: u64,
    pub current_tps: u64,
    pub elapsed_seconds: f64,
}

/// GET /health — unauthenticated liveness probe for load-balancers and Render.
pub async fn health_handler() -> impl IntoResponse {
    (axum::http::StatusCode::OK, "ok")
}

/// GET /status — returns a live JSON snapshot of simulator metrics.
pub async fn status_handler(State(ctx): State<DashboardCtx>) -> Json<DashboardSnapshot> {
    Json(DashboardSnapshot {
        scenario_name: ctx.scenario_name.clone(),
        logs_per_second: ctx.metrics.logs_per_second.get(),
        kafka_buffer_lag: ctx.metrics.kafka_buffer_lag.get(),
        total_logs_produced: ctx.state.logs_produced.load(Ordering::Relaxed),
        compression_ratio: ctx.metrics.compression_ratio.get(),
        cold_storage_cost_reduction_pct: ctx.metrics.cost_reduction_pct.get(),
        current_block: ctx.state.current_block.load(Ordering::Relaxed),
        current_tps: ctx.state.current_tps.load(Ordering::Relaxed),
        elapsed_seconds: ctx.state.elapsed_secs(),
    })
}

/// GET /metrics — returns Prometheus text exposition format.
pub async fn metrics_handler(State(ctx): State<DashboardCtx>) -> impl IntoResponse {
    let body = ctx.metrics.render_text();
    (
        [(axum::http::header::CONTENT_TYPE, "text/plain; version=0.0.4")],
        body,
    )
}

/// GET / — minimal HTML dashboard for browser viewing.
pub async fn index_handler(State(ctx): State<DashboardCtx>) -> impl IntoResponse {
    let snap = DashboardSnapshot {
        scenario_name: ctx.scenario_name.clone(),
        logs_per_second: ctx.metrics.logs_per_second.get(),
        kafka_buffer_lag: ctx.metrics.kafka_buffer_lag.get(),
        total_logs_produced: ctx.state.logs_produced.load(Ordering::Relaxed),
        compression_ratio: ctx.metrics.compression_ratio.get(),
        cold_storage_cost_reduction_pct: ctx.metrics.cost_reduction_pct.get(),
        current_block: ctx.state.current_block.load(Ordering::Relaxed),
        current_tps: ctx.state.current_tps.load(Ordering::Relaxed),
        elapsed_seconds: ctx.state.elapsed_secs(),
    };

    let html = format!(
        r#"<!DOCTYPE html>
<html>
<head><title>Blockchain Simulator Dashboard</title>
<meta http-equiv="refresh" content="2">
<style>
  body {{ font-family: monospace; background: #0d1117; color: #c9d1d9; padding: 2em; }}
  h1   {{ color: #58a6ff; }}
  table {{ border-collapse: collapse; width: 60%; }}
  td, th {{ border: 1px solid #30363d; padding: 8px 16px; text-align: left; }}
  th {{ color: #8b949e; }}
  .pass {{ color: #3fb950; }}
</style>
</head>
<body>
<h1>SYSTEM METRICS &amp; COST PROJECTIONS</h1>
<p>Scenario: <b>{scenario}</b> &nbsp;|&nbsp; Elapsed: <b>{elapsed:.1}s</b></p>
<table>
  <tr><th>Metric</th><th>Value</th></tr>
  <tr><td>Current Block</td><td class="pass">{block}</td></tr>
  <tr><td>Current Ingestion Rate</td><td class="pass">{tps} Logs / sec</td></tr>
  <tr><td>Total Logs Produced</td><td class="pass">{total}</td></tr>
  <tr><td>Kafka Buffer Lag</td><td>{lag:.0} msgs in-flight</td></tr>
  <tr><td>ClickHouse Compression</td><td class="pass">{cr:.1}x Savings</td></tr>
  <tr><td>Cold S3 Archival Offload</td><td class="pass">{cost:.0}% Cost Reduction</td></tr>
</table>
<p style="color:#8b949e; font-size:0.85em">Auto-refreshes every 2 seconds. Use <code>/metrics</code> for Prometheus scraping.</p>
</body>
</html>"#,
        scenario = snap.scenario_name,
        elapsed = snap.elapsed_seconds,
        block = snap.current_block,
        tps = snap.current_tps,
        total = snap.total_logs_produced,
        lag = snap.kafka_buffer_lag,
        cr = snap.compression_ratio,
        cost = snap.cold_storage_cost_reduction_pct,
    );

    (
        [(axum::http::header::CONTENT_TYPE, "text/html; charset=utf-8")],
        html,
    )
}
