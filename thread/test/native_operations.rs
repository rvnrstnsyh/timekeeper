#[cfg(test)]
mod tests {
    use std::{
        sync::{
            Arc, Mutex,
            atomic::{AtomicUsize, Ordering},
        },
        thread as std_thread,
        time::Duration,
    };

    use anyhow::Result;
    use thread::native::types::{Config, CoreAllocation, Manager};
    use thread::native::{manager::default_manager, pool::default_pool};

    #[test]
    fn test_thread_manager_basic() -> Result<()> {
        let manager = default_manager("test-manager")?;

        assert_eq!(manager.name(), "test-manager");
        assert_eq!(manager.running_count(), 0);
        assert!(!manager.is_full());

        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        let handle = manager.spawn(move || {
            counter_clone.fetch_add(1, Ordering::SeqCst);
            std_thread::sleep(Duration::from_millis(50));
            counter_clone.fetch_add(1, Ordering::SeqCst);
            "test result"
        })?;

        assert_eq!(manager.running_count(), 1);

        // Wait for thread to complete
        let result = handle.join().unwrap();
        assert_eq!(result, "test result");
        assert_eq!(counter.load(Ordering::SeqCst), 2);
        assert_eq!(manager.running_count(), 0);

        Ok(())
    }

    #[test]
    fn test_thread_manager_named() -> Result<()> {
        let manager = default_manager("test-manager")?;

        let shared_data = Arc::new(Mutex::new(Vec::new()));
        let shared_data_clone = shared_data.clone();

        let handle = manager.spawn_named("custom-worker".to_string(), move || {
            let thread_name = std_thread::current().name().unwrap_or("unknown").to_string();
            let mut data = shared_data_clone.lock().unwrap();
            data.push(thread_name);
            "done"
        })?;

        let result = handle.join().unwrap();
        assert_eq!(result, "done");

        let data = shared_data.lock().unwrap();
        assert_eq!(data.len(), 1);
        assert_eq!(data[0], "custom-worker");

        Ok(())
    }

    #[test]
    fn test_thread_manager_max_threads() -> Result<()> {
        let config = Config {
            max_threads: 2,
            ..Default::default()
        };

        let manager = Manager::new("limited-manager".to_string(), config)?;

        let handle1 = manager.spawn(|| {
            std_thread::sleep(Duration::from_millis(50));
            1
        })?;

        let handle2 = manager.spawn(|| {
            std_thread::sleep(Duration::from_millis(50));
            2
        })?;

        assert_eq!(manager.running_count(), 2);
        assert!(manager.is_full());

        // Try to spawn a third thread - should fail
        let result = manager.spawn(|| {
            std_thread::sleep(Duration::from_millis(50));
            3
        });

        assert!(result.is_err());

        // Join threads
        assert_eq!(handle1.join().unwrap(), 1);
        assert_eq!(handle2.join().unwrap(), 2);

        assert_eq!(manager.running_count(), 0);
        assert!(!manager.is_full());

        Ok(())
    }

    #[test]
    fn test_core_allocation() -> Result<()> {
        // Test core mask conversion
        let alloc = CoreAllocation::PinnedCores { min: 0, max: 3 };
        let cores = alloc.as_core_mask_vector();
        assert_eq!(cores, vec![0, 1, 2, 3]);

        // Test invalid range
        let invalid = CoreAllocation::PinnedCores { min: 5, max: 3 };
        let cores = invalid.as_core_mask_vector();
        assert!(cores.is_empty());

        // Test validation
        assert!(alloc.validate().is_ok());

        // If system has less than 100 cores, this should fail validation
        let too_high = CoreAllocation::DedicatedCoreSet { min: 0, max: 100 };
        if num_cpus::get() <= 100 {
            assert!(too_high.validate().is_err());
        }

        let bad_range = CoreAllocation::PinnedCores { min: 5, max: 3 };
        assert!(bad_range.validate().is_err());

        Ok(())
    }

    #[test]
    fn test_thread_pool_basic() -> Result<()> {
        let pool = default_pool("test-pool")?;

        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        pool.execute(move || {
            counter_clone.fetch_add(1, Ordering::SeqCst);
            Ok(())
        })?;

        // Allow time for task to execute
        std_thread::sleep(Duration::from_millis(100));

        assert_eq!(counter.load(Ordering::SeqCst), 1);

        let stats = pool.stats();
        assert_eq!(stats.total_jobs_completed, 1);
        assert_eq!(stats.failed_jobs, 0);

        // Cleanup
        pool.shutdown()?;

        Ok(())
    }

    #[test]
    fn test_thread_pool_execute_wait() -> Result<()> {
        let pool = default_pool("test-pool")?;

        let result = pool.execute_wait(|| {
            std_thread::sleep(Duration::from_millis(50));
            Ok(42)
        })?;

        assert_eq!(result, 42);

        // Test error propagation
        let err_result = pool.execute_wait::<_, ()>(|| {
            anyhow::bail!("Test error");
        });

        assert!(err_result.is_err());
        assert!(err_result.unwrap_err().to_string().contains("Test error"));

        // Cleanup
        pool.shutdown()?;

        Ok(())
    }

