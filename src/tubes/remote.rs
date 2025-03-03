use tokio::net::{TcpStream, ToSocketAddrs};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::Mutex;

use std::sync::Arc;

use crate::tubes::{Tube, TubesError};
use crate::tubes::buffer::Buffer;
use crate::logging as log;


#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Connection Error: {0}")]
    ConnectionError(String),

    #[error("Stdio Error")]
    StdIOError(#[from] std::io::Error),

    #[error("Recv Error: {0}")]
    RecvError(String),

    #[error("Send Error: {0}")]
    SendError(String),
}

//#[derive(Debug, Clone)]
//pub struct IO {
//    pub conn:  Arc<Mutex<TcpStream>>,
//}
//
//impl IO {
//    pub fn new(conn: TcpStream) -> IO {
//        IO { conn: Arc::new(Mutex::new(conn)) }
//    }
//}

pub struct ProcessConfig {
    pub aslr: bool
}

impl Default for ProcessConfig {
    fn default() -> Self {
        ProcessConfig { aslr: true }
    }
}

#[derive(Debug, Clone)]
pub struct Remote {
    pub conn: Arc<Mutex<TcpStream>>,
    buffer: Buffer,
    //pub io: IO,
}

// TODO: implement builder pattern for initializing processes
//       it'd be much cleaner for being able to specify options and config
//       overrides alternative is to expand ProcessConfig to hold stuff like
//       env-vars which seems messy

impl Remote {
    pub async fn new<Addr: ToSocketAddrs + std::fmt::Display>(
        addr: Addr,
    ) -> Result<Remote, Error> {

        log::info(format!("Establishing remote connection to '{}'", &addr));

        let stream = TcpStream::connect(&addr).await.map_err(|e| Error::ConnectionError(e.to_string()))?;

        let sync_stream = Arc::new(Mutex::new(stream));

        Ok(Remote {
            conn: sync_stream,
            buffer: Buffer::default(),
        })
    }

}

impl Tube for Remote {
    fn buffer(&mut self) -> &mut Buffer {
        &mut self.buffer
    }

    async fn recv_raw(&mut self, _numb: usize, duration: std::time::Duration)
    -> Result<Vec<u8>, TubesError> {
        let mut buf = vec![];

        let _ = tokio::time::timeout(
            duration,
            self.conn.lock().await.read_buf(&mut buf)
        ).await;

        Ok(buf.to_vec())
    }

    async fn send_raw(&mut self, data: &[u8], duration: std::time::Duration)
    -> Result<(), TubesError> {
        let _ = tokio::time::timeout(duration, (self.conn.lock().await).write_all(data)).await;
        Ok(())
    }

}
