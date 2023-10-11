use crate::timer::{TimeoutVal, TimerError, countdown, timeout_to_duration};
use crate::process::buffer::{Buffer, BufData};
use crate::context;

use std::sync::{Mutex, Arc};
use std::time::Duration;
use std::thread;

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

fn context_timeout() -> TimeoutVal {
    context::access(|ctx| {
        ctx.timeout
    })
}

pub trait Tube : Clone {
    fn buffer(&mut self) -> &mut Buffer;

    fn recv_raw(&mut self, numb: usize, timeout: Duration)
                -> Result<Vec<u8>, TubesError>;

    fn send_raw(&mut self, data: Vec<u8>, timeout: Duration)
                -> Result<(), TubesError>;

    fn _fill_buffer(&mut self, timeout: TimeoutVal) -> Result<(), TubesError> {
        let duration = timeout_to_duration(timeout);
        let buf = self.recv_raw(0, duration)?;
        self.buffer().add(&mut BufData::ByteVec(buf));
        Ok(())
    }

    fn _recv(&mut self, numb: Option<usize>, timeout: TimeoutVal)
             -> Result<Vec<u8>, TubesError> {
        let numb = self.buffer().get_fill_size(numb);

        if numb > self.buffer().len() {
            self._fill_buffer(timeout)?;
        }

        Ok(self.buffer().get(numb))
    }

    fn _send(&mut self, data: Vec<u8>, timeout: TimeoutVal)
             -> Result<(), TubesError> {
        let duration = match timeout {
            TimeoutVal::Duration(duration) => duration,
            TimeoutVal::Default => Duration::from_secs(1),
            TimeoutVal::Forever => Duration::MAX
        };
        self.send_raw(data, duration)
    }

    fn recv_timeout(&mut self, numb: usize, timeout: TimeoutVal)
            -> Result<Vec<u8>, TubesError> {
        let numb = self.buffer().get_fill_size(Some(numb));
        return self._recv(Some(numb), timeout)
    }

    fn recv(&mut self, numb: usize) -> Result<Vec<u8>, TubesError> {
        return self.recv_timeout(numb, context_timeout())
    }

    fn recvuntil_timeout(&mut self, needle: Vec<u8>, timeout: TimeoutVal)
            -> Result<Vec<u8>, TubesError> {
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
                if *lock == false {
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

            data.append(&mut self._recv(None, timeout)?);
        }
    }

    fn recvuntil(&mut self, needle: Vec<u8>) -> Result<Vec<u8>, TubesError> {
        self.recvuntil_timeout(needle, context_timeout())
    }

    fn recvline_timeout(&mut self, timeout: TimeoutVal)
            -> Result<Vec<u8>, TubesError> {
        self.recvuntil_timeout("\n".into(), timeout)
    }

    fn recvline(&mut self) -> Result<Vec<u8>, TubesError> {
        self.recvuntil("\n".into())
    }

    fn send_timeout(&mut self, data: Vec<u8>, timeout: TimeoutVal)
            -> Result<(), TubesError> {
        self._send(data, timeout)
    }

    fn send(&mut self, data: Vec<u8>) -> Result<(), TubesError> {
        self.send_timeout(data, context_timeout())
    }

    fn sendline_timeout(&mut self, data: Vec<u8>, timeout: TimeoutVal)
            -> Result<(), TubesError> {
        let mut data = data.clone();
        data.push('\n' as u8);
        self._send(data, timeout)
    }

    fn sendline(&mut self, data: Vec<u8>) -> Result<(), TubesError> {
        self.sendline_timeout(data, context_timeout())
    }

    // FIXME: this can technically do 2x timeout
    fn sendafter_timeout(&mut self, needle: Vec<u8>, data: Vec<u8>,
                         timeout: TimeoutVal) -> Result<(), TubesError> {
        self.recvuntil_timeout(needle, timeout)?;
        self.send_timeout(data, timeout)?;
        Ok(())
    }

    fn sendafter(&mut self, needle: Vec<u8>, data: Vec<u8>)
                 -> Result<(), TubesError> {
        self.sendafter_timeout(needle, data, context_timeout())
    }

    // FIXME: this can technically do 2x timeout
    fn sendlineafter_timeout(&mut self, needle: Vec<u8>, data: Vec<u8>,
                             timeout: TimeoutVal) -> Result<(), TubesError> {
        self.recvuntil_timeout(needle, timeout)?;
        self.sendline_timeout(data, timeout)?;
        Ok(())
    }

    fn sendlineafter(&mut self, needle: Vec<u8>, data: Vec<u8>)
                     -> Result<(), TubesError> {
        self.sendlineafter_timeout(needle, data, context_timeout())
    }

    fn interactive(&mut self) -> Result<(),TubesError> where Self: 'static + Send {
        // spawn thread to print stdout to terminal
        let mut self_ref = self.clone();

        // synchronization for sender and receiver
        let cont = Arc::new(Mutex::new(true));
        let cont_recv = Arc::clone(&cont);

        let handle = thread::spawn(move || -> Result<(), TubesError> {
            loop {
                let recvd = self_ref._recv(None, context_timeout());
                match recvd {
                    Ok(data) => {
                        let data_str = unsafe {
                            String::from_utf8_unchecked(data)
                        };
                        print!("{}", data_str);
                    }
                    Err(TubesError::RecvError(err)) => {
                        *cont_recv.lock().unwrap() = false;
                        break Err(TubesError::RecvError(err))
                    },
                    _ => {}
                }
                thread::sleep(Duration::from_millis(200));
                if !*cont_recv.lock().unwrap() {
                    break Ok(())
                }
            }
        });

        // TODO: implement crossterm or termion to have more control over
        //       the input prompt

        // receive lines from stdin and send to child
        let mut rl = rustyline::DefaultEditor::new()?;
        let send_result: Result<(), TubesError> = loop {
            let input = rl.readline("");
            if let Ok(input) = input {
                let res = self.sendline_timeout(input.into(), TimeoutVal::Default);
                match &res {
                    Err(TubesError::StdIOError(_)) => {
                        break res
                    }
                    _ => { }
                }
            } else if let Err(input) = input {
                break Err(TubesError::ReadlineError(input))
            }
        };

        handle.join().unwrap()?;
        send_result?;

        Ok(())
    }
}
