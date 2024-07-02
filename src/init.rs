use std::fs::{self, create_dir_all, File};

use crate::{git_errors::errors::ErrorType, repo_paths::RepoPaths};

pub fn git_init(paths: RepoPaths) -> Result<String, ErrorType> {
    // Verifica si el directorio ya es un repositorio Git
    if paths.get_home().join(".git-rustico/").exists() {
        return Err(ErrorType::RepositoryError(
            "Already a git-rustico repository".to_string(),
        ));
    }
    // Crea el directorio .git-rustico dentro de la carpeta especificada
    create_dir_all(paths.get_objects())?;
    create_dir_all(paths.get_refs_heads())?;

    // Crea el archivo HEAD
    fs::write(paths.get_head(), "master")?;

    // Crea el archivo index
    File::create(paths.get_index())?;

    File::create(paths.get_config())?;

    File::create(paths.get_remote())?;

    Ok("git-rustico repository succesfully created".to_string())
}

// #[cfg(test)]
// mod tests {
//     use super::*;

// para este test habria que agregar la funcionalidad de que reciba un path y
// que cree el repo en el path (creandolo si no existe) .
// queda como nice to have
//     #[test]
//     fn creacion_de_directorios() {
//         let carpeta = String::from("repo_prueba"); // cambiar "repo_prueba" por "test/repo_prueba"
//         assert!(git_init(carpeta.clone()).is_ok());
//         // existencia de carpetas
//         assert!(Path::new(&*format!("{}/{}", carpeta, SUB_PATH)).exists());
//         assert!(Path::new(&*format!("{}/{}/objects", carpeta, SUB_PATH)).exists());
//         assert!(Path::new(&*format!("{}/{}/refs", carpeta, SUB_PATH)).exists());
//         assert!(Path::new(&*format!("{}/{}/refs/heads", carpeta, SUB_PATH)).exists());
//         // existencia de archivos
//         assert!(Path::new(&*format!("{}/{}/HEAD", carpeta, SUB_PATH)).exists());
//         assert!(Path::new(&*format!("{}/{}/INDEX", carpeta, SUB_PATH)).exists());
//     }
// }
