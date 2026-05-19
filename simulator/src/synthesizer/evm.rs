use rand::Rng;

use crate::config::{Complexity, ContractType};
use crate::synthesizer::abi::{
    abi_encode_address, abi_encode_bytes, abi_encode_int256,
    abi_encode_uint256, abi_encode_uint32,
};

/// Builds a structurally-valid ABI-encoded byte stream for the given contract/complexity.
/// The output matches what a real EVM `eth_getLogs` would return in the `data` field.
pub fn generate_evm_data(
    contract_type: &ContractType,
    complexity: &Complexity,
    rng: &mut impl Rng,
) -> Vec<u8> {
    match (contract_type, complexity) {
        (ContractType::DeFiLendingPool, Complexity::High) => encode_flash_loan_full(rng),
        (ContractType::DeFiLendingPool, _) => encode_flash_loan_simple(rng),
        (ContractType::AmmSwapPool, Complexity::High) => encode_swap_full(rng),
        (ContractType::AmmSwapPool, _) => encode_swap_simple(rng),
        (ContractType::NftMarketplace, _) => encode_nft_transfer(rng),
        (ContractType::Bridge, _) => encode_bridge(rng),
        (ContractType::Governance, _) => encode_governance(rng),
    }
}

// Flash loan: initiator(addr) token(addr) amount(uint256) premium(uint256) referralCode(uint16)
fn encode_flash_loan_simple(rng: &mut impl Rng) -> Vec<u8> {
    let mut out = Vec::with_capacity(5 * 32);
    out.extend_from_slice(&abi_encode_address(&random_addr(rng)));
    out.extend_from_slice(&abi_encode_address(&random_addr(rng)));
    out.extend_from_slice(&abi_encode_uint256(rng.gen_range(1u128..20_000_000u128) * 1e18 as u128));
    out.extend_from_slice(&abi_encode_uint256(rng.gen_range(1u128..10u128) * 1_000_000_000_000_000));
    out.extend_from_slice(&abi_encode_uint256(rng.gen_range(0u128..65535u128)));
    out
}

// Flash loan with dynamic bytes payload (high complexity)
fn encode_flash_loan_full(rng: &mut impl Rng) -> Vec<u8> {
    let mut head = Vec::new();
    head.extend_from_slice(&abi_encode_address(&random_addr(rng)));
    head.extend_from_slice(&abi_encode_address(&random_addr(rng)));
    head.extend_from_slice(&abi_encode_uint256(rng.gen_range(1u128..20_000_000u128) * 1_000_000_000_000_000_000));
    head.extend_from_slice(&abi_encode_uint256(rng.gen_range(1u128..100u128) * 1_000_000_000_000_000));
    head.extend_from_slice(&abi_encode_uint256(rng.gen_range(0u128..65535u128)));

    // Dynamic bytes (params payload) — offset points after the 5 head slots
    let head_size = 5 * 32;
    let mut offset_slot = [0u8; 32];
    offset_slot[24..32].copy_from_slice(&(head_size as u64).to_be_bytes());
    head.extend_from_slice(&offset_slot);

    let payload_len: usize = rng.gen_range(64..=512);
    let mut payload = vec![0u8; payload_len];
    rng.fill(payload.as_mut_slice());
    head.extend_from_slice(&abi_encode_bytes(&payload));
    head
}

// Uniswap v3 Swap: sender(addr) recipient(addr) amount0(int256) amount1(int256) sqrtPriceX96(uint160) liquidity(uint128) tick(int24)
fn encode_swap_simple(rng: &mut impl Rng) -> Vec<u8> {
    let mut out = Vec::with_capacity(7 * 32);
    out.extend_from_slice(&abi_encode_address(&random_addr(rng)));
    out.extend_from_slice(&abi_encode_address(&random_addr(rng)));
    let a0 = rng.gen_range(-1_000_000_000_000_000_000i128..1_000_000_000_000_000_000i128);
    out.extend_from_slice(&abi_encode_int256(a0));
    out.extend_from_slice(&abi_encode_int256(-a0 + rng.gen_range(-1_000i128..1_000i128)));
    out.extend_from_slice(&abi_encode_uint256(rng.gen_range(1u128..u128::MAX / 2)));
    out.extend_from_slice(&abi_encode_uint256(rng.gen_range(1_000_000u128..10_000_000_000_000_000_000u128)));
    out.extend_from_slice(&abi_encode_int256(rng.gen_range(-887272i128..887272i128)));
    out
}

fn encode_swap_full(rng: &mut impl Rng) -> Vec<u8> {
    let mut out = encode_swap_simple(rng);
    // Append fee growth globals (uint256 x2) and protocol fees
    out.extend_from_slice(&abi_encode_uint256(rng.gen_range(0u128..u128::MAX / 4)));
    out.extend_from_slice(&abi_encode_uint256(rng.gen_range(0u128..u128::MAX / 4)));
    out.extend_from_slice(&abi_encode_uint256(rng.gen_range(0u128..1_000_000_000_000u128)));
    out.extend_from_slice(&abi_encode_uint256(rng.gen_range(0u128..1_000_000_000_000u128)));
    out
}

// ERC-721 Transfer: from(addr) to(addr) tokenId(uint256)
fn encode_nft_transfer(rng: &mut impl Rng) -> Vec<u8> {
    let mut out = Vec::with_capacity(3 * 32);
    out.extend_from_slice(&abi_encode_address(&random_addr(rng)));
    out.extend_from_slice(&abi_encode_address(&random_addr(rng)));
    out.extend_from_slice(&abi_encode_uint256(rng.gen_range(0u128..10000u128)));
    out
}

// Bridge: nonce(uint256) token(addr) amount(uint256)
fn encode_bridge(rng: &mut impl Rng) -> Vec<u8> {
    let mut out = Vec::with_capacity(3 * 32);
    out.extend_from_slice(&abi_encode_uint256(rng.gen::<u64>() as u128));
    out.extend_from_slice(&abi_encode_address(&random_addr(rng)));
    out.extend_from_slice(&abi_encode_uint256(rng.gen_range(1u128..1_000_000u128) * 1_000_000_000_000_000_000));
    out
}

// Governance VoteCast: proposalId(uint256) support(uint8) weight(uint256)
fn encode_governance(rng: &mut impl Rng) -> Vec<u8> {
    let mut out = Vec::with_capacity(3 * 32);
    out.extend_from_slice(&abi_encode_uint256(rng.gen_range(1u128..10000u128)));
    out.extend_from_slice(&abi_encode_uint32(rng.gen_range(0u32..3u32)));
    out.extend_from_slice(&abi_encode_uint256(rng.gen_range(1u128..1_000_000u128) * 1_000_000_000_000_000_000));
    out
}

fn random_addr(rng: &mut impl Rng) -> [u8; 20] {
    let mut bytes = [0u8; 20];
    rng.fill(&mut bytes);
    bytes
}
