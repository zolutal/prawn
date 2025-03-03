#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Unpacking Error: {0}")]
    UnpackError(String),
}

pub fn u64(bytevec: &[u8]) -> u64 {
    u64::from_le_bytes(bytevec[0..8].try_into().unwrap())
}

pub fn u32(bytevec: &[u8]) -> u32 {
    u32::from_le_bytes(bytevec[0..4].try_into().unwrap())
}

pub fn u16(bytevec: &[u8]) -> u16 {
    u16::from_le_bytes(bytevec[0..2].try_into().unwrap())
}

pub fn u8(bytevec: &[u8]) -> u8 {
    bytevec[0]
}

pub fn u64_chk(bytevec: &[u8]) -> Result<u64, Error> {
    if bytevec.len() != 8 {
        return Err(Error::UnpackError("Expected slice of len 8!".into()));
    }
    Ok(u64(bytevec))
}

pub fn u32_chk(bytevec: &[u8]) -> Result<u32, Error> {
    if bytevec.len() != 4 {
        return Err(Error::UnpackError("Expected slice of len 4!".into()));
    }
    Ok(u32(bytevec))
}

pub fn u16_chk(bytevec: &[u8]) -> Result<u16, Error> {
    if bytevec.len() != 2 {
        return Err(Error::UnpackError("Expected slice of len 2!".into()));
    }
    Ok(u16(bytevec))
}

pub fn u8_chk(bytevec: &[u8]) -> Result<u8, Error> {
    if bytevec.len() != 1 {
        return Err(Error::UnpackError("Expected slice of len 1!".into()));
    }
    Ok(u8(bytevec))
}

pub fn p64(val: u64) -> Vec<u8> {
    val.to_le_bytes().to_vec()
}

pub fn p32(val: u32) -> Vec<u8> {
    val.to_le_bytes().to_vec()
}

pub fn p16(val: u16) -> Vec<u8> {
    val.to_le_bytes().to_vec()
}

pub fn p8(val: u8) -> Vec<u8> {
    val.to_le_bytes().to_vec()
}
