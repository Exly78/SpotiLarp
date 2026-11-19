//! Logins kept between runs, in the system keyring: Credential Manager on
//! Windows, the Secret Service (gnome-keyring, KWallet, KeePassXC) on Linux.
//!
//! Plenty of Linux setups run no Secret Service (many OpenRC and bare window
//! manager ones), and there the keyring can't keep anything: logins lasted
//! until the app closed. So on Linux a login the keyring turns away goes to a
//! file in the config folder that only this user can read, and moves back
//! into the keyring the next time it's saved and the keyring takes it.

use keyring::{Entry, Error};

pub fn save(service: &str, user: &str, secret: &str) -> Result<(), String> {
    match Entry::new(service, user).and_then(|entry| entry.set_password(secret)) {
        Ok(()) => {
            // In the keyring now: a copy kept in the file before isn't needed.
            let _ = file::clear(service, user);
            Ok(())
        }
        #[cfg(target_os = "linux")]
        Err(e) => file::save(service, user, secret).map_err(|file_error| format!("{e}; {file_error}")),
        #[cfg(not(target_os = "linux"))]
        Err(e) => Err(e.to_string()),
    }
}

pub fn load(service: &str, user: &str) -> Option<String> {
    Entry::new(service, user)
        .and_then(|entry| entry.get_password())
        .ok()
        .or_else(|| file::load(service, user))
}

pub fn clear(service: &str, user: &str) -> Result<(), String> {
    file::clear(service, user)?;
    match Entry::new(service, user).and_then(|entry| entry.delete_credential()) {
        Ok(()) | Err(Error::NoEntry) => Ok(()),
        // No Secret Service to reach: nothing can be kept in it either.
        Err(Error::PlatformFailure(_)) if cfg!(target_os = "linux") => Ok(()),
        Err(e) => Err(e.to_string()),
    }
}

#[cfg(target_os = "linux")]
mod file {
    use std::collections::BTreeMap;
    use std::io::{ErrorKind, Write};
    use std::os::unix::fs::OpenOptionsExt;
    use std::path::{Path, PathBuf};
    use std::sync::Mutex;

    /// Keyed "service/user".
    type Logins = BTreeMap<String, String>;

    static LOCK: Mutex<()> = Mutex::new(());

    fn path() -> PathBuf {
        crate::config::dir().join("logins.json")
    }

    fn key(service: &str, user: &str) -> String {
        format!("{service}/{user}")
    }

    pub fn save(service: &str, user: &str, secret: &str) -> Result<(), String> {
        let _lock = LOCK.lock().unwrap_or_else(|e| e.into_inner());
        save_in(&path(), &key(service, user), secret)
    }

    pub fn load(service: &str, user: &str) -> Option<String> {
        let _lock = LOCK.lock().unwrap_or_else(|e| e.into_inner());
        read(&path()).remove(&key(service, user))
    }

    pub fn clear(service: &str, user: &str) -> Result<(), String> {
        let _lock = LOCK.lock().unwrap_or_else(|e| e.into_inner());
        clear_in(&path(), &key(service, user))
    }

    pub(super) fn save_in(path: &Path, key: &str, secret: &str) -> Result<(), String> {
        let mut logins = read(path);
        logins.insert(key.to_string(), secret.to_string());
        write(path, &logins)
    }

    pub(super) fn clear_in(path: &Path, key: &str) -> Result<(), String> {
        let mut logins = read(path);
        if logins.remove(key).is_none() {
            return Ok(());
        }
        write(path, &logins)
    }

    pub(super) fn read(path: &Path) -> Logins {
        std::fs::read(path)
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default()
    }

    /// Written to a new file that's made private before anything goes in,
    /// then moved over the old one.
    fn write(path: &Path, logins: &Logins) -> Result<(), String> {
        let failed = |e: std::io::Error| format!("Couldn't save the login in {}: {e}", path.display());
        if logins.is_empty() {
            return match std::fs::remove_file(path) {
                Err(e) if e.kind() != ErrorKind::NotFound => Err(failed(e)),
                _ => Ok(()),
            };
        }
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(failed)?;
        }
        let temp = path.with_extension("json.tmp");
        let _ = std::fs::remove_file(&temp);
        let body = serde_json::to_vec(logins).map_err(|e| e.to_string())?;
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&temp)
            .map_err(failed)?;
        file.write_all(&body).and_then(|()| file.sync_all()).map_err(failed)?;
        std::fs::rename(&temp, path).map_err(failed)
    }
}

/// Without the Linux fallback there's never anything in a file.
#[cfg(not(target_os = "linux"))]
mod file {
    pub fn load(_service: &str, _user: &str) -> Option<String> {
        None
    }

    pub fn clear(_service: &str, _user: &str) -> Result<(), String> {
        Ok(())
    }
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::file::{clear_in, read, save_in};
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn file_logins_are_private_and_cleared() {
        let dir = std::env::temp_dir().join(format!("spotilarp-secret-store-{}", std::process::id()));
        let path = dir.join("logins.json");
        let _ = std::fs::remove_dir_all(&dir);

        save_in(&path, "a/refresh_token", "one").unwrap();
        save_in(&path, "b/refresh_token", "two").unwrap();
        assert_eq!(read(&path).get("a/refresh_token").map(String::as_str), Some("one"));
        assert_eq!(std::fs::metadata(&path).unwrap().permissions().mode() & 0o777, 0o600);

        clear_in(&path, "a/refresh_token").unwrap();
        assert_eq!(read(&path).get("a/refresh_token"), None);
        assert_eq!(read(&path).get("b/refresh_token").map(String::as_str), Some("two"));

        // The last one out removes the file.
        clear_in(&path, "b/refresh_token").unwrap();
        assert!(!path.exists());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
