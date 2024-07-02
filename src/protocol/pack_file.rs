use crate::compressor::Compressor;
use crate::files::object_type::ObjectType;
use crate::git_errors::errors::ErrorType;
use crate::hash::GitHash;
use flate2::bufread::ZlibDecoder;
use std::io::{self, BufRead, Cursor, Read, Write};

const PACKFILE_HEADER: [u8; 4] = [80, 65, 67, 75]; //PACK
const PACKFILE_VERSION: [u8; 4] = [0, 0, 0, 2]; // 2

////////////////////////////////////////////////////////////////////////////////////////
// PACK FILE
////////////////////////////////////////////////////////////////////////////////////////

// external docs:
// https://code.googlesource.com/git/+/v2.8.2/Documentation/technical/pack-format.txt
// https://shafiul.github.io/gitbook/7_the_packfile.html

// header 12 Bytes
//     4 Bytes -> PACK
//     4 Bytes -> Version
//     4 Bytes -> #Objects
// packed data for object 0
// ...
// packed data for object N-1
// checksum 20 Bytes

// todo mover a otro lado
// Reads a fixed number of bytes from a stream.
// Rust's "const generics" make this function very useful.
pub fn read_bytes<R: Read, const N: usize>(stream: &mut R) -> io::Result<[u8; N]> {
    let mut bytes = [0; N];
    stream.read_exact(&mut bytes)?;
    Ok(bytes)
}

//////////////// CREATE ////////////////

pub fn send_packfile(
    stream: &mut impl Write,
    packfile_objects: Vec<(ObjectType, GitHash, Vec<u8>)>,
) -> Result<(), ErrorType> {
    let mut packfile = Vec::new();

    let header: Vec<u8> = [PACKFILE_HEADER, PACKFILE_VERSION].concat(); //header
    stream.write_all(&header)?;
    packfile.extend_from_slice(&header);

    let object_ammount: [u8; 4] = usize_to_bytes(packfile_objects.len())?;
    stream.write_all(&object_ammount)?;
    packfile.extend_from_slice(&object_ammount);

    for object in packfile_objects {
        let object_entry = build_packfile_object_entry(object)?;
        stream.write_all(&object_entry)?;
        packfile.extend_from_slice(&object_entry);
    }

    let checksum = GitHash::hash_sha1(&packfile);
    stream.write_all(&checksum.to_hex()?)?;

    Ok(())
}

pub fn build_packfile(
    packfile_objects: Vec<(ObjectType, GitHash, Vec<u8>)>,
) -> Result<Vec<u8>, ErrorType> {
    let mut packfile: Vec<u8> = [PACKFILE_HEADER, PACKFILE_VERSION].concat(); //header

    let object_ammount: [u8; 4] = usize_to_bytes(packfile_objects.len())?;

    packfile.write_all(&object_ammount)?;

    for object in packfile_objects {
        let object_entry = build_packfile_object_entry(object)?;
        packfile.write_all(&object_entry)?;
    }

    let checksum = GitHash::hash_sha1(&packfile);
    packfile.write_all(&checksum.to_hex()?)?;

    Ok(packfile)
}

fn usize_to_bytes(object_ammount: usize) -> Result<[u8; 4], ErrorType> {
    if object_ammount > u32::MAX as usize {
        return Err(ErrorType::ProtocolError(format!(
            "too many objects ({object_ammount}). packfile accepts up to 4k entries"
        )));
    }
    let result = (object_ammount as u32).to_be_bytes();
    Ok(result)
}

fn build_packfile_object_entry(
    object: (ObjectType, GitHash, Vec<u8>),
) -> Result<Vec<u8>, ErrorType> {
    let obj_type = object.0;
    let content = object.2;

    let type_byte: u8 = match obj_type {
        ObjectType::Commit => 1,
        ObjectType::Tree => 2,
        ObjectType::Blob => 3,
        // object_type::ObjectType::Tag => 64
    };

    let mut result = generate_packfile_object_header(type_byte, content.len())?; // header (type and size)

    let compressed_content = Compressor::compress(content)?;

    result.write_all(&compressed_content)?;

    Ok(result)
}

fn generate_packfile_object_header(typ: u8, size: usize) -> Result<Vec<u8>, ErrorType> {
    if typ > 7 {
        Err(ErrorType::ProtocolError(format!(
            "invalid object type number when generating packfile object header: {}",
            typ
        )))?;
    }
    let mut header_bytes = Vec::new();

    let mut first_byte = match size > 15 {
        true => 0b10000000,
        false => 0b00000000,
    };

    let type_bits = typ << 4;
    first_byte |= type_bits;

    let mut remaining_size = size;

    first_byte |= (remaining_size & 0b1111) as u8;
    remaining_size >>= 4;

    while remaining_size > 0 {
        let mut byte = (remaining_size & 0b01111111) as u8;
        remaining_size >>= 7;

        if remaining_size > 0 {
            byte |= 0b10000000;
        }
        header_bytes.push(byte);
    }
    header_bytes.insert(0, first_byte);

    Ok(header_bytes)
}

