/// Encodes a u128 value left-padded to 32 bytes (ABI uint256 slot).
pub fn abi_encode_uint256(value: u128) -> [u8; 32] {
    let mut slot = [0u8; 32];
    slot[16..32].copy_from_slice(&value.to_be_bytes());
    slot
}

/// Encodes a signed i128 as two's-complement, left-padded to 32 bytes (ABI int256 slot).
pub fn abi_encode_int256(value: i128) -> [u8; 32] {
    let mut slot = if value < 0 { [0xffu8; 32] } else { [0u8; 32] };
    slot[16..32].copy_from_slice(&value.to_be_bytes());
    slot
}

/// Encodes a 20-byte Ethereum address left-padded to 32 bytes.
pub fn abi_encode_address(addr: &[u8; 20]) -> [u8; 32] {
    let mut slot = [0u8; 32];
    slot[12..32].copy_from_slice(addr);
    slot
}

/// Returns a 32-byte slot directly (bytes32 ABI type).
pub fn abi_encode_bytes32(data: &[u8; 32]) -> [u8; 32] {
    *data
}

/// Encodes a dynamic `bytes` value: returns length slot + right-padded data.
/// Does NOT include the 32-byte offset pointer — caller appends at correct position.
pub fn abi_encode_bytes(data: &[u8]) -> Vec<u8> {
    let len = data.len();
    let padded_len = (len + 31) & !31;
    let mut out = Vec::with_capacity(32 + padded_len);

    let mut len_slot = [0u8; 32];
    len_slot[24..32].copy_from_slice(&(len as u64).to_be_bytes());
    out.extend_from_slice(&len_slot);

    out.extend_from_slice(data);
    out.extend(std::iter::repeat(0u8).take(padded_len - len));
    out
}

/// Encodes a u32 left-padded to 32 bytes.
pub fn abi_encode_uint32(value: u32) -> [u8; 32] {
    let mut slot = [0u8; 32];
    slot[28..32].copy_from_slice(&value.to_be_bytes());
    slot
}
