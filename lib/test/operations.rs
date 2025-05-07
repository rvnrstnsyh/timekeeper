#[cfg(test)]
mod operations {
    use std::time::{Duration, Instant};

    use lib::utils::hash::extend_hash_chain;

    use ring::digest::{Context, Digest, SHA256};

    #[test]
    fn test_hash_chain_correctness() {
        let seed: [u8; 32] = [0u8; 32];
        // Verify with small number of iterations.
        let hash1: [u8; 32] = manual_hash_chain(&seed, 10);
        let hash2: [u8; 32] = extend_hash_chain(&seed, 10);
        assert_eq!(hash1, hash2, "Hash chains should produce identical results.");
    }

    #[test]
    fn test_hash_chain_performance() {
        let seed: [u8; 32] = [0u8; 32];
        let iterations: u64 = 10_000;

        let start: Instant = Instant::now();
        let _ = extend_hash_chain(&seed, iterations);
        let optimized_duration: Duration = start.elapsed();

        println!("Optimized: {:?} for {} iterations.", optimized_duration, iterations);
    }

    // Reference implementation for testing.
    fn manual_hash_chain(prev_hash: &[u8; 32], iterations: u64) -> [u8; 32] {
        let mut current_hash: [u8; 32] = *prev_hash;
        for _ in 0..iterations {
            let mut context: Context = Context::new(&SHA256);
            context.update(&current_hash);
            let result: Digest = context.finish();
            current_hash.copy_from_slice(result.as_ref());
        }
        return current_hash;
    }
}
