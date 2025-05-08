use std::sync::atomic::{AtomicBool, Ordering};

use anyhow::Result;

static AFFINITY_WARNING_SHOWN: AtomicBool = AtomicBool::new(false);
static PRIORITY_WARNING_SHOWN: AtomicBool = AtomicBool::new(false);

pub fn set_affinity(cores: &[usize]) -> Result<()> {
    if cores.is_empty() {
        return Ok(());
    }
    #[cfg(target_os = "linux")]
    {
        affinity::set_affinity(cores).map_err(|e| {
            if !AFFINITY_WARNING_SHOWN.swap(true, Ordering::Relaxed) {
                eprintln!("Warning: Thread affinity operations may require elevated privileges.");
            }
            anyhow::anyhow!("Failed to set thread affinity: {}", e)
        })
    }
    #[cfg(not(target_os = "linux"))]
    {
        if !AFFINITY_WARNING_SHOWN.swap(true, Ordering::Relaxed) {
            eprintln!("Warning: Thread affinity is only supported on Linux platforms.");
        }
        return Ok(());
    }
}

pub fn set_priority(priority: u8) -> Result<()> {
    if priority == 0 {
        return Ok(());
    }
    #[cfg(target_os = "linux")]
    {
        let prio = thread_priority::ThreadPriority::Crossplatform(priority.into());
        let policy = thread_priority::ThreadSchedulePolicy::Normal(thread_priority::NormalThreadSchedulePolicy::Other);

        thread_priority::Thread::current().set_priority_and_policy(policy, prio).map_err(|e| {
            if !PRIORITY_WARNING_SHOWN.swap(true, Ordering::Relaxed) {
                eprintln!("Warning: Thread priority operations may require elevated privileges.");
            }
            anyhow::anyhow!("Failed to set thread priority: {:?}", e)
        })?;
    }
    #[cfg(not(target_os = "linux"))]
    {
        if !PRIORITY_WARNING_SHOWN.swap(true, Ordering::Relaxed) {
            eprintln!("Warning: Thread priority is only supported on Linux platforms.");
        }
    }
    return Ok(());
}
