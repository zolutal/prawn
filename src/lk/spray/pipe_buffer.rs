/// Represents the Linux pipe resource
pub struct Pipe {
    pub fd_read: i32,
    pub fd_write: i32
}

impl Pipe {
    /// Create a new pipe
    pub fn new() -> Self {
        let mut fds = [0; 2];
        unsafe {
            if libc::pipe(fds.as_mut_ptr()) == -1 {
                panic!("pipe failed");
            }
        }
        Self { fd_read: fds[0], fd_write: fds[1] }
    }

    /// Close the read end of the pipe
    pub fn close_read(&self) {
        unsafe {
            libc::close(self.fd_read);
        }
    }

    /// Close the write end of the pipe
    pub fn close_write(&self) {
        unsafe {
            libc::close(self.fd_write);
        }
    }

    /// Close both ends of the pipe
    pub fn close(&self) {
        self.close_read();
        self.close_write();
    }

    /// Close both ends of the pipe
    pub fn free(&self) {
        self.close();
    }

    /// Write data to the pipe
    ///
    /// Args:
    /// * `data` - A vector of bytes to write to the pipe
    pub fn write(&self, data: &Vec<u8>) {
        unsafe {
            let res = libc::write(
                self.fd_write,
                data.as_ptr() as *const libc::c_void,
                data.len()
            );
            if res < 0 {
                panic!("pipe write failed");
            }
        }
    }

    /// Read data from the pipe
    ///
    /// Args:
    /// * `length` - the number of bytes to read from the pipe
    ///
    /// Return:
    /// * A vector of bytes read from the pipe
    pub fn read(&self, length: usize) -> Vec<u8> {
        let mut buffer = vec![0;length];
        unsafe {
            let res = libc::read(
                self.fd_write,
                buffer.as_mut_ptr() as *mut libc::c_void,
                length
            );
            if res < 0 {
                panic!("pipe read failed");
            }
        }
        buffer
    }

    /// Set the capacity of the pipe, affects the size of the allocation
    /// associated with the pipe_buffer structs of this pipe
    ///
    /// Args:
    /// * `capacity` - The size in bytes to set the pipe capacity to
    pub fn resize_ring(&self, capacity: usize) {
        unsafe {
            let res = libc::fcntl(self.fd_write, libc::F_SETPIPE_SZ, capacity);
            if res < 0 {
                panic!("fcntl F_SETPIPE_SZ failed");
            }
        }
    }
}

/// Struct for managing a pipe_buffer heap spray
pub struct PipeBufferSpray {
    pub pipes: Vec<Pipe>
}

impl PipeBufferSpray {
    /// Initialize a for managing a pipe_buffer heap spray
    ///
    /// Args:
    /// * `count` - the number of pipe buffer structs to be allocated
    pub fn new(count: usize) -> Self {
        let mut pipes: Vec<Pipe> = vec![];
        for _ in 0..count {
            pipes.push(Pipe::new());
        }
        Self { pipes }
    }

    /// Write data to all pipes, populates the pipe_buffer structs
    ///
    /// Args:
    /// * `data` - the data to write to all of the pipes
    pub fn write_to_all(&self, data: &Vec<u8>) {
        for pipe in &self.pipes {
            pipe.write(&data);
        }
    }

    /// Closes all pipes
    pub fn close_all(&self) {
        for pipe in &self.pipes {
            pipe.close();
        }
    }

    /// Resize all pipes, can change which slab the pipe_buffers are allocated in
    ///
    /// Args:
    /// * `capacity` - the capacity in bytes to resize the pipes to
    pub fn resize_all(&self, capacity: usize) {
        for pipe in &self.pipes {
            pipe.resize_ring(capacity);
        }
    }
}
