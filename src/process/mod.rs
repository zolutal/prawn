pub mod buffer;
pub mod tubes;

use std::process::{Child, Command, ChildStdin, ChildStdout, ChildStderr, Stdio};
use std::io::{BufReader, BufRead, Write};
use std::sync::{Arc,Mutex};

use linux_personality::personality;

use crate::timer::run_with_timeout;
use crate::process::tubes::TubesError;
use crate::process::buffer::Buffer;
use crate::logging as log;
use crate::context;


#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Arugments Error: {0}")]
    ArgsError(String),

    #[error("Stdio Error")]
    StdIOError(#[from] std::io::Error),

    #[error("Recv Error: {0}")]
    RecvError(String),

    #[error("Send Error: {0}")]
    SendError(String),
}

#[derive(Debug, Clone)]
pub struct IO {
    stdin:  Arc<Mutex<ChildStdin>>,
    stdout: Arc<Mutex<BufReader<ChildStdout>>>,
    stderr: Arc<Mutex<BufReader<ChildStderr>>>,
}

impl IO {
    pub fn new(stdin:  ChildStdin, stdout: ChildStdout,
               stderr: ChildStderr) -> IO {

        let stdin = Arc::new(Mutex::new(stdin));
        let stdout = Arc::new(Mutex::new(BufReader::new(stdout)));
        let stderr = Arc::new(Mutex::new(BufReader::new(stderr)));

        IO { stdin, stdout, stderr }
    }
}

pub struct ProcessConfig {
    pub aslr: bool
}

impl Default for ProcessConfig {
    fn default() -> Self {
        ProcessConfig { aslr: true }
    }
}

#[derive(Debug, Clone)]
pub struct Process {
    pub handle: Arc<Mutex<Child>>,
    buffer: buffer::Buffer,
    pub io: IO,
}

// TODO: implement builder pattern for initializing processes
//       it'd be much cleaner for being able to specify options and config
//       overrides alternative is to expand ProcessConfig to hold stuff like
//       env-vars which seems messy

impl Process {
    pub fn new<T: AsRef<str>>(
        argv: impl AsRef<[T]>,
        _cfg: &ProcessConfig
    ) -> Result<Process, Error> {
        let args = argv.as_ref().iter().map(
            |x| String::from(x.as_ref())
        ).collect::<Vec<_>>();

        if args.is_empty() {
            return Err(Error::ArgsError(
                String::from("Process argv was empty!")
            ));
        }

        let enable_aslr = context::access(|ctx| {
            ctx.aslr
        });

        let orig_personality = {
            if !enable_aslr {
                let orig = linux_personality::get_personality().unwrap();
                personality(orig | linux_personality::ADDR_NO_RANDOMIZE)
                    .expect("Failed to disable ASLR");
                Some(orig)
            } else{
                None
            }
        };

        let mut cmd = Command::new(&args[0]);
        cmd.args(args.clone().into_iter().skip(1));

        cmd.stdin (Stdio::piped());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let mut handle: Child = cmd.spawn()?;

        log::info(format!("Starting local process '{}': pid {}", &args[0], handle.id()));

        if !enable_aslr {
            if let Some(orig_personality) = orig_personality {
                personality(orig_personality)
                    .expect(
                        "Failed to reset personality after spawn!"
                    );
            }
        }

        let stdin  = handle.stdin .take().unwrap();
        let stdout = handle.stdout.take().unwrap();
        let stderr = handle.stderr.take().unwrap();

        // needs to be able to be shared between threads for timeouts
        let io = IO::new(stdin, stdout, stderr);

        Ok(Process {
            handle: Arc::new(Mutex::new(handle)),
            buffer: Buffer::default(),
            io
        })
    }

}

impl tubes::Tube for Process {
    fn buffer(&mut self) -> &mut buffer::Buffer {
        &mut self.buffer
    }

    async fn recv_raw(&mut self, _numb: usize, timeout: std::time::Duration)
    -> Result<Vec<u8>, TubesError> {
        let reader = Arc::clone(&self.io.stdout);
        let received = run_with_timeout(move || {
            loop {
                let mut reader = reader.lock().unwrap();
                let buf = reader.fill_buf();
                let buf_vec = buf?.to_vec();
                if !buf_vec.is_empty() {
                    return Ok(buf_vec);
                }
            }
        }, timeout).await;

        // only consume afterwards, if we consume in the thread we are likely
        // to lose data
        let reader = Arc::clone(&self.io.stdout);
        //dbg!(&received);
        if let Ok(recv_result) = &received {
            if let Ok(buf_vec) = recv_result {
                reader.clone().lock().unwrap().consume(buf_vec.len());
            }
        } else if let Ok(mut handle) = self.handle.lock() {
            if let Ok(Some(exit_status)) = handle.try_wait() {
                //dbg!(exit_status);
                let msg = format!("Tried to receive but process had \
                                  already exited! status: {}", exit_status);
                return Err(TubesError::RecvError(msg))
            }
        }
        received?
    }

    async fn send_raw(&mut self, data: Vec<u8>, timeout: std::time::Duration)
    -> Result<(), TubesError> {
        let writer = Arc::clone(&self.io.stdin);
        let result = run_with_timeout(move || {
            if let Ok(mut writer) = writer.lock() {
                writer.write_all(&data)?;
            }
            Ok(())
        }, timeout).await;
        result?
    }

}
