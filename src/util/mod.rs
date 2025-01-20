#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Unpacking Error: {0}")]
    UnpackError(String),

    #[error("Packing Error: {0}")]
    PackError(String),
}

pub fn u64(bytevec: &[u8]) -> Result<u64, Error> {
    if bytevec.len() != 8 {
        return Err(Error::UnpackError("Expected vector of len 8!".into()));
    }
    let mut val = 0u64;
    for i in 0..8 {
        val |= (bytevec[i] as u64) << (i*8);
    }

    Ok(val)
}

pub fn p64(val: u64) -> Result<Vec<u8>, Error> {
    let mut bytevec = vec![0; 8];
    for i in 0..8 {
        bytevec[i] = (val >> (i*8)) as u8;
    }

    Ok(bytevec)
}
