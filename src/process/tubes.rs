use crate::timer::{TimeoutVal, TimerError, countdown, timeout_to_duration};
use crate::process::buffer::{Buffer, BufData};
use crate::context;

use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::future::Future;

#[derive(thiserror::Error, Debug)]
pub enum TubesError {
    #[error("Stdio Error")]
    StdIOError(#[from] std::io::Error),

    #[error("Timer Error")]
    TimerError(#[from] TimerError),

    #[error("Receive Error")]
    RecvError(String),

    #[error("Readline Error")]
    ReadlineError(#[from] rustyline::error::ReadlineError),
}

pub fn context_timeout() -> TimeoutVal {
    context::access(|ctx| {
        ctx.timeout
    })
}

pub trait Tube : Clone + Send where Self: 'static {
    fn buffer(&mut self) -> &mut Buffer;

    fn recv_raw(
        &mut self,
        numb: usize,
        timeout: Duration
    ) -> impl Future<Output = Result<Vec<u8>, TubesError>> + Send;

    fn send_raw(&mut self, data: Vec<u8>, timeout: Duration)
    -> impl Future<Output = Result<(), TubesError>> + Send;

    fn _fill_buffer(&mut self, timeout: TimeoutVal)
    -> impl Future<Output = Result<(), TubesError>> + Send {
        async move {
            let duration = timeout_to_duration(timeout);
            let buf = self.recv_raw(0, duration).await?;
            self.buffer().add(&mut BufData::ByteVec(buf));
            Ok(())
        }
    }

    fn _recv(&mut self, numb: Option<usize>, timeout: TimeoutVal)
    -> impl Future<Output = Result<Vec<u8>, TubesError>> + Send {
        async move {
            let numb = self.buffer().get_fill_size(numb);

            if numb > self.buffer().len() {
                self._fill_buffer(timeout).await?;
            }

            Ok(self.buffer().get(numb))
        }
    }

    fn _send(&mut self, data: Vec<u8>, timeout: TimeoutVal)
    -> impl Future<Output = Result<(), TubesError>> + Send {
        async move {
            let duration = match timeout {
                TimeoutVal::Duration(duration) => duration,
                TimeoutVal::Default => Duration::from_secs(1),
                TimeoutVal::Forever => Duration::MAX
            };
            self.send_raw(data, duration).await
        }
    }

    fn recv_timeout(&mut self, numb: usize, timeout: TimeoutVal)
    -> impl Future<Output = Result<Vec<u8>, TubesError>> + Send {
        async move {
            let numb = self.buffer().get_fill_size(Some(numb));
            return self._recv(Some(numb), timeout).await
        }
    }

    fn recv(&mut self, numb: usize)
    -> impl Future<Output = Result<Vec<u8>, TubesError>> + Send {
        async move {
            return self.recv_timeout(numb, context_timeout()).await
        }
    }

    fn recvuntil_timeout(&mut self, needle: Vec<u8>, timeout: TimeoutVal)
    -> impl Future<Output = Result<Vec<u8>, TubesError>> + Send {
        async move {
            let mut data: Vec<u8> = vec![];

            // receive what we have buffered case it lets us exit without having to
            // invoke _recv
            let buf = self.buffer();
            data.append(&mut buf.get(buf.len()));

            let lock = Arc::new(Mutex::new(false));
            let duration = timeout_to_duration(timeout);

            // could be cleaner, countdown will set lock to true for duration
            countdown(duration, &lock);

            loop {
                if let Ok(lock) = Arc::clone(&lock).lock() {
                    if !(*lock) {
                        self.buffer().unget(BufData::ByteVec(data));
                        let err = TubesError::TimerError( TimerError::TimeoutError);
                        return Err(err);
                    }
                }

                // check if needle is in data
                let idx = data.windows(needle.len())
                              .position(|window| window == needle.as_slice());
                if let Some(idx) = idx {
                    let res_data = data.drain(0..idx+needle.len()).collect();
                    self.buffer().unget(BufData::ByteVec(data.clone()));
                    return Ok(res_data);
                }

                data.append(&mut self._recv(None, timeout).await?);
            }
        }
    }

    fn recvuntil(&mut self, needle: Vec<u8>)
    -> impl Future<Output = Result<Vec<u8>, TubesError>> + Send {
        async move {
            self.recvuntil_timeout(needle, context_timeout()).await
        }
    }

    async fn recvline_timeout(&mut self, timeout: TimeoutVal)
            -> Result<Vec<u8>, TubesError> {
        self.recvuntil_timeout("\n".into(), timeout).await
    }

    async fn recvline(&mut self) -> Result<Vec<u8>, TubesError> {
        self.recvuntil("\n".into()).await
    }

    async fn send_timeout(&mut self, data: Vec<u8>, timeout: TimeoutVal)
    -> Result<(), TubesError> {
        self._send(data, timeout).await
    }

    async fn send(&mut self, data: Vec<u8>) -> Result<(), TubesError> {
        self.send_timeout(data, context_timeout()).await
    }

    async fn sendline_timeout(&mut self, data: Vec<u8>, timeout: TimeoutVal)
    -> Result<(), TubesError> {
        let mut data = data.clone();
        data.push('\n' as u8);
        self._send(data, timeout).await
    }

    async fn sendline(&mut self, data: Vec<u8>) -> Result<(), TubesError> {
        self.sendline_timeout(data, context_timeout()).await
    }

    // FIXME: this can technically do 2x timeout
    async fn sendafter_timeout(&mut self, needle: Vec<u8>, data: Vec<u8>,
                         timeout: TimeoutVal) -> Result<(), TubesError> {
        self.recvuntil_timeout(needle, timeout).await?;
        self.send_timeout(data, timeout).await?;
        Ok(())
    }

    async fn sendafter(&mut self, needle: Vec<u8>, data: Vec<u8>)
    -> Result<(), TubesError> {
        self.sendafter_timeout(needle, data, context_timeout()).await
    }

    // FIXME: this can technically do 2x timeout
    async fn sendlineafter_timeout(&mut self, needle: Vec<u8>, data: Vec<u8>,
                             timeout: TimeoutVal) -> Result<(), TubesError> {
        self.recvuntil_timeout(needle, timeout).await?;
        self.sendline_timeout(data, timeout).await?;
        Ok(())
    }

    async fn sendlineafter(&mut self, needle: Vec<u8>, data: Vec<u8>)
    -> Result<(), TubesError> {
        self.sendlineafter_timeout(needle, data, context_timeout()).await
    }

    async fn interactive(&mut self) -> Result<(), TubesError> {
        // spawn thread to print stdout to terminal
        //let mut self_ref = self.clone();

        // synchronization for sender and receiver
        let self_ref = Arc::new(tokio::sync::Mutex::new(self.clone()));
        let cont = Arc::new(tokio::sync::Mutex::new(true));


        let cont_copy = Arc::clone(&cont);
        let self_ref_copy = Arc::clone(&self_ref);

        let _handle = tokio::spawn(test(self_ref_copy, cont_copy));

        // TODO: implement crossterm or termion to have more control over
        //       the input prompt

        // receive lines from stdin and send to child
        let mut rl = rustyline::DefaultEditor::new()?;
        let send_result: Result<(), TubesError> = loop {
            let input = rl.readline("");
            if let Ok(input) = input {
                let res = self_ref.lock().await.sendline_timeout(input.into(), crate::timer::TimeoutVal::Default).await;
                if let Err(TubesError::StdIOError(_)) = &res {
                    break res
                }
            } else if let Err(input) = input {
                *cont.lock().await = false;
                break Err(TubesError::ReadlineError(input))
            }
        };

        //handle.join().unwrap()?;
        send_result?;

        Ok(())
    }
}


async fn test<T>(self_ref_copy: Arc<tokio::sync::Mutex<T>>, cont_copy: Arc<tokio::sync::Mutex<bool>>)
    -> Result<(), TubesError> where T: crate::process::tubes::Tube {
    loop {
        let recvd = self_ref_copy.lock().await._recv(
            None,
            crate::timer::TimeoutVal::Duration(std::time::Duration::from_millis(50))
        ).await;

        match recvd {
            Ok(data) => {
                let data_str = unsafe {
                    String::from_utf8_unchecked(data)
                };
                print!("{}", data_str);
            }
            Err(TubesError::RecvError(err)) => {
                *cont_copy.lock().await = false;
                break Err(TubesError::RecvError(err))
            },
            _ => {}
        }
        if !*cont_copy.lock().await {
            break Ok(())
        }
    }
}
