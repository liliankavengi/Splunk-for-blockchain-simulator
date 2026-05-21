use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use anyhow::Context;

/// Raw deserialization target — accepts both the legacy field names and the
/// newer declarative-template schema (scenario_id / attack_vector / etc.).
#[derive(Debug, Clone, Deserialize, Serialize)]
struct RawScenario {
    // ── new schema ───────────────────────────────────────────────────────────
    pub scenario_id: Option<String>,
    pub attack_vector: Option<String>,
    pub steady_state_tps: Option<u64>,
    pub spike_peak_tps: Option<u64>,
    pub payload_nesting_depth: Option<String>,

    // ── legacy schema (still supported) ──────────────────────────────────────
    pub scenario_name: Option<String>,
    pub base_tps: Option<u64>,
    pub burst_tps: Option<u64>,
    pub duration_seconds: Option<u64>,
    pub target_contract_type: Option<ContractType>,
    pub nested_fields_complexity: Option<Complexity>,
    #[serde(default)]
    pub reorg_backtrack_blocks: Option<u64>,
}

/// Resolved, normalised blueprint used by the engine.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ScenarioBlueprint {
    pub scenario_id: Option<String>,
    pub attack_vector: Option<String>,
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
    let raw: RawScenario = match ext {
        "yaml" | "yml" => serde_yaml::from_str(&content)
            .with_context(|| "Failed to parse YAML scenario")?,
        _ => serde_json::from_str(&content)
            .with_context(|| "Failed to parse JSON scenario")?,
    };

    // Resolve name: prefer scenario_name, fall back to attack_vector, then scenario_id.
    let scenario_name = raw.scenario_name
        .or_else(|| raw.attack_vector.clone())
        .or_else(|| raw.scenario_id.clone())
        .unwrap_or_else(|| "unnamed".to_string());

    // Resolve TPS: new names take priority over legacy names.
    let base_tps  = raw.steady_state_tps.or(raw.base_tps)
        .unwrap_or(100);
    let burst_tps = raw.spike_peak_tps.or(raw.burst_tps)
        .unwrap_or(base_tps);

    // Resolve complexity: payload_nesting_depth "recursive_map" / "high" -> High, etc.
    let nested_fields_complexity = raw.nested_fields_complexity.unwrap_or_else(|| {
        match raw.payload_nesting_depth.as_deref() {
            Some("recursive_map") | Some("high") => Complexity::High,
            Some("medium")                        => Complexity::Medium,
            _                                     => Complexity::Low,
        }
    });

    Ok(ScenarioBlueprint {
        scenario_id:   raw.scenario_id,
        attack_vector: raw.attack_vector,
        scenario_name,
        base_tps,
        burst_tps,
        duration_seconds: raw.duration_seconds.unwrap_or(60),
        target_contract_type: raw.target_contract_type
            .unwrap_or(ContractType::DeFiLendingPool),
        nested_fields_complexity,
        reorg_backtrack_blocks: raw.reorg_backtrack_blocks,
    })
}