//////////////// END CREATE ////////////////

//////////////// READ ////////////////

// pub fn read_to_end(stream: &mut impl Read) -> Result<Vec<u8>, ErrorType> {
//     const BUFFER_SIZE: usize = 1024;
//     let mut result = Vec::new();
//     let mut buffer = [0; BUFFER_SIZE];

//     loop {
//         let bytes_read = stream.read(&mut buffer)?;

//         if bytes_read == 0 {
//             // Se alcanzó el final del stream
//             break;
//         }

//         result.extend_from_slice(&buffer[..bytes_read]);
//     }

//     Ok(result)
// }

pub fn read_packfile<R: BufRead>(stream: &mut R) -> Result<Vec<(ObjectType, Vec<u8>)>, ErrorType> {
    let mut full_content = Vec::new();

    let header: [u8; 4] = read_bytes(stream)?;
    if header != PACKFILE_HEADER {
        return Err(ErrorType::ProtocolError(format!(
            "invalid packfile header {:?}, expected {:?}  (PACK)",
            header, PACKFILE_HEADER
        )));
    }
    full_content.extend_from_slice(&header);

    let version: [u8; 4] = read_bytes(stream)?;

    if version != PACKFILE_VERSION {
        return Err(ErrorType::ProtocolError(format!(
            "invalid packfile version {:?}, expected {:?} (2)",
            version, PACKFILE_VERSION
        )));
    }
    full_content.extend_from_slice(&version);

    let object_ammount_bytes: [u8; 4] = read_bytes(stream)?;
    let object_amount = u32::from_be_bytes(object_ammount_bytes);

    full_content.extend_from_slice(&object_ammount_bytes);

    let mut packfile_objects = Vec::new();

    for _ in 0..object_amount {
        let (object_type, uncompressed_content, compressed_content) = read_pack_object(stream)?;
        full_content.extend_from_slice(&compressed_content);
        packfile_objects.push((object_type, uncompressed_content));
    }

    let checksum: [u8; 20] = read_bytes(stream)?;

    let _checksum = GitHash::from_hex(&checksum)?;
    let _hash = GitHash::hash_sha1(&full_content);

    //TODO :  VERIFY CHECKSUM (posible solution below)

    // this failed because aparently uncompressing and then recompressing with zlib might not be deterministic
    // (a few bytes are changed. tested by calling read_to_end instead and then parsing)
    // possible solution below
    // if checksum != hash {
    //     return Err(ErrorType::ProtocolError(format!("packfile checksum ({checksum}) didn't match its content checksum ({hash})")))
    // }

    Ok(packfile_objects)
}

// solucion al checksum

// entro a la funcion de leer objeto
// leo con stream.take(tamanio descomprimido)
//     guardo lo leido como bytes originales
//     con lo leido descomprimo hasta llenar el tamanio descomprimido
//         si no llena -> packfile corrupto
//     si lee bien muy probablemente el take haya leido de más
//     entonces tengo que guardar esos extras y ponerlos en la proxima lectura
//     (afuera de esta funcion) antes de seguir leyendo del stream
//
// Entonces tengo que armar struct wrapper horrible que tenga el stream y
// una cola y que lea de la cola si tiene elementos y sino del stream?

fn _parse_packfile_header(reader: &mut Cursor<Vec<u8>>) -> Result<u32, ErrorType> {
    let header: [u8; 4] = read_bytes(reader)?;
    if header != PACKFILE_HEADER {
        return Err(ErrorType::ProtocolError(format!(
            "invalid packfile header {:?}, expected {:?}  (PACK)",
            header, PACKFILE_HEADER
        )));
    }
    let version: [u8; 4] = read_bytes(reader)?;
    if version != PACKFILE_VERSION {
        return Err(ErrorType::ProtocolError(format!(
            "invalid packfile version {:?}, expected {:?} (2)",
            version, PACKFILE_VERSION
        )));
    }
    let object_ammount_bytes: [u8; 4] = read_bytes(reader)?;
    Ok(u32::from_be_bytes(object_ammount_bytes))
}

