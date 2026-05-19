use rand::Rng;
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};

use crate::synthesizer::hashes::{random_address, random_hash32, FixedHash32};

// ─── Flash Loan ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlashLoanParams {
    pub initiator: String,
    pub token: String,
    pub amount: u128,
    pub premium: u128,
    pub referral_code: u16,
    pub on_behalf_of: String,
    pub swap_path: Vec<SwapHop>,
    pub collateral_assets: Vec<CollateralEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapHop {
    pub pool_address: String,
    pub token_in: String,
    pub token_out: String,
    pub fee_tier: u32,
    pub sqrt_price_x96: String,
    pub liquidity: u128,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollateralEntry {
    pub asset: String,
    pub amount: u128,
    pub price_usd_e8: u64,
    pub ltv: u16,
}

pub fn gen_flash_loan(rng: &mut impl Rng) -> FlashLoanParams {
    let n_hops: usize = rng.gen_range(2..=5);
    let n_collateral: usize = rng.gen_range(1..=4);
    let fee_tiers = [500u32, 3000, 10000];

    FlashLoanParams {
        initiator: random_address(rng),
        token: random_address(rng),
        amount: rng.gen_range(1_000_000u128..20_000_000u128) * 1_000_000_000_000_000_000,
        premium: rng.gen_range(1u128..10u128) * 1_000_000_000_000_000,
        referral_code: rng.gen(),
        on_behalf_of: random_address(rng),
        swap_path: (0..n_hops)
            .map(|_| SwapHop {
                pool_address: random_address(rng),
                token_in: random_address(rng),
                token_out: random_address(rng),
                fee_tier: *fee_tiers.choose(rng).unwrap(),
                sqrt_price_x96: format!("{}", rng.gen_range(1u128..u128::MAX / 2)),
                liquidity: rng.gen_range(1_000_000u128..10_000_000_000_000_000_000u128),
            })
            .collect(),
        collateral_assets: (0..n_collateral)
            .map(|_| CollateralEntry {
                asset: random_address(rng),
                amount: rng.gen_range(1u128..1_000_000u128) * 1_000_000_000_000_000_000,
                price_usd_e8: rng.gen_range(100_000_000u64..10_000_000_000u64),
                ltv: rng.gen_range(5000u16..8500u16),
            })
            .collect(),
    }
}

// ─── AMM Swap ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AmmSwapParams {
    pub sender: String,
    pub recipient: String,
    pub amount0: i128,
    pub amount1: i128,
    pub sqrt_price_x96: String,
    pub liquidity: u128,
    pub tick: i32,
    pub fee_growth_global0_x128: String,
    pub fee_growth_global1_x128: String,
    pub protocol_fees: ProtocolFees,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtocolFees {
    pub token0: u128,
    pub token1: u128,
}

pub fn gen_amm_swap(rng: &mut impl Rng) -> AmmSwapParams {
    let amount0: i128 = rng.gen_range(-1_000_000_000_000_000_000i128..1_000_000_000_000_000_000i128);
    let amount1: i128 = -amount0 + rng.gen_range(-1_000_000i128..1_000_000i128);

    AmmSwapParams {
        sender: random_address(rng),
        recipient: random_address(rng),
        amount0,
        amount1,
        sqrt_price_x96: format!("{}", rng.gen_range(1u128..u128::MAX / 2)),
        liquidity: rng.gen_range(1_000_000u128..10_000_000_000_000_000_000u128),
        tick: rng.gen_range(-887272i32..887272i32),
        fee_growth_global0_x128: format!("{}", rng.gen_range(0u128..u128::MAX / 4)),
        fee_growth_global1_x128: format!("{}", rng.gen_range(0u128..u128::MAX / 4)),
        protocol_fees: ProtocolFees {
            token0: rng.gen_range(0u128..1_000_000_000_000u128),
            token1: rng.gen_range(0u128..1_000_000_000_000u128),
        },
    }
}

// ─── NFT Sale ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NftSaleParams {
    pub collection: String,
    pub token_id: u128,
    pub seller: String,
    pub buyer: String,
    pub price_wei: u128,
    pub royalty_wei: u128,
    pub marketplace_fee: u128,
    pub traits: Vec<NftTrait>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NftTrait {
    pub key: String,
    pub value: String,
    pub rarity_bps: u16,
}

const TRAIT_KEYS: &[&str] = &["Background", "Eyes", "Mouth", "Hat", "Body", "Accessory"];
const TRAIT_VALUES: &[&str] = &["Rare", "Common", "Uncommon", "Epic", "Legendary", "Mythic"];

pub fn gen_nft_sale(rng: &mut impl Rng) -> NftSaleParams {
    let price: u128 = rng.gen_range(100_000_000_000_000_000u128..100_000_000_000_000_000_000u128);
    let royalty = price / 20;
    let fee = price / 50;
    let n_traits: usize = rng.gen_range(3..=6);

    NftSaleParams {
        collection: random_address(rng),
        token_id: rng.gen_range(0u128..10000u128),
        seller: random_address(rng),
        buyer: random_address(rng),
        price_wei: price,
        royalty_wei: royalty,
        marketplace_fee: fee,
        traits: (0..n_traits)
            .map(|i| NftTrait {
                key: TRAIT_KEYS[i % TRAIT_KEYS.len()].to_string(),
                value: TRAIT_VALUES.choose(rng).unwrap().to_string(),
                rarity_bps: rng.gen_range(50u16..5000u16),
            })
            .collect(),
    }
}

// ─── Bridge Transfer ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeTransferParams {
    pub source_chain_id: u64,
    pub dest_chain_id: u64,
    pub token: String,
    pub amount: u128,
    pub nonce: u64,
    pub merkle_proof: Vec<FixedHash32>,
}

const CHAIN_IDS: &[u64] = &[1, 10, 42161, 137, 8453, 56];

pub fn gen_bridge_transfer(rng: &mut impl Rng) -> BridgeTransferParams {
    let src = *CHAIN_IDS.choose(rng).unwrap();
    let dst = loop {
        let d = *CHAIN_IDS.choose(rng).unwrap();
        if d != src {
            break d;
        }
    };
    let proof_len: usize = rng.gen_range(4..=16);

    BridgeTransferParams {
        source_chain_id: src,
        dest_chain_id: dst,
        token: random_address(rng),
        amount: rng.gen_range(1_000_000_000_000_000_000u128..10_000_000_000_000_000_000_000u128),
        nonce: rng.gen(),
        merkle_proof: (0..proof_len).map(|_| random_hash32(rng)).collect(),
    }
}

// ─── Unified enum ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DecodedParams {
    FlashLoan(FlashLoanParams),
    AmmSwap(AmmSwapParams),
    NftSale(NftSaleParams),
    BridgeTransfer(BridgeTransferParams),
}
