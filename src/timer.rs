use std::sync::{Mutex, Arc};
use std::time::Duration;
use tokio::time::timeout;

#[derive(thiserror::Error, Debug)]
pub enum TimerError {
    #[error("Timeout Error")]
    TimeoutError,

    #[error("Disconnect Error")]
    DisconnectError,

    #[error("Stdio Error")]
    StdIOError(#[from] std::io::Error),
}

#[derive(Clone, Copy, Debug)]
#[derive(Default)]
pub enum TimeoutVal {
    #[default]
    Default,
    Forever,
    Duration(Duration),
}


pub fn timeout_to_duration(timeout: TimeoutVal) -> Duration {
    
    match timeout {
        TimeoutVal::Duration(duration) => duration,
        TimeoutVal::Default => Duration::from_millis(100),
        TimeoutVal::Forever => Duration::MAX
    }
}

pub fn countdown(duration: Duration, lock: &Arc<Mutex<bool>>) {
    let timer = timer::Timer::new();
    let duration = chrono::Duration::from_std(duration).unwrap();

    if let Ok(mut lock) = Arc::clone(lock).lock() {
        *lock = true;
    }

    let lock_ref = Arc::clone(lock);
    let _guard = timer.schedule_with_delay(duration, move || {
        if let Ok(mut lock) = lock_ref.lock() {
            *lock = false;
        }
    });
}

pub async fn run_with_timeout<T, F>(f: F, duration: Duration) -> Result<T, TimerError>
where
    T: Send + 'static,
    F: FnOnce() -> T,
    F: Send + 'static,
{

    // Use the runtime to block on the task with a timeout
    let task_result = tokio::spawn(timeout(duration, tokio::task::spawn_blocking(f)));

    match task_result.await.unwrap() {
        Ok(a) => { Ok(a.expect("Join Error")) }
        Err(_) => { Err(TimerError::TimeoutError) }
    }
}
