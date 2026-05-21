#![cfg(feature = "kafka")]

use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use rskafka::client::partition::{Compression, UnknownTopicHandling};
use rskafka::client::{Client, ClientBuilder};
use rskafka::record::Record;
use chrono::Utc;

use crate::dashboard::metrics::SimMetrics;
use crate::synthesizer::log_builder::BlockchainLog;

pub struct KafkaProducer {
    partition_client: rskafka::client::partition::PartitionClient,
    topic: String,
    in_flight: Arc<AtomicU64>,
    metrics: Arc<SimMetrics>,
}

impl KafkaProducer {
    pub async fn new(
        brokers: &str,
        topic: &str,
        metrics: Arc<SimMetrics>,
    ) -> anyhow::Result<Self> {
        let broker_list: Vec<String> = brokers
            .split(',')
            .map(|s| s.trim().to_string())
            .collect();

        let client: Client = ClientBuilder::new(broker_list)
            .build()
            .await
            .map_err(|e| anyhow::anyhow!("Kafka connection failed ({}): {}", brokers, e))?;

        let partition_client = client
            .partition_client(topic, 0, UnknownTopicHandling::Retry)
            .await
            .map_err(|e| anyhow::anyhow!("Kafka partition client error (topic={}): {}", topic, e))?;

        tracing::info!(brokers, topic, "Kafka producer ready (partition 0)");

        Ok(Self {
            partition_client,
            topic: topic.to_string(),
            in_flight: Arc::new(AtomicU64::new(0)),
            metrics,
        })
    }

    /// Serialises one log to JSON and sends it to Kafka.
    pub async fn publish(&self, log: &BlockchainLog) -> anyhow::Result<()> {
        let payload = serde_json::to_vec(log)
            .map_err(|e| anyhow::anyhow!("Serialization failed: {}", e))?;
        let key = log.block_number.to_string().into_bytes();

        let record = Record {
            key: Some(key),
            value: Some(payload),
            headers: BTreeMap::new(),
            timestamp: Utc::now(),
        };

        self.in_flight.fetch_add(1, Ordering::Relaxed);
        let result = self
            .partition_client
            .produce(vec![record], Compression::Snappy)
            .await;
        self.in_flight.fetch_sub(1, Ordering::Relaxed);

        result.map_err(|e| anyhow::anyhow!("Kafka produce error: {}", e))?;
        Ok(())
    }

    /// Sends a batch of reorg logs in one Kafka request.
    pub async fn publish_batch(&self, logs: &[BlockchainLog]) -> anyhow::Result<()> {
        if logs.is_empty() {
            return Ok(());
        }
        let records: Vec<Record> = logs
            .iter()
            .map(|log| Record {
                key: Some(log.block_number.to_string().into_bytes()),
                value: Some(serde_json::to_vec(log).unwrap_or_default()),
                headers: BTreeMap::new(),
                timestamp: Utc::now(),
            })
            .collect();

        self.partition_client
            .produce(records, Compression::Snappy)
            .await
            .map_err(|e| anyhow::anyhow!("Kafka batch produce error: {}", e))?;

        Ok(())
    }

    /// Returns the current estimated in-flight message count.
    pub fn in_flight_count(&self) -> i32 {
        self.in_flight.load(Ordering::Relaxed) as i32
    }

    /// Returns queue utilization as a fraction [0.0, 1.0] (capped at 1000 in-flight).
    pub fn queue_utilization(&self) -> f64 {
        (self.in_flight.load(Ordering::Relaxed) as f64 / 1000.0).min(1.0)
    }
}
