use crate::timer::{TimeoutVal, TimerError, countdown, timeout_to_duration};
use crate::tubes::buffer::{Buffer, BufData};
use crate::context;

pub mod process;
pub mod remote;
pub mod buffer;

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

    fn send_raw(&mut self, data: &[u8], timeout: Duration)
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

    fn _send(&mut self, data: &[u8], timeout: TimeoutVal)
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

    fn recvuntil_timeout(&mut self, needle: &[u8], timeout: TimeoutVal)
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
                let idx = data.windows(
                    needle.len()
                ).position(|window| window == needle);

                if let Some(idx) = idx {
                    let res_data = data.drain(0..idx+needle.len()).collect();
                    self.buffer().unget(BufData::ByteVec(data.clone()));
                    return Ok(res_data);
                }

                data.append(&mut self._recv(None, timeout).await?);
            }
        }
    }

    fn recvuntil(&mut self, needle: &[u8])
    -> impl Future<Output = Result<Vec<u8>, TubesError>> + Send {
        async move {
            self.recvuntil_timeout(needle, context_timeout()).await
        }
    }

    fn recvline_timeout(&mut self, timeout: TimeoutVal)
    -> impl Future<Output = Result<Vec<u8>, TubesError>> + Send {
        async move {
            let mut buf = self.recvuntil_timeout(b"\n", timeout).await?;
            buf.pop();
            Ok(buf)
        }
    }

    fn recvline(&mut self)
    -> impl Future<Output = Result<Vec<u8>, TubesError>> + Send {
        async move {
            let mut buf = self.recvuntil(b"\n").await?;
            buf.pop();
            Ok(buf)
        }
    }

    fn send_timeout(&mut self, data: &[u8], timeout: TimeoutVal)
    -> impl Future<Output = Result<(), TubesError>> + Send {
        async move {
            self._send(data, timeout).await
        }
    }

    fn send(&mut self, data: &[u8])
    -> impl Future<Output = Result<(), TubesError>> + Send {
        async move {
            self.send_timeout(data, context_timeout()).await
        }
    }

    fn sendline_timeout(&mut self, data: &[u8], timeout: TimeoutVal)
    -> impl Future<Output = Result<(), TubesError>> + Send {
        async move {
            let mut data = data.to_vec();
            data.push(b'\n');
            self._send(&data, timeout).await
        }
    }

    fn sendline(&mut self, data: &[u8])
    -> impl Future<Output = Result<(), TubesError>> + Send {
        async move {
            self.sendline_timeout(data, context_timeout()).await
        }
    }

    // FIXME: this can technically do 2x timeout
    fn sendafter_timeout(
        &mut self,
        needle: &[u8],
        data: &[u8],
        timeout: TimeoutVal
    ) -> impl Future<Output = Result<(), TubesError>> + Send {
        async move {
            self.recvuntil_timeout(needle, timeout).await?;
            self.send_timeout(data, timeout).await?;
            Ok(())
        }
    }

    fn sendafter(&mut self, needle: &[u8], data: &[u8])
    -> impl Future<Output = Result<(), TubesError>> + Send {
        async move {
            self.sendafter_timeout(needle, data, context_timeout()).await
        }
    }

    // FIXME: this can technically do 2x timeout
    fn sendlineafter_timeout(
        &mut self,
        needle: &[u8],
        data: &[u8],
        timeout: TimeoutVal
    ) -> impl Future<Output = Result<(), TubesError>> + Send {
        async move {
            self.recvuntil_timeout(needle, timeout).await?;
            self.sendline_timeout(data, timeout).await?;
            Ok(())
        }
    }

    fn sendlineafter(&mut self, needle: &[u8], data: &[u8])
    -> impl Future<Output = Result<(), TubesError>> + Send {
        async move {
            self.sendlineafter_timeout(needle, data, context_timeout()).await
        }
    }

    fn interactive(&mut self)
    -> impl Future<Output = Result<(), TubesError>> + Send {
        async move {
            // Synchronization for sender and receiver
            let self_clone = Arc::new(tokio::sync::Mutex::new(self.clone()));
            let cont = Arc::new(tokio::sync::Mutex::new(true));

            let cont_ref = Arc::clone(&cont);
            let self_clone_ref = Arc::clone(&self_clone);

            // spawn thread to handle displaying program output
            let handle = tokio::spawn(interactive_out(self_clone_ref, cont_ref));

            // TODO: implement crossterm or termion to have more control over
            //       the input prompt

            // receive lines from stdin and send to child
            let mut rl = rustyline::DefaultEditor::new()?;
            let send_result: Result<(), TubesError> = loop {
                let input = rl.readline("");
                if let Ok(input) = input {
                    let res = self_clone.lock().await.sendline_timeout(
                        input.as_bytes(),
                        crate::timer::TimeoutVal::Default
                    ).await;
                    if let Err(TubesError::StdIOError(_)) = &res {
                        break res
                    }
                } else if let Err(input) = input {
                    *cont.lock().await = false;
                    break Err(TubesError::ReadlineError(input))
                }
            };

            send_result?;

            handle.await.expect("JoinError").expect("");

            Ok(())
        }
    }
}


async fn interactive_out<T>(self_ref_copy: Arc<tokio::sync::Mutex<T>>, cont_copy: Arc<tokio::sync::Mutex<bool>>)
    -> Result<(), TubesError> where T: Tube {
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
