//! Persistent bee identity — ported from hum's `nest-common::identity`
//! (minus the hum-path/ensemble deps). Each hive install gets its own
//! Ed25519 keypair at `$XDG_STATE_HOME/hum/bees/<kind>.key`; the pubkey
//! hashes to a stable role-prefixed Hid (`fbee_<hex>`) that survives
//! reconnect and restart. Same file format as humd's bee keys.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use ed25519_dalek::SigningKey;
use rand::RngCore;

use super::hid::{Hid, HidPrefix};
use super::paths;

#[derive(Debug)]
pub struct BeeKey {
    pub signing: SigningKey,
    pub hid: Hid,
}

pub fn bee_key_path(kind: &str) -> PathBuf {
    paths::bee_key(kind)
}

pub fn load_or_mint_bee_key(kind: &str, prefix: HidPrefix) -> Result<BeeKey> {
    let path = bee_key_path(kind);
    if path.exists() {
        let bytes = fs::read(&path)
            .with_context(|| format!("read bee key {}", path.display()))?;
        if bytes.len() != 32 {
            return Err(anyhow!("bee key at {} is {} bytes, expected 32", path.display(), bytes.len()));
        }
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&bytes);
        let signing = SigningKey::from_bytes(&arr);
        let hid = Hid::from_pubkey(prefix, &signing.verifying_key().to_bytes());
        return Ok(BeeKey { signing, hid });
    }

    let mut seed = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut seed);
    let signing = SigningKey::from_bytes(&seed);
    persist(&path, &seed)?;
    let hid = Hid::from_pubkey(prefix, &signing.verifying_key().to_bytes());
    Ok(BeeKey { signing, hid })
}

fn persist(path: &Path, seed: &[u8; 32]) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("mkdir -p {}", parent.display()))?;
    }
    let tmp = match path.file_name() {
        Some(name) => {
            let mut tmp_name = name.to_os_string();
            tmp_name.push(".tmp");
            path.with_file_name(tmp_name)
        }
        None => return Err(anyhow!("bee key path has no file name: {}", path.display())),
    };

    {
        let mut opts = fs::OpenOptions::new();
        opts.write(true).create(true).truncate(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            opts.mode(0o600);
        }
        let mut f = opts
            .open(&tmp)
            .with_context(|| format!("create {}", tmp.display()))?;
        f.write_all(seed)
            .with_context(|| format!("write {}", tmp.display()))?;
        f.sync_all().ok();
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&tmp, fs::Permissions::from_mode(0o600));
    }

    fs::rename(&tmp, path)
        .with_context(|| format!("rename {} -> {}", tmp.display(), path.display()))?;
    Ok(())
}
