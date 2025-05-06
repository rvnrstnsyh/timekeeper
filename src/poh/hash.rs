use blake3::Hasher as Blake3Hasher;
use ring::digest::{Context as RingContext, Digest, SHA256, digest};

use crate::DEFAULT_HASH;

// Helper function to get algorithm type safely.
#[inline]
fn get_hash_algorithm() -> u8 {
    unsafe {
        match DEFAULT_HASH {
            0 => 0, // SHA256.
            1 => 1, // BLAKE3.
            _ => 0, // Default to SHA256 for any other value.
        }
    }
}

/// Computes a hash of the input data using the selected algorithm.
///
/// # Parameters
/// - `data`: The data to hash.
///
/// # Returns
/// A 32-byte array containing the hash.
#[inline]
pub fn hash(data: &[u8]) -> [u8; 32] {
    match get_hash_algorithm() {
        1 => {
            // BLAKE3.
            *blake3::hash(data).as_bytes()
        }
        _ => {
            // SHA256 (default).
            let hash_result: Digest = digest(&SHA256, data);
            let mut hash_bytes: [u8; 32] = [0u8; 32];
            hash_bytes.copy_from_slice(hash_result.as_ref());
            hash_bytes
        }
    }
}

/// Compute a hash of the concatenation of previous hash and data.
///
/// # Parameters
/// - `prev_hash`: The previous hash in the chain.
/// - `data`: Additional data to include in the hash.
///
/// # Returns
/// A 32-byte array containing the new hash.
#[inline]
pub fn hash_with_data(prev_hash: &[u8; 32], data: &[u8]) -> [u8; 32] {
    match get_hash_algorithm() {
        1 => {
            // BLAKE3.
            let mut hasher: Blake3Hasher = Blake3Hasher::new();
            hasher.update(prev_hash);
            hasher.update(data);
            *hasher.finalize().as_bytes()
        }
        _ => {
            // SHA256 (default).
            let mut context: RingContext = RingContext::new(&SHA256);
            context.update(prev_hash);
            context.update(data);
            let result: Digest = context.finish();
            let mut hash_bytes: [u8; 32] = [0u8; 32];
            hash_bytes.copy_from_slice(result.as_ref());
            hash_bytes
        }
    }
}

/// Extends the hash chain by applying the hash function iteratively.
/// Optimized for better performance with loop unrolling.
///
/// # Parameters
/// - `prev_hash`: The previous hash in the chain.
/// - `iterations`: Number of times to apply the hash function.
///
/// # Returns
/// A 32-byte array containing the resulting hash after all iterations.
pub fn extend_hash_chain(prev_hash: &[u8; 32], iterations: u64) -> [u8; 32] {
    let mut current_hash: [u8; 32] = *prev_hash;
    // Short path for small iteration counts.
    if iterations < 8 {
        for _ in 0..iterations {
            current_hash = hash_single(&current_hash);
        }
        return current_hash;
    }
    // Main loop with unrolling for better pipelining.
    let mut i: u64 = 0;
    while i + 8 <= iterations {
        // Each iteration is strictly sequential but unrolling helps CPU pipeline.
        current_hash = hash_single(&current_hash);
        current_hash = hash_single(&current_hash);
        current_hash = hash_single(&current_hash);
        current_hash = hash_single(&current_hash);
        current_hash = hash_single(&current_hash);
        current_hash = hash_single(&current_hash);
        current_hash = hash_single(&current_hash);
        current_hash = hash_single(&current_hash);
        i += 8;
    }
    // Handle remaining iterations.
    for _ in i..iterations {
        current_hash = hash_single(&current_hash);
    }
    return current_hash;
}

/// Single hash operation - extracted for better inlining and optimization.
#[inline(always)]
fn hash_single(input: &[u8; 32]) -> [u8; 32] {
    match get_hash_algorithm() {
        1 => {
            // BLAKE3.
            let mut hasher: Blake3Hasher = Blake3Hasher::new();
            hasher.update(input);
            *hasher.finalize().as_bytes()
        }
        _ => {
            // SHA256 (default).
            let mut context: RingContext = RingContext::new(&SHA256);
            context.update(input);
            let result: Digest = context.finish();
            let mut hash_bytes: [u8; 32] = [0u8; 32];
            hash_bytes.copy_from_slice(result.as_ref());
            hash_bytes
        }
    }
}

/// Verifies that a hash is the result of extending the previous hash
/// by the specified number of iterations.
///
/// # Parameters
/// - `prev_hash`: The starting hash.
/// - `next_hash`: The hash to verify.
/// - `iterations`: The number of hash iterations that should connect them.
/// - `event_data`: Optional event data that was inserted before hashing.
///
/// # Returns
/// `true` if the hash chain is valid, `false` otherwise.
pub fn verify_hash_chain(prev_hash: &[u8; 32], next_hash: &[u8; 32], iterations: u64, event_data: Option<&[u8]>) -> bool {
    let mut expected_hash: [u8; 32] = *prev_hash;
    // If there's event data, hash it with the previous hash first.
    if let Some(data) = event_data {
        expected_hash = hash_with_data(&expected_hash, data);
    }
    // Extend the hash chain by the specified number of iterations.
    expected_hash = extend_hash_chain(&expected_hash, iterations);
    // Constant-time comparison to prevent timing attacks.
    constant_time_eq(&expected_hash, next_hash)
}

/// Compare two hashes in constant time to prevent timing attacks.
#[inline]
fn constant_time_eq(a: &[u8; 32], b: &[u8; 32]) -> bool {
    let mut result: u8 = 0;
    for i in 0..32 {
        result |= a[i] ^ b[i];
    }
    result == 0
}

/// Set the hash algorithm to use.
///
/// # Parameters
/// - `algorithm`: 0 for SHA256, 1 for BLAKE3
///
/// # Safety
/// This function changes a global static variable and should be called
/// only during initialization or when it's guaranteed no other threads
/// are using the hash functions.
pub fn set_hash_algorithm(algorithm: u8) {
    unsafe {
        DEFAULT_HASH = match algorithm {
            0 => 0, // SHA256.
            1 => 1, // BLAKE3.
            _ => 0, // Default to SHA256.
        };
    }
}

/// Get the currently selected hash algorithm.
///
/// # Returns
/// 0 for SHA256, 1 for BLAKE3.
pub fn get_current_algorithm() -> u8 {
    get_hash_algorithm()
}

/// Get the name of the currently selected hash algorithm.
///
/// # Returns
/// A string describing the algorithm in use.
pub fn get_algorithm_name() -> &'static str {
    match get_hash_algorithm() {
        1 => "BLAKE3",
        _ => "SHA256",
    }
}
