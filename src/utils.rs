use crate::git_errors::errors::ErrorType;
use chrono::{DateTime, Utc};
use std::path::Path;
use std::time::SystemTime;

// todo borrar
pub fn verify_path_exists(path: &Path) -> Result<(), ErrorType> {
    if !path.exists() {
        // user passed an inexistent path
        return Err(ErrorType::FileNotFound(format!("{}", path.display())));
    }
    Ok(())
}

pub fn get_current_date_time() -> Result<String, ErrorType> {
    let system_time = SystemTime::now();
    let date_time: DateTime<Utc> = system_time.into();
    Ok(date_time.format("%d/%m/%Y %T").to_string())
}
