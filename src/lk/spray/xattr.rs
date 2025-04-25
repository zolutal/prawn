use std::ffi::CString;
use std::path::{Path, PathBuf};
use std::fs;

pub struct Xattr {
    path: PathBuf,
}

impl Xattr {
    pub fn new(path: &Path) -> Self {
        if !&path.exists() {
            if let Some(parent) = &path.parent() {
                if !parent.exists() {
                    fs::create_dir_all(parent).expect("Xattr failed to create dir for file");
                }
            }
            fs::File::create(&path).expect("Xattr could not create file");
        }
        Xattr { path: path.to_path_buf() }
    }

    pub fn setxattr(&self, name: &Vec<u8>, value: &Vec<u8>) {
        let path = CString::new(self.path.to_str().unwrap()).expect("CString");
        let name = CString::new(name.clone()).expect("CString");

        unsafe {
            let ret = libc::setxattr(
                path.as_ptr() as *const libc::c_char,
                name.as_ptr() as *const libc::c_char,
                value.as_ptr() as *const libc::c_void,
                value.len(),
                0
            );
            if ret == -1 {
                libc::perror("readlink\0".as_ptr() as *const libc::c_char);
                panic!("setxattr failed");
            }
        }
    }
}
