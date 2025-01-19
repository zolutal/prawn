use lazy_static::lazy_static;
use prawn::process::*;

use tokio::sync::Mutex;
use std::sync::Arc;

lazy_static! {
    pub static ref PROC: Arc<Mutex<Option<Process>>> = Arc::new(Mutex::new(None));
}

#[macro_export]
macro_rules! p {
    () => {{
        PROC.lock().await.as_mut().expect("")
    }};
}

#[macro_export]
macro_rules! recv {
    ($num:expr) => {{
        let mut process = PROC.lock().await;
        if let Some(p) = &mut *process {
            p.recv($num).await
        } else {
            panic!("Process not initialized")
        }
    }};
}

#[macro_export]
macro_rules! recvuntil {
    ($data:expr) => {{
        let mut process = PROC.lock().await;
        if let Some(p) = &mut *process {
            p.recvuntil($data).await
        } else {
            panic!("Process not initialized")
        }
    }};
}

#[macro_export]
macro_rules! recvline {
    () => {{
        let mut process = PROC.lock().await;
        if let Some(p) = &mut *process {
            p.recvline().await
        } else {
            panic!("Process not initialized")
        }
    }};
}

#[macro_export]
macro_rules! send {
    ($data:expr) => {{
        let mut process = PROC.lock().await;
        if let Some(p) = &mut *process {
            p.send($data).await
        } else {
            panic!("Process not initialized")
        }
    }};
}

#[macro_export]
macro_rules! sendline {
    ($data:expr) => {{
        let mut process = PROC.lock().await;
        if let Some(p) = &mut *process {
            p.sendline($data).await
        } else {
            panic!("Process not initialized")
        }
    }};
}

#[macro_export]
macro_rules! sendlineafter {
    ($needle:expr, $data:expr) => {{
        let mut process = PROC.lock().await;
        if let Some(p) = &mut *process {
            p.sendlineafter($needle, $data).await
        } else {
            panic!("Process not initialized")
        }
    }};
}