    #[test]
    fn test_thread_pool_execute_batch() -> Result<()> {
        let pool = default_pool("batch-pool")?;

        let counter = Arc::new(AtomicUsize::new(0));

        let jobs = (0..10)
            .map(|_| {
                let counter = counter.clone();
                move || {
                    counter.fetch_add(1, Ordering::SeqCst);
                    Ok(())
                }
            })
            .collect::<Vec<_>>();

        let count = pool.execute_batch(jobs)?;
        assert_eq!(count, 10);

        // Wait for all jobs to complete
        pool.wait_for_completion()?;

        assert_eq!(counter.load(Ordering::SeqCst), 10);

        let stats = pool.stats();
        assert_eq!(stats.total_jobs_completed, 10);
        assert_eq!(stats.failed_jobs, 0);

        // Cleanup
        pool.shutdown()?;

        Ok(())
    }

    #[test]
    fn test_thread_pool_shutdown() -> Result<()> {
        let mut pool = default_pool("shutdown-pool")?;

        // Add a job that takes some time
        pool.execute(|| {
            std_thread::sleep(Duration::from_millis(100));
            Ok(())
        })?;

        // Shutdown now should abort the job
        pool.shutdown_now()?;

        // Should not be able to add more jobs
        let result = pool.execute(|| Ok(()));
        assert!(result.is_err());

        // Join should not hang and return stats
        let stats = pool.join()?;

        // We can't assert on exact stats since the job might or might not have completed
        // before shutdown_now, but we can assert that join succeeded
        assert!(stats.total_jobs_completed <= 1);

        Ok(())
    }

    #[test]
    fn test_thread_pool_stats() -> Result<()> {
        let pool = default_pool("stats-pool")?;

        // Execute multiple jobs with different sleep durations
        for i in 0..5 {
            let sleep_ms = 10 * (i + 1);
            pool.execute(move || {
                std_thread::sleep(Duration::from_millis(sleep_ms));
                Ok(())
            })?;
        }

        // Add one failing job
        pool.execute(|| {
            anyhow::bail!("Test failure");
        })?;

        // Wait for all jobs to complete
        pool.wait_for_completion()?;

        let stats = pool.stats();
        assert_eq!(stats.total_jobs_completed, 6);
        assert_eq!(stats.failed_jobs, 1);
        assert!(stats.avg_processing_time.is_some());

        // Check other stats methods
        assert_eq!(pool.completed_job_count(), 6);
        assert_eq!(pool.queued_job_count(), 0);
        assert!(!pool.is_shutting_down());

        // Cleanup
        pool.shutdown()?;

        Ok(())
    }

    #[test]
    fn test_thread_config_validation() -> Result<()> {
        // Valid config should pass validation
        let config = Config::default();
        assert!(config.validate().is_ok());

        // Zero max_threads should fail
        let invalid_max = Config {
            max_threads: 0,
            ..Default::default()
        };
        assert!(invalid_max.validate().is_err());

        // Too small stack size should fail
        let invalid_stack = Config {
            stack_size_bytes: 1024, // Too small (< 64KB)
            ..Default::default()
        };
        assert!(invalid_stack.validate().is_err());

        // Invalid core allocation should fail
        let invalid_cores = Config {
            core_allocation: CoreAllocation::PinnedCores { min: 5, max: 3 },
            ..Default::default()
        };
        assert!(invalid_cores.validate().is_err());

        Ok(())
    }

    #[test]
    fn test_thread_manager_panic_handling() -> Result<()> {
        let manager = default_manager("panic-test")?;

        let handle = manager.spawn(|| {
            if true {
                panic!("Test panic");
            }
            "should not reach here"
        });

        assert!(handle.is_ok());
        let handle = handle.unwrap();

        let result = handle.join();
        assert!(result.is_err());

        // Ensure manager is still usable after a thread panic
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        let handle = manager.spawn(move || {
            counter_clone.fetch_add(1, Ordering::SeqCst);
            "success after panic"
        })?;

        let result = handle.join().unwrap();
        assert_eq!(result, "success after panic");
        assert_eq!(counter.load(Ordering::SeqCst), 1);

        Ok(())
    }

    #[test]
    fn test_thread_pool_concurrent_stress() -> Result<()> {
        let pool = default_pool("stress-pool")?;

        let counter = Arc::new(AtomicUsize::new(0));
        let total_jobs = 100;

        // Submit many jobs concurrently
        for _ in 0..total_jobs {
            let counter = counter.clone();
            pool.execute(move || {
                // Simulate some work with random duration
                let sleep_ms = (std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().subsec_nanos() % 10) as u64;
                std_thread::sleep(Duration::from_millis(sleep_ms));
                counter.fetch_add(1, Ordering::SeqCst);
                Ok(())
            })?;
        }

        // Wait for all jobs to complete
        pool.wait_for_completion()?;

        assert_eq!(counter.load(Ordering::SeqCst), total_jobs);
        assert_eq!(pool.completed_job_count(), total_jobs);

        // Cleanup
        pool.shutdown()?;

        Ok(())
    }
}
