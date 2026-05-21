use std::sync::Arc;

use tokio::io::{AsyncWriteExt, BufWriter};
use tokio::fs::File;
use tokio::sync::Mutex;

use crate::synthesizer::log_builder::BlockchainLog;

/// Writes newline-delimited JSON to a file (or stdout when path is "-").
/// Zero native dependencies — works on any platform without a C toolchain.
pub struct FileProducer {
    sink: Arc<Mutex<Sink>>,
}

enum Sink {
    File(BufWriter<File>),
    Stdout,
}

impl FileProducer {
    pub async fn to_file(path: &str) -> anyhow::Result<Self> {
        let file = tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .await
            .map_err(|e| anyhow::anyhow!("Cannot open output file {}: {}", path, e))?;
        tracing::info!(path, "File publisher ready");
        Ok(Self {
            sink: Arc::new(Mutex::new(Sink::File(BufWriter::new(file)))),
        })
    }

    pub fn to_stdout() -> Self {
        tracing::info!("Stdout publisher ready (NDJSON)");
        Self {
            sink: Arc::new(Mutex::new(Sink::Stdout)),
        }
    }

    pub async fn publish(&self, log: &BlockchainLog) -> anyhow::Result<()> {
        let line = serde_json::to_string(log)
            .map_err(|e| anyhow::anyhow!("Serialization error: {}", e))?;
        let mut sink = self.sink.lock().await;
        match &mut *sink {
            Sink::File(w) => {
                w.write_all(line.as_bytes()).await?;
                w.write_all(b"\n").await?;
            }
            Sink::Stdout => {
                println!("{}", line);
            }
        }
        Ok(())
    }

    pub async fn publish_batch(&self, logs: &[BlockchainLog]) -> anyhow::Result<()> {
        if logs.is_empty() {
            return Ok(());
        }
        let mut sink = self.sink.lock().await;
        for log in logs {
            let line = serde_json::to_string(log)
                .map_err(|e| anyhow::anyhow!("Serialization error: {}", e))?;
            match &mut *sink {
                Sink::File(w) => {
                    w.write_all(line.as_bytes()).await?;
                    w.write_all(b"\n").await?;
                }
                Sink::Stdout => {
                    println!("{}", line);
                }
            }
        }
        Ok(())
    }

    pub fn in_flight_count(&self) -> i32 { 0 }
    pub fn queue_utilization(&self) -> f64 { 0.0 }
}
