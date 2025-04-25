pub struct MsgQueue {
    msgqid: i32,
}

#[derive(Debug)]
pub struct Msg {
    pub mtype: libc::c_long,
    pub mtext: Vec<u8>,
}

impl MsgQueue {
    pub fn new() -> Self {
        unsafe {
            Self {
                msgqid: libc::msgget(libc::IPC_PRIVATE, 0o666),
            }
        }
    }

    pub fn send(&self, msg: &Msg) {
        let total_size = std::mem::size_of::<libc::c_long>() + msg.mtext.len();
        let mut buffer: Vec<u8> = Vec::with_capacity(total_size);

        unsafe {
            buffer.extend_from_slice(&msg.mtype.to_ne_bytes());
            buffer.extend_from_slice(&msg.mtext);

            let res = libc::msgsnd(
                self.msgqid,
                buffer.as_ptr() as *const libc::c_void,
                msg.mtext.len(),
                0,
            );
            if res < 0 {
                panic!("msgsnd failed");
            }
        }
    }

    pub fn recv_internal(&self, len: usize, flags: i32) -> Msg {
        let total_size = std::mem::size_of::<libc::c_long>() + len;
        let buffer: Vec<u8> = vec![0; total_size];
        unsafe {
            let received_bytes = libc::msgrcv(
                self.msgqid,
                buffer.as_ptr() as *mut libc::c_void,
                len,
                0,
                flags,
            );
            if received_bytes < 0 {
                panic!("msgrcv failed");
            }

            // Extract mtype from the beginning of the buffer
            let mtype = libc::c_long::from_ne_bytes(
                buffer[..std::mem::size_of::<libc::c_long>()]
                    .try_into()
                    .unwrap(),
            );

            // Extract mtext starting right after mtype
            let mtext_start = std::mem::size_of::<libc::c_long>();
            let mtext = buffer[mtext_start..(mtext_start + received_bytes as usize)].to_vec();

            Msg { mtype, mtext }
        }
    }

    pub fn recv(&self, len: usize) -> Msg {
        self.recv_internal(len, 0)
    }

    pub fn recv_noerror(&self, len: usize) -> Msg {
        self.recv_internal(len, libc::MSG_NOERROR)
    }

    pub fn recv_copy(&self, len: usize) -> Msg {
        self.recv_internal(len, libc::IPC_NOWAIT | libc::MSG_COPY)
    }
}
