use crate::git_errors::command_error::CommandError::{InvalidHash, UnknownOption};
use crate::git_errors::errors::ErrorType;
use flate2::read::ZlibDecoder;
use std::fs::File;
use std::io::Read;
use std::path::Path;

// TODO BORRAR

// todo path hardcodeado
const DIR_OF_OBJECTS_FILES: &str = ".git-rustico/objects";

/// Proporciona información de contenido o tipo y tamaño para los objetos de repositorio.
/// Dada una opcion y un hash retorna inforacion del archivo vinculado al hash.
/// La informacion depende de la opcion:
///     -p informacion del contenido del archivo.
///     -s tamaño en bytes del archivo.
///     -t typo de objeto que es el archivo.
/// La opcion a elegir es lowercase.
/// Caso en que no exista el archivo, la opcion sea incorrecta o falle la lectura del archivo
/// retornara error.
pub fn cat_file(option: &str, hash_oject: &str) -> Result<String, ErrorType> {
    if hash_oject.len() != 40 {
        return Err(ErrorType::CommandError(InvalidHash(
            "Hash length must be 40".to_string(),
        )));
    }
    match option {
        "-p" => Ok(cat_file_objects_content(hash_oject, DIR_OF_OBJECTS_FILES)?),
        "-s" => Ok(cat_file_object_size(hash_oject, DIR_OF_OBJECTS_FILES)?),
        "-t" => Ok(cat_file_object_type(hash_oject, DIR_OF_OBJECTS_FILES)?),
        _ => Err(ErrorType::CommandError(UnknownOption(
            "-p, -s or -t".to_string(),
            option.to_string(),
        ))),
    }
}

// TODO EL TREE NO ES LEGIBLE PQ TIENE BINARIO
/// Retorna el contenido basado en su tipo.
fn cat_file_objects_content(hash_oject: &str, directory: &str) -> Result<String, ErrorType> {
    let text = decoder_object(hash_oject, directory)?;
    let content = match text.split('\0').last() {
        None => {
            return Err(ErrorType::FormatError(
                "without separation by null character.".to_string(),
            ))
        }
        Some(a) => a,
    };
    Ok(content.to_owned())
}

/// Retorna el tamaño del objeto.
fn cat_file_object_size(hash_oject: &str, directory: &str) -> Result<String, ErrorType> {
    let text = decoder_object(hash_oject, directory)?;
    let size = match text.split('\0').next() {
        None => {
            return Err(ErrorType::FormatError(
                "without separation by null character.".to_string(),
            ))
        }
        Some(t) => match t.split_whitespace().last() {
            None => {
                return Err(ErrorType::FormatError(
                    "without separation by whitespace character.".to_string(),
                ))
            }
            Some(t) => t,
        },
    };
    Ok(size.to_string())
}

/// Retorna el tipo de archivo.
fn cat_file_object_type(hash_oject: &str, directory: &str) -> Result<String, ErrorType> {
    let text = decoder_object(hash_oject, directory)?;
    match text.split_whitespace().next() {
        None => Err(ErrorType::FormatError(
            "without separation by whitespace character.".to_string(),
        )),
        Some(t) => Ok(t.to_owned()),
    }
}

fn open_file_for_cat_file(hash_object: &str, directory: &str) -> Result<File, ErrorType> {
    let (dir, file) = hash_object.split_at(2);
    let path = format!("{}/{}/{}", directory, dir, file);
    if !Path::new(&path).exists() {
        return Err(ErrorType::CommandError(InvalidHash(format!(
            "{hash_object} not in objects"
        ))));
    }
    Ok(File::open(path)?)
}

fn read_decoder(file: &mut File) -> Result<String, ErrorType> {
    let mut buffer = String::from("");
    let mut decoder = ZlibDecoder::new(file);
    decoder.read_to_string(&mut buffer)?;
    Ok(buffer)
}

fn decoder_object(hash_oject: &str, directory: &str) -> Result<String, ErrorType> {
    let mut file = open_file_for_cat_file(hash_oject, directory)?;
    let text = read_decoder(&mut file)?;
    Ok(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    const DIRECTORIO_DE_ARCHIVOS_DE_PRUEBA: &str = "tests/tests_files/objects";

    #[test]
    fn cat_file_retorna_error_cuando_no_recibe_parametro_adecuado() {
        let letras: Vec<String> = (b'a'..=b'z')
            .map(|c| c as char)
            .filter(|c| c.is_alphabetic())
            .map(|c| format!("-{}", c))
            .collect();
        for letra in letras {
            if letra == "-p" || letra == "-s" || letra == "-t" {
                continue;
            }
            assert!(cat_file(letra.as_str(), "").is_err(), "falla en {}", letra);
        }
    }

    #[test]
    fn cat_file_abre_archivos_para_su_lectura() {
        assert!(open_file_for_cat_file(
            "10500012fca9b4425b50de67a7258a12cba0c076",
            DIRECTORIO_DE_ARCHIVOS_DE_PRUEBA
        )
        .is_ok())
    }

    #[test]
    fn cat_file_lee_y_decodifica_archivos() -> Result<(), ErrorType> {
        let mut archivo = open_file_for_cat_file(
            "10500012fca9b4425b50de67a7258a12cba0c076",
            DIRECTORIO_DE_ARCHIVOS_DE_PRUEBA,
        )?;
        let lectura = read_decoder(&mut archivo)?;

        // el archivo original solo contenia "asd" por lo que es de tamaño 3 y es un blob.
        // se utilizo el comando save del file info para crearlo.
        assert_eq!(lectura, "blob 3\0asd".to_string());
        Ok(())
    }

    #[test]
    fn cat_file_size() -> Result<(), ErrorType> {
        let size_file = cat_file_object_size(
            "10500012fca9b4425b50de67a7258a12cba0c076",
            DIRECTORIO_DE_ARCHIVOS_DE_PRUEBA,
        )?;
        assert_eq!(size_file, "3");
        Ok(())
    }

    #[test]
    fn cat_file_type() -> Result<(), ErrorType> {
        let type_file = cat_file_object_type(
            "10500012fca9b4425b50de67a7258a12cba0c076",
            DIRECTORIO_DE_ARCHIVOS_DE_PRUEBA,
        )?;
        assert_eq!(type_file, "blob");
        Ok(())
    }

    #[test]
    fn cat_file_content() -> Result<(), ErrorType> {
        let content_file = cat_file_objects_content(
            "10500012fca9b4425b50de67a7258a12cba0c076",
            DIRECTORIO_DE_ARCHIVOS_DE_PRUEBA,
        )?;
        assert_eq!(content_file, "asd");
        Ok(())
    }
}
