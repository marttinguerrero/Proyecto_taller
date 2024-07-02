use crate::git_errors::errors::ErrorType;
use flate2::read::ZlibDecoder;

use flate2::write::ZlibEncoder;
use std::io::{Read, Write};

pub(crate) struct Compressor;

impl Compressor {
    pub fn compress(content: Vec<u8>) -> Result<Vec<u8>, ErrorType> {
        let mut encoder = ZlibEncoder::new(Vec::new(), flate2::Compression::default());
        encoder.write_all(&content)?;
        Ok(encoder.finish()?)
    }

    pub fn uncompress<R: Read>(readable: R) -> Result<Vec<u8>, ErrorType> {
        let mut buffer = Vec::new();
        let mut decoder = ZlibDecoder::new(readable);
        decoder.read_to_end(&mut buffer)?;
        Ok(buffer)
    }
}
