use rand::Rng;
use sha2::{Digest, Sha256};
use serde::{Deserialize, Serialize};

use crate::config::ContractType;

/// A 32-byte value represented as "0x" + 64 hex chars.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FixedHash32(pub String);

impl FixedHash32 {
    pub fn from_bytes(bytes: &[u8; 32]) -> Self {
        Self(format!("0x{}", hex::encode(bytes)))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for FixedHash32 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Generates a random 32-byte hash.
pub fn random_hash32(rng: &mut impl Rng) -> FixedHash32 {
    let mut bytes = [0u8; 32];
    rng.fill(&mut bytes);
    FixedHash32::from_bytes(&bytes)
}

/// Generates a deterministic tx hash from block_number + tx_index.
pub fn deterministic_tx_hash(block_number: u64, tx_index: u32) -> FixedHash32 {
    let mut hasher = Sha256::new();
    hasher.update(b"tx:");
    hasher.update(block_number.to_be_bytes());
    hasher.update(tx_index.to_be_bytes());
    let result: [u8; 32] = hasher.finalize().into();
    FixedHash32::from_bytes(&result)
}

/// Generates a deterministic block hash from block_number + a nonce (for reorg forks).
pub fn deterministic_block_hash(block_number: u64, nonce: u64) -> FixedHash32 {
    let mut hasher = Sha256::new();
    hasher.update(b"block:");
    hasher.update(block_number.to_be_bytes());
    hasher.update(nonce.to_be_bytes());
    let result: [u8; 32] = hasher.finalize().into();
    FixedHash32::from_bytes(&result)
}

/// Returns a known Keccak-like event signature topic for each contract type.
/// Uses SHA256 as a substitute (real Keccak is not included to avoid extra deps).
pub fn event_signature_topic(contract_type: &ContractType, variant: u8) -> FixedHash32 {
    let sig = match (contract_type, variant % 4) {
        (ContractType::DeFiLendingPool, 0) => "FlashLoan(address,address,uint256,uint256,uint8)",
        (ContractType::DeFiLendingPool, 1) => "Borrow(address,address,address,uint256,uint8,uint256,uint16)",
        (ContractType::DeFiLendingPool, 2) => "Repay(address,address,address,uint256,bool)",
        (ContractType::DeFiLendingPool, _) => "LiquidationCall(address,address,address,uint256,uint256,address,bool)",
        (ContractType::AmmSwapPool, 0) => "Swap(address,address,int256,int256,uint160,uint128,int24)",
        (ContractType::AmmSwapPool, 1) => "Mint(address,address,int24,int24,uint128,uint256,uint256)",
        (ContractType::AmmSwapPool, 2) => "Burn(address,int24,int24,uint128,uint256,uint256)",
        (ContractType::AmmSwapPool, _) => "Collect(address,address,int24,int24,uint128,uint128)",
        (ContractType::NftMarketplace, 0) => "Transfer(address,address,uint256)",
        (ContractType::NftMarketplace, 1) => "OrderFulfilled(bytes32,address,address,address)",
        (ContractType::NftMarketplace, 2) => "Approval(address,address,uint256)",
        (ContractType::NftMarketplace, _) => "ApprovalForAll(address,address,bool)",
        (ContractType::Bridge, 0) => "TokensBridged(address,address,uint256,bytes32)",
        (ContractType::Bridge, 1) => "MessageSent(bytes32,address,uint256,bytes)",
        (ContractType::Bridge, 2) => "MessageRelayed(bytes32,address,bool)",
        (ContractType::Bridge, _) => "TokensClaimed(bytes32,address,uint256)",
        (ContractType::Governance, 0) => "ProposalCreated(uint256,address,uint256,uint256,string)",
        (ContractType::Governance, 1) => "VoteCast(address,uint256,uint8,uint256,string)",
        (ContractType::Governance, 2) => "ProposalExecuted(uint256)",
        (ContractType::Governance, _) => "ProposalCanceled(uint256)",
    };
    let mut hasher = Sha256::new();
    hasher.update(sig.as_bytes());
    let result: [u8; 32] = hasher.finalize().into();
    FixedHash32::from_bytes(&result)
}

/// Generates a random indexed topic (topic_1 through topic_3 are indexed params).
pub fn random_indexed_topic(rng: &mut impl Rng) -> FixedHash32 {
    random_hash32(rng)
}

/// Generates a random 20-byte Ethereum address as "0x" + 40 hex chars.
pub fn random_address(rng: &mut impl Rng) -> String {
    let mut bytes = [0u8; 20];
    rng.fill(&mut bytes);
    format!("0x{}", hex::encode(bytes))
}
