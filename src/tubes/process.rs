use tokio::process::{Child, Command, ChildStdin, ChildStdout, ChildStderr};
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::sync::Mutex;

use std::process::Stdio;
use std::sync::Arc;

use linux_personality::personality;

use crate::tubes::{Tube, TubesError};
use crate::tubes::buffer::Buffer;
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
    pub stdin:  Arc<Mutex<ChildStdin>>,
    pub stdout: Arc<Mutex<BufReader<ChildStdout>>>,
    pub stderr: Arc<Mutex<BufReader<ChildStderr>>>,
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
    buffer: Buffer,
    pub io: IO,
}

// TODO: implement builder pattern for initializing processes
//       it'd be much cleaner for being able to specify options and config
//       overrides alternative is to expand ProcessConfig to hold stuff like
//       env-vars which seems messy

impl Process {
    pub async fn new<T: AsRef<str>>(
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

        let handle: Child = cmd.spawn()?;

        log::info(format!("Starting local process '{}': pid {:?}", &args[0], handle.id()));

        let sync_handle = Arc::new(Mutex::new(handle));

        let stdin  = sync_handle.lock().await.stdin .take().unwrap();
        let stdout = sync_handle.lock().await.stdout.take().unwrap();
        let stderr = sync_handle.lock().await.stderr.take().unwrap();

        let sync_handle_ref = sync_handle.clone();
        tokio::spawn(
            async move {
                let status = sync_handle_ref.lock().await.wait().await
                        .expect("child process encountered an error");

                println!("child status was: {}", status);
            }
        );

        if !enable_aslr {
            if let Some(orig_personality) = orig_personality {
                personality(orig_personality)
                    .expect(
                        "Failed to reset personality after spawn!"
                    );
            }
        }

        // needs to be able to be shared between threads for timeouts
        let io = IO::new(stdin, stdout, stderr);

        Ok(Process {
            handle: sync_handle,
            buffer: Buffer::default(),
            io
        })
    }

}

impl Tube for Process {
    fn buffer(&mut self) -> &mut Buffer {
        &mut self.buffer
    }

    async fn recv_raw(&mut self, _numb: usize, duration: std::time::Duration)
    -> Result<Vec<u8>, TubesError> {
        let mut buf = vec![];

        let res = tokio::time::timeout(
            duration,
            self.io.stdout.lock().await.read_buf(&mut buf)
        ).await;


        if let Err(res) = res {
            if matches!(res, Elapsed) {
                return Ok(buf.to_vec())
            } else {
                unreachable!();
            }
        }

        Ok(buf.to_vec())
    }

    async fn send_raw(&mut self, data: Vec<u8>, duration: std::time::Duration)
    -> Result<(), TubesError> {
        let writer = Arc::clone(&self.io.stdin);
        let res = tokio::time::timeout(duration, (*writer.lock().await).write_all(&data)).await;
        if let Err(res) = res {
            if matches!(res,  Elapsed) {
                return Ok(())
            } else {
                unreachable!();
            }
        }

        Ok(())
    }

}
