use crate::git_errors::errors::ErrorType;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

pub struct RepositoryAccessPermission {
    permit_map: HashMap<PathBuf, Arc<RwLock<bool>>>,
}

impl RepositoryAccessPermission {
    pub fn init_repository_access_permission() -> Self {
        RepositoryAccessPermission {
            permit_map: HashMap::new(),
        }
    }

    /// In the case where it is required to add a new repository to the permissions manager,
    /// it returns true.
    /// if it is not necessary add new repository false.
    pub fn requier_write_permision(&self, key: &PathBuf) -> bool {
        !self.permit_map.contains_key(key)
    }

    /// The execution of this function is done with a writing lock
    /// Add the address linked to a repository to the permissions to be granted
    /// and return said permission
    /// In the event that this function has been called by another thread with the same parameters,
    /// it will return the permission
    pub fn add_repository_to_permission(
        &mut self,
        key: PathBuf,
    ) -> Result<Arc<RwLock<bool>>, ErrorType> {
        match self.permit_map.get(&key) {
            None => {
                // caso normal donde el lock no existe para la direccion del repo
                let new_block = Arc::new(RwLock::new(true));
                self.permit_map.insert(key.clone(), new_block.clone());
                Ok(new_block)
            }
            Some(result) => {
                // caso borde donde otro hilo creo el lock para la direccion del repo
                Ok(result.clone())
            }
        }
    }

    /// get permission from repository
    pub fn new_permission_for_repository(
        &self,
        key: PathBuf,
    ) -> Result<Arc<RwLock<bool>>, ErrorType> {
        match self.permit_map.get(&key) {
            None => {
                // caso donde no existe la clave
                // se deberia haber agregado con "add_repository_to_permission"
                let path = match key.to_str() {
                    None => "/".to_string(),
                    Some(k) => k.to_string(),
                };
                Err(ErrorType::InvalidPath(format!(
                    "Not exist {} in the repository access permission.",
                    path
                )))
            }
            Some(result) => {
                // caso donde si existe la clave
                Ok(result.clone())
            }
        }
    }
}

pub fn get_permision_for_reposiory_from_repository_access_permission(
    repository_permission: &Arc<RwLock<RepositoryAccessPermission>>,
    repo_pathbuf: PathBuf,
) -> Result<Arc<RwLock<bool>>, ErrorType> {
    let read_repository_access_permision = match repository_permission.read() {
        Ok(guard_repo) => guard_repo,
        Err(_) => {
            return Err(ErrorType::ConfigError(
                "Error in deny read permissions in repository access permission.".to_string(),
            ));
        }
    };
    if read_repository_access_permision.requier_write_permision(&repo_pathbuf) {
        drop(read_repository_access_permision);
        let mut write_repository_access_permision = match repository_permission.write() {
            Ok(guard_repo) => guard_repo,
            Err(_) => {
                return Err(ErrorType::ConfigError(
                    "Error in deny add new repository in repository access permission.".to_string(),
                ));
            }
        };
        let result =
            write_repository_access_permision.add_repository_to_permission(repo_pathbuf.clone())?;
        drop(write_repository_access_permision); // drop inesesario mas halla de test manuales
        Ok(result)
    } else {
        let result =
            read_repository_access_permision.new_permission_for_repository(repo_pathbuf.clone())?;
        drop(read_repository_access_permision); // drop inesesario mas halla de test manuales
        Ok(result)
    }
}
