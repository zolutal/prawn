use crossbeam_channel::{bounded, RecvTimeoutError};
use std::sync::{Mutex, Arc};
use std::time::Duration;
use std::thread;

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
pub enum TimeoutVal {
    Default,
    Forever,
    Duration(Duration),
}

impl Default for TimeoutVal {
    fn default() -> Self {
        TimeoutVal::Default
    }
}

pub fn timeout_to_duration(timeout: TimeoutVal) -> Duration {
    let duration = match timeout {
        TimeoutVal::Duration(duration) => duration,
        TimeoutVal::Default => Duration::from_secs(1),
        TimeoutVal::Forever => Duration::MAX
    };
    duration
}

pub fn countdown(duration: Duration, lock: &Arc<Mutex<bool>>) {
    let timer = timer::Timer::new();
    let duration = chrono::Duration::from_std(duration).unwrap();

    if let Ok(mut lock) = Arc::clone(&lock).lock() {
        *lock = true;
    }

    let lock_ref = Arc::clone(&lock);
    let _guard = timer.schedule_with_delay(duration, move || {
        if let Ok(mut lock) = lock_ref.lock() {
            *lock = false;
        }
    });
}

pub fn run_with_timeout<T, F>(f: F, duration: Duration) -> Result<T, TimerError>
where
    T: Send + 'static,
    F: FnOnce() -> T,
    F: Send + 'static,
{
    let (sender, receiver) = bounded(1);

    thread::spawn(move || -> Result<(), crossbeam_channel::SendError<T>> {
        let result = f();
        let res = sender.send(result);
        res?;
        Ok(())
    });

    match receiver.recv_timeout(duration) {
        Ok(msg) => {Ok(msg)},
        Err(RecvTimeoutError::Timeout) => Err(TimerError::TimeoutError),
        Err(RecvTimeoutError::Disconnected) => Err(TimerError::DisconnectError),
    }
}
