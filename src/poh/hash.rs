use ring::digest::{Context, Digest, SHA256, digest};

/// Computes a SHA-256 hash of the input data.
///
/// # Parameters
/// - `data`: The data to hash
///
/// # Returns
/// A 32-byte array containing the hash.
#[inline]
pub fn hash(data: &[u8]) -> [u8; 32] {
    let hash_result: Digest = digest(&SHA256, data);
    let mut hash_bytes: [u8; 32] = [0u8; 32];
    hash_bytes.copy_from_slice(hash_result.as_ref());
    return hash_bytes;
}

/// Compute a SHA-256 hash of the concatenation of previous hash and data.
///
/// # Parameters
/// - `prev_hash`: The previous hash in the chain
/// - `data`: Additional data to include in the hash
///
/// # Returns
/// A 32-byte array containing the new hash.
#[inline]
pub fn hash_with_data(prev_hash: &[u8; 32], data: &[u8]) -> [u8; 32] {
    let mut context: Context = Context::new(&SHA256);
    context.update(prev_hash);
    context.update(data); // Direct data processing, no double hashing.

    let result: Digest = context.finish();
    let mut hash_bytes: [u8; 32] = [0u8; 32];
    hash_bytes.copy_from_slice(result.as_ref());
    return hash_bytes;
}

/// Extends the hash chain by applying the hash function iteratively.
/// Optimized for better performance with loop unrolling.
///
/// # Parameters
/// - `prev_hash`: The previous hash in the chain
/// - `iterations`: Number of times to apply the hash function
///
/// # Returns
/// A 32-byte array containing the resulting hash after all iterations
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
        // Unroll 8 iterations for better instruction pipelining.
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
    let mut context: Context = Context::new(&SHA256);
    context.update(input);
    let result: Digest = context.finish();
    let mut hash_bytes: [u8; 32] = [0u8; 32];
    hash_bytes.copy_from_slice(result.as_ref());
    return hash_bytes;
}

/// Verifies that a hash is the result of extending the previous hash
/// by the specified number of iterations.
///
/// # Parameters
/// - `prev_hash`: The starting hash
/// - `next_hash`: The hash to verify
/// - `iterations`: The number of hash iterations that should connect them
/// - `event_data`: Optional event data that was inserted before hashing
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