type UncompressedObject = Vec<u8>;
type CompressedBytes = Vec<u8>;
fn read_pack_object<R: BufRead>(
    stream: &mut R,
) -> Result<(ObjectType, UncompressedObject, CompressedBytes), ErrorType> {
    let mut header_bytes = Vec::new();
    loop {
        let [byte] = read_bytes(stream)?;
        header_bytes.push(byte);
        if byte & 0b10000000 == 0 {
            break; // first bit is 0, header ends here
        }
    }

    let (object_type, size) = parse_object_size_and_type(header_bytes.clone())?;

    let object_type = match object_type {
        1 => ObjectType::Commit,
        2 => ObjectType::Tree,
        3 => ObjectType::Blob,
        //   4 => Base(Tag),
        //   6 => OffsetDelta,
        //   7 => HashDelta,
        _ => {
            return Err(ErrorType::RepositoryError(format!(
                "Invalid object type: {}",
                object_type
            )))
        }
    };

    let mut uncompressed_object: Vec<u8> = vec![0; size];
    ZlibDecoder::new(stream).read_exact(&mut uncompressed_object)?;

    if uncompressed_object.len() != size {
        return Err(ErrorType::ProtocolError(
            "corrupt packfile: object size didn't match object read through stream".to_string(),
        ));
    }

    let compressed_bytes = [
        header_bytes,
        Compressor::compress(uncompressed_object.clone())?,
    ]
    .concat();

    Ok((object_type, uncompressed_object, compressed_bytes))
}

fn parse_object_size_and_type(mut object_header_bytes: Vec<u8>) -> Result<(u8, usize), ErrorType> {
    if object_header_bytes.is_empty() {
        return Err(ErrorType::ProtocolError(
            "corrupt packfile: invalid object header".to_string(),
        ));
    }
    let mut object_size: usize = 0;

    object_header_bytes.reverse();
    for byte in object_header_bytes
        .iter()
        .take(object_header_bytes.len() - 1)
    {
        let byte = byte & 0b01111111;
        object_size <<= 7;
        object_size |= byte as usize;
    }

    let first_byte = object_header_bytes.last().ok_or(ErrorType::ProtocolError(
        "corrupt packfile: invalid object header".to_string(),
    ))?;
    object_size <<= 4;
    object_size |= (first_byte & 0b1111) as usize;

    let object_type = (first_byte >> 4) & 0b111_u8;

    Ok((object_type, object_size))
}

///////  END READ //////////

#[cfg(test)]
mod tests_parse_packfile_object_header {
    use super::*;

    #[test]
    fn test_commit_type() {
        let input = vec![0b10010000, 0b00010010];
        let expected_output = (1, 288);
        assert_eq!(parse_object_size_and_type(input).unwrap(), expected_output);
    }

    #[test]
    fn test_tree_type() {
        let input = vec![0b10101000, 0b00001100];
        let expected_output = (2, 200);
        assert_eq!(parse_object_size_and_type(input).unwrap(), expected_output);
    }

    #[test]
    fn test_blob_type_1() {
        let input = vec![176, 4];
        let expected_output = (3, 64);
        assert_eq!(parse_object_size_and_type(input).unwrap(), expected_output);
    }

    #[test]
    fn test_blob_type_2() {
        let input = vec![180, 131, 1];
        let expected_output = (3, 2100);
        assert_eq!(parse_object_size_and_type(input).unwrap(), expected_output);
    }

    #[test]
    fn test_invalid_object_header() {
        let input = vec![];
        assert!(parse_object_size_and_type(input).is_err());
    }
}

#[cfg(test)]
mod tests_generate_packfile_object_header {
    use super::*;

    #[test]
    fn test_commit_header() -> Result<(), ErrorType> {
        let typ = 1; // Commit type
        let size = 288;

        let header_bytes = generate_packfile_object_header(typ, size)?;

        assert_eq!(header_bytes, vec![0b10010000, 0b00010010]);
        Ok(())
    }

    #[test]
    fn test_tree_header() -> Result<(), ErrorType> {
        let typ = 2; // Tree type
        let size = 200;

        let header_bytes = generate_packfile_object_header(typ, size)?;

        let expected_bytes: Vec<u8> = vec![0b10101000, 0b00001100];

        assert_eq!(header_bytes, expected_bytes);
        Ok(())
    }

    #[test]
    fn test_blob_header() -> Result<(), ErrorType> {
        let typ = 3; // Blob type
        let size = 64;

        let header_bytes = generate_packfile_object_header(typ, size)?;

        assert_eq!(header_bytes, vec![176, 4]);
        Ok(())
    }

    #[test]
    fn test_blob_header_3_bytes() -> Result<(), ErrorType> {
        let typ = 3; // Blob type
        let size = 2100;

        let header_bytes = generate_packfile_object_header(typ, size)?;

        assert_eq!(header_bytes, vec![180, 131, 1]);
        Ok(())
    }
}
