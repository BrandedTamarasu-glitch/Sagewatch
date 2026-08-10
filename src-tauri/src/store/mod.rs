use crate::domain::{Preferences, Provider, ProviderStatus};
use serde::{de::DeserializeOwned, Serialize};
use std::{
    collections::BTreeMap,
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
};
use thiserror::Error;

const SNAPSHOTS_FILE: &str = "snapshots.json";
const PREFERENCES_FILE: &str = "preferences.json";

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("filesystem operation failed: {0}")]
    Io(#[from] io::Error),
    #[error("stored JSON is invalid: {0}")]
    Json(#[from] serde_json::Error),
}

#[derive(Clone, Debug)]
pub struct JsonStore {
    root: PathBuf,
}

impl JsonStore {
    pub fn new(root: impl Into<PathBuf>) -> Result<Self, StoreError> {
        let root = root.into();
        ensure_private_dir(&root)?;
        Ok(Self { root })
    }

    pub fn load_snapshots(&self) -> Result<BTreeMap<Provider, ProviderStatus>, StoreError> {
        self.read_or_recover(SNAPSHOTS_FILE, BTreeMap::new())
    }

    pub fn save_snapshots(
        &self,
        snapshots: &BTreeMap<Provider, ProviderStatus>,
    ) -> Result<(), StoreError> {
        atomic_write_json(&self.root.join(SNAPSHOTS_FILE), snapshots)
    }

    pub fn load_preferences(&self) -> Result<Preferences, StoreError> {
        self.read_or_recover(PREFERENCES_FILE, Preferences::default())
    }

    pub fn save_preferences(&self, preferences: &Preferences) -> Result<(), StoreError> {
        atomic_write_json(&self.root.join(PREFERENCES_FILE), preferences)
    }

    fn read_or_recover<T>(&self, name: &str, default: T) -> Result<T, StoreError>
    where
        T: DeserializeOwned + Serialize + Clone,
    {
        let path = self.root.join(name);
        match fs::read(&path) {
            Ok(bytes) => match serde_json::from_slice(&bytes) {
                Ok(value) => Ok(value),
                Err(_) => {
                    let corrupt = self.root.join(format!("{name}.corrupt"));
                    if corrupt.exists() {
                        fs::remove_file(&corrupt)?;
                    }
                    fs::rename(&path, corrupt)?;
                    atomic_write_json(&path, &default)?;
                    Ok(default)
                }
            },
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(default),
            Err(error) => Err(error.into()),
        }
    }
}

fn ensure_private_dir(path: &Path) -> io::Result<()> {
    fs::create_dir_all(path)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
    }
    Ok(())
}

fn atomic_write_json<T: Serialize + ?Sized>(path: &Path, value: &T) -> Result<(), StoreError> {
    let parent = path
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "store path has no parent"))?;
    ensure_private_dir(parent)?;
    let tmp = path.with_extension(format!(
        "tmp-{}-{}",
        std::process::id(),
        rand::random::<u64>()
    ));
    let bytes = serde_json::to_vec_pretty(value)?;
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(&tmp)?;
    file.write_all(&bytes)?;
    file.sync_all()?;
    fs::rename(&tmp, path)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o600))?;
    }
    if let Ok(directory) = fs::File::open(parent) {
        directory.sync_all()?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn recovers_corrupt_preferences() {
        let dir = tempfile::tempdir().unwrap();
        let store = JsonStore::new(dir.path()).unwrap();
        fs::write(dir.path().join(PREFERENCES_FILE), b"not-json").unwrap();
        assert_eq!(store.load_preferences().unwrap(), Preferences::default());
        assert!(dir.path().join("preferences.json.corrupt").exists());
    }
    #[cfg(unix)]
    #[test]
    fn uses_restrictive_permissions() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let store = JsonStore::new(dir.path().join("state")).unwrap();
        store.save_preferences(&Preferences::default()).unwrap();
        assert_eq!(
            fs::metadata(dir.path().join("state"))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o700
        );
        assert_eq!(
            fs::metadata(dir.path().join("state/preferences.json"))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
    }
}
