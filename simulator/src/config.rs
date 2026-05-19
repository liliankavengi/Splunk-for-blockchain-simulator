use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use anyhow::Context;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ScenarioBlueprint {
    pub scenario_name: String,
    pub base_tps: u64,
    pub burst_tps: u64,
    pub duration_seconds: u64,
    pub target_contract_type: ContractType,
    pub nested_fields_complexity: Complexity,
    #[serde(default)]
    pub reorg_backtrack_blocks: Option<u64>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, Hash)]
pub enum ContractType {
    #[serde(rename = "DeFi_Lending_Pool")]
    DeFiLendingPool,
    #[serde(rename = "AMM_Swap_Pool")]
    AmmSwapPool,
    #[serde(rename = "NFT_Marketplace")]
    NftMarketplace,
    #[serde(rename = "Bridge")]
    Bridge,
    #[serde(rename = "Governance")]
    Governance,
}

impl std::fmt::Display for ContractType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DeFiLendingPool => write!(f, "DeFi_Lending_Pool"),
            Self::AmmSwapPool     => write!(f, "AMM_Swap_Pool"),
            Self::NftMarketplace  => write!(f, "NFT_Marketplace"),
            Self::Bridge          => write!(f, "Bridge"),
            Self::Governance      => write!(f, "Governance"),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Complexity {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    pub kafka_brokers: String,
    pub kafka_topic: String,
    pub dashboard_port: u16,
    pub dashboard_password: String,
    pub clickhouse_url: String,
    pub scenario_path: PathBuf,
}

pub fn load_scenario(path: &Path) -> anyhow::Result<ScenarioBlueprint> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Cannot read scenario file: {}", path.display()))?;

    let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
    match ext {
        "yaml" | "yml" => serde_yaml::from_str(&content)
            .with_context(|| "Failed to parse YAML scenario"),
        _ => serde_json::from_str(&content)
            .with_context(|| "Failed to parse JSON scenario"),
    }
}
