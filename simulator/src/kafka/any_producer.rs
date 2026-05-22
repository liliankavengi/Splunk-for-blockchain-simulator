use super::file_producer::FileProducer;
use crate::synthesizer::log_builder::BlockchainLog;

#[cfg(feature = "kafka")]
use super::producer::KafkaProducer;

/// Unified producer that dispatches to whichever backend was configured.
/// Add new variants here as needed (e.g. HTTP REST proxy).
pub enum AnyProducer {
    File(FileProducer),
    #[cfg(feature = "kafka")]
    Kafka(KafkaProducer),
}

impl AnyProducer {
    pub async fn publish(&self, log: &BlockchainLog) -> anyhow::Result<()> {
        match self {
            Self::File(p) => p.publish(log).await,
            #[cfg(feature = "kafka")]
            Self::Kafka(p) => p.publish(log).await,
        }
    }

    pub async fn publish_batch(&self, logs: &[BlockchainLog]) -> anyhow::Result<()> {
        match self {
            Self::File(p) => p.publish_batch(logs).await,
            #[cfg(feature = "kafka")]
            Self::Kafka(p) => p.publish_batch(logs).await,
        }
    }

    pub async fn flush(&self) -> anyhow::Result<()> {
        match self {
            Self::File(p) => p.flush().await,
            #[cfg(feature = "kafka")]
            Self::Kafka(_) => Ok(()),
        }
    }

    pub fn in_flight_count(&self) -> i32 {
        match self {
            Self::File(p) => p.in_flight_count(),
            #[cfg(feature = "kafka")]
            Self::Kafka(p) => p.in_flight_count(),
        }
    }

    pub fn queue_utilization(&self) -> f64 {
        match self {
            Self::File(p) => p.queue_utilization(),
            #[cfg(feature = "kafka")]
            Self::Kafka(p) => p.queue_utilization(),
        }
    }
}
