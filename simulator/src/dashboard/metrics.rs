use anyhow::Context;
use prometheus::{Encoder, Gauge, IntCounter, Registry, TextEncoder};

pub struct SimMetrics {
    pub registry: Registry,
    pub logs_per_second: Gauge,
    pub kafka_buffer_lag: Gauge,
    pub total_logs_produced: IntCounter,
    pub compression_ratio: Gauge,
    pub cost_reduction_pct: Gauge,
}

impl SimMetrics {
    pub fn new() -> anyhow::Result<Self> {
        let registry = Registry::new();

        macro_rules! gauge {
            ($name:expr, $help:expr) => {{
                let g = Gauge::new($name, $help)
                    .with_context(|| concat!("creating gauge ", $name))?;
                registry.register(Box::new(g.clone()))
                    .with_context(|| concat!("registering gauge ", $name))?;
                g
            }};
        }

        let logs_per_second = gauge!("sim_logs_per_second", "Current log generation rate (logs/sec)");
        let kafka_buffer_lag = gauge!("sim_kafka_buffer_lag", "Kafka producer in-flight message count");
        let compression_ratio = gauge!("sim_clickhouse_compression_ratio", "Estimated ClickHouse LZ4 compression ratio");
        let cost_reduction_pct = gauge!("sim_cost_reduction_pct", "Estimated storage cost reduction vs raw (%)");

        let total_logs_produced = IntCounter::new(
            "sim_total_logs_produced",
            "Cumulative number of logs published to Kafka",
        )
        .context("creating counter sim_total_logs_produced")?;
        registry
            .register(Box::new(total_logs_produced.clone()))
            .context("registering counter sim_total_logs_produced")?;

        // Static estimates — updated by background task based on observed compression
        compression_ratio.set(7.4);
        cost_reduction_pct.set(82.0);

        Ok(Self {
            registry,
            logs_per_second,
            kafka_buffer_lag,
            total_logs_produced,
            compression_ratio,
            cost_reduction_pct,
        })
    }

    /// Renders all metrics in Prometheus text exposition format.
    pub fn render_text(&self) -> String {
        let encoder = TextEncoder::new();
        let families = self.registry.gather();
        let mut buf = Vec::new();
        if let Err(e) = encoder.encode(&families, &mut buf) {
            tracing::warn!(error = %e, "Prometheus encode failed");
        }
        String::from_utf8(buf).unwrap_or_default()
    }
}
