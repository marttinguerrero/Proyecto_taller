use crate::git_errors::errors::ErrorType;
use std::io::Read;

use super::pack_file::read_bytes;

/// given a text it returns the coresponding pktline (adds size and \n)
pub fn create_pkt_line(text: &str) -> Result<String, ErrorType> {
    let length = text.len() + 1;
    if length > u32::MAX as usize {
        return Err(ErrorType::ProtocolError(
            "pktline: can't be larger than 4k".to_string(),
        ));
    }
    Ok(format!("{:04x}{}\n", length + 4, text))
}

pub fn read_pkt_line<R: Read>(stream: &mut R) -> Result<Option<String>, ErrorType> {
    let size_bytes: [u8; 4] = match read_bytes(stream) {
        Ok(l) => l,
        Err(_) => {
            return Err(ErrorType::ProtocolError(
                "failed to read pktline".to_string(),
            ))
        }
    };

    if size_bytes == *b"0000" {
        // flush
        return Ok(None);
    }

    let line_size = pkt_line_size(size_bytes)?;

    Ok(Some(pkt_line_content(line_size, stream)?))
}

pub fn pkt_line_content<R: Read>(line_size: usize, stream: &mut R) -> Result<String, ErrorType> {
    let mut content: Vec<u8> = vec![0; line_size - 4];
    // -4 because the first size bytes are included
    stream.read_exact(&mut content)?;

    match String::from_utf8(content) {
        Ok(s) => Ok(s.trim_end().to_string()),
        Err(_) => Err(ErrorType::ProtocolError(
            "corrupt pktline: failed to parse to utf8".to_string(),
        )),
    }
}

pub fn pkt_line_size(size_bytes: [u8; 4]) -> Result<usize, ErrorType> {
    let line_size = match String::from_utf8(size_bytes.to_vec()) {
        Ok(s) => s,
        Err(_) => Err(ErrorType::ProtocolError(
            "corrupt pktline: failed to parse to utf8".to_string(),
        ))?,
    };
    let line_size = usize::from_str_radix(&line_size, 16)?;
    Ok(line_size)
}
