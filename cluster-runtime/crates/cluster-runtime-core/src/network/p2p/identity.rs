use std::path::{Path, PathBuf};

use iroh::SecretKey;

const KEY_FILE: &str = "secret.key";

/// Load or create a persistent iroh [`SecretKey`] under `{data_dir}/iroh/`.
pub fn load_or_generate(data_dir: &Path) -> Result<SecretKey, String> {
    let dir = data_dir.join("iroh");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = dir.join(KEY_FILE);

    if path.exists() {
        let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
        if bytes.len() != 32 {
            return Err(format!(
                "iroh secret key at {} has invalid length {}",
                path.display(),
                bytes.len()
            ));
        }
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&bytes);
        return Ok(SecretKey::from_bytes(&arr));
    }

    let key = SecretKey::generate();
    std::fs::write(&path, key.to_bytes()).map_err(|e| e.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
    }
    Ok(key)
}

pub fn identity_path(data_dir: &Path) -> PathBuf {
    data_dir.join("iroh").join(KEY_FILE)
}
