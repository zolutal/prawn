use std::sync::{Mutex, Arc};
use crate::logging;
use crate::timer;

#[derive(Default)]
pub struct Context {
    pub aslr: bool,
    pub log_level: logging::LogLevel,
    pub timeout: timer::TimeoutVal,
}

lazy_static::lazy_static!{
    static ref CONTEXT: Arc<Mutex<Context>> = Arc::new(Mutex::new(
                                                       Context::default()));
}

pub fn access<F, R>(f: F) -> R
where
    F: FnOnce(&mut Context) -> R,
{
    let mut guard = CONTEXT.lock().unwrap();
    f(&mut *guard)
}
