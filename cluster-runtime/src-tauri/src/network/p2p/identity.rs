use std::path::{Path, PathBuf};

use libp2p::identity::Keypair;
use libp2p::PeerId;

const KEY_FILE: &str = "identity.key";

/// Load or create a persistent Ed25519 keypair under `{data_dir}/p2p/`.
pub fn load_or_generate(data_dir: &Path) -> Result<(Keypair, PeerId), String> {
    let dir = data_dir.join("p2p");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = dir.join(KEY_FILE);

    let keypair = if path.exists() {
        let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
        Keypair::from_protobuf_encoding(&bytes).map_err(|e| e.to_string())?
    } else {
        let kp = Keypair::generate_ed25519();
        let bytes = kp.to_protobuf_encoding().map_err(|e| e.to_string())?;
        std::fs::write(&path, bytes).map_err(|e| e.to_string())?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
        }
        kp
    };

    let peer_id = PeerId::from(keypair.public());
    Ok((keypair, peer_id))
}

pub fn identity_path(data_dir: &Path) -> PathBuf {
    data_dir.join("p2p").join(KEY_FILE)
}
