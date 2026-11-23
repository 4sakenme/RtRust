//! System keyring entries for the signed-in session and saved accounts.
//! Legacy publishing-key entries remain accessible for older clients.
//!
//! On iOS (sideloaded builds without signed `keychain-access-groups` entitlement)
//! and Android, or whenever the system keyring fails, secrets fall back to a
//! `0600`-permission file inside the app's private data directory.

use crate::{Error, Result};

const SERVICE: &str = "rustudio";
const API_KEY_USER: &str = "roblox-open-cloud-api-key";
/// Left behind by OAuth sign-in before the cookie session; removed on sight.
const OLD_OAUTH_USER: &str = "roblox-oauth-tokens";

#[cfg(all(not(test), not(any(target_os = "android", target_os = "ios"))))]
fn entry(user: &str) -> Result<keyring::Entry> {
    keyring::Entry::new(SERVICE, user)
        .map_err(|e| Error(format!("system keyring unavailable: {e}")))
}

#[cfg(not(test))]
fn secret_file_path(user: &str) -> std::path::PathBuf {
    let safe: String = user
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.') { c } else { '_' })
        .collect();
    if cfg!(target_os = "ios") {
        let base = std::env::var_os("HOME")
            .filter(|v| !v.is_empty())
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("Documents");
        return base.join(format!(".auth_session_{safe}.dat"));
    }
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .filter(|v| !v.is_empty())
        .map(std::path::PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME")
                .filter(|v| !v.is_empty())
                .map(|h| std::path::PathBuf::from(h).join(".config"))
        })
        .unwrap_or_else(|| std::path::PathBuf::from("."));
    base.join("rustudio").join("secrets").join(format!("{safe}.bin"))
}

#[cfg(not(test))]
fn file_load(user: &str) -> Result<Option<Vec<u8>>> {
    let path = secret_file_path(user);
    match std::fs::read(&path) {
        Ok(v) => Ok(Some(v)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(Error(format!("read secret {}: {e}", path.display()))),
    }
}

#[cfg(not(test))]
fn file_save(user: &str, secret: &[u8]) -> Result<()> {
    let path = secret_file_path(user);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| Error(format!("create secret dir {}: {e}", parent.display())))?;
    }
    std::fs::write(&path, secret)
        .map_err(|e| Error(format!("write secret {}: {e}", path.display())))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
    }
    Ok(())
}

#[cfg(not(test))]
fn file_delete(user: &str) -> Result<()> {
    let path = secret_file_path(user);
    match std::fs::remove_file(&path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(Error(format!("remove secret {}: {e}", path.display()))),
    }
}

/// The secret stored for `user` under service `rustudio`, if any. Raw
/// bytes, so Windows (which stores a password as UTF-16) fits twice as much.
#[cfg(all(not(test), not(any(target_os = "android", target_os = "ios"))))]
pub(crate) fn load(user: &str) -> Result<Option<Vec<u8>>> {
    if let Ok(e) = entry(user) {
        match e.get_secret() {
            Ok(v) => return Ok(Some(v)),
            Err(keyring::Error::NoEntry) => {
                if let Ok(Some(v)) = file_load(user) {
                    return Ok(Some(v));
                }
                return Ok(None);
            }
            Err(_) => {}
        }
    }
    file_load(user)
}

#[cfg(all(not(test), not(any(target_os = "android", target_os = "ios"))))]
pub(crate) fn save(user: &str, secret: &[u8]) -> Result<()> {
    if let Ok(e) = entry(user) {
        if e.set_secret(secret).is_ok() {
            let _ = file_save(user, secret);
            return Ok(());
        }
    }
    file_save(user, secret)
}

#[cfg(all(not(test), not(any(target_os = "android", target_os = "ios"))))]
pub(crate) fn delete(user: &str) -> Result<()> {
    if let Ok(e) = entry(user) {
        let _ = e.delete_credential();
    }
    file_delete(user)
}

#[cfg(all(not(test), any(target_os = "android", target_os = "ios")))]
pub(crate) fn load(user: &str) -> Result<Option<Vec<u8>>> {
    file_load(user)
}

#[cfg(all(not(test), any(target_os = "android", target_os = "ios")))]
pub(crate) fn save(user: &str, secret: &[u8]) -> Result<()> {
    file_save(user, secret)
}

#[cfg(all(not(test), any(target_os = "android", target_os = "ios")))]
pub(crate) fn delete(user: &str) -> Result<()> {
    file_delete(user)
}

#[cfg(test)]
thread_local! {
    /// Tests keep the entries in memory (per test thread) instead of the
    /// person's keyring.
    static MEMORY: std::cell::RefCell<std::collections::HashMap<String, Vec<u8>>> =
        Default::default();
}

#[cfg(test)]
pub(crate) fn load(user: &str) -> Result<Option<Vec<u8>>> {
    Ok(MEMORY.with(|m| m.borrow().get(user).cloned()))
}

#[cfg(test)]
pub(crate) fn save(user: &str, secret: &[u8]) -> Result<()> {
    MEMORY.with(|m| m.borrow_mut().insert(user.to_string(), secret.to_vec()));
    Ok(())
}

#[cfg(test)]
pub(crate) fn delete(user: &str) -> Result<()> {
    MEMORY.with(|m| m.borrow_mut().remove(user));
    Ok(())
}

pub struct PublishKeyStore;

impl PublishKeyStore {
    pub fn save(key: &str) -> Result<()> {
        save(API_KEY_USER, key.trim().as_bytes())
    }

    /// The saved publishing key, if any.
    pub fn load() -> Result<Option<String>> {
        let Some(bytes) = load(API_KEY_USER)? else {
            return Ok(None);
        };
        let s = String::from_utf8_lossy(&bytes).into_owned();
        Ok(Some(s).filter(|k| !k.trim().is_empty()))
    }

    pub fn clear() -> Result<()> {
        delete(API_KEY_USER)
    }

    /// Deletes the OAuth tokens an older rustudio saved.
    pub fn forget_oauth_tokens() {
        let _ = delete(OLD_OAUTH_USER);
    }
}
