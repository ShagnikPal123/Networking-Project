//! Relay `store` module (error prefix `RLY-ST`). Local persistence: config, roster, transfer state.
//!
//! Boundaries and rules: see `docs/design/01-architecture.md` and
//! `docs/agent/modules/store.md`.
//!
//! The store keeps opaque bytes under namespaced keys. It never knows what the bytes mean.
//! [`MemoryStore`] is real; [`FileStore`] is a stub until disk I/O is built.

use std::collections::HashMap;
use std::fmt;
use std::path::PathBuf;
use std::sync::Mutex;

use relay_common::{ErrorCode, Module, RelayError};

/// This crate's module tag, used in error codes and health reports.
pub const MODULE: Module = Module::Store;

/// Errors raised by the `store` module.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The feature exists in the design but is not built yet.
    #[error("{code} not implemented yet: {what}", code = ErrorCode::new(MODULE, 1))]
    NotImplemented {
        /// The missing feature.
        what: &'static str,
    },
    /// A key part is empty or has characters outside `a-z 0-9 . _ -`.
    #[error("{code} invalid key part {part:?}", code = ErrorCode::new(MODULE, 2))]
    InvalidKey {
        /// The rejected namespace or name.
        part: String,
    },
}

impl RelayError for Error {
    fn code(&self) -> ErrorCode {
        match self {
            Error::NotImplemented { .. } => ErrorCode::new(MODULE, 1),
            Error::InvalidKey { .. } => ErrorCode::new(MODULE, 2),
        }
    }
}

/// Result type for this module.
pub type Result<T> = std::result::Result<T, Error>;

/// A namespaced key, such as `config/device` or `roster/members`.
///
/// Both parts are limited to `a-z 0-9 . _ -` and must not be empty, so a key can never
/// escape its namespace when mapped to a file path later.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Key {
    namespace: String,
    name: String,
}

impl Key {
    /// Builds a key, rejecting unsafe parts. A part of only dots (`.`, `..`) is rejected too.
    pub fn new(namespace: &str, name: &str) -> Result<Self> {
        for part in [namespace, name] {
            let ok = !part.is_empty()
                && !part.chars().all(|c| c == '.')
                && part
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || "._-".contains(c));
            if !ok {
                return Err(Error::InvalidKey {
                    part: part.to_string(),
                });
            }
        }
        Ok(Self {
            namespace: namespace.to_string(),
            name: name.to_string(),
        })
    }

    /// The namespace part.
    pub fn namespace(&self) -> &str {
        &self.namespace
    }

    /// The name part.
    pub fn name(&self) -> &str {
        &self.name
    }
}

impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.namespace, self.name)
    }
}

/// A byte store over namespaced keys.
pub trait Store: Send + Sync {
    /// Returns the bytes under `key`, or `None` if nothing is stored.
    fn get(&self, key: &Key) -> Result<Option<Vec<u8>>>;

    /// Stores `value` under `key`, replacing any previous value.
    fn put(&self, key: &Key, value: &[u8]) -> Result<()>;

    /// Removes `key`. Returns whether something was removed.
    fn delete(&self, key: &Key) -> Result<bool>;
}

/// A store that lives only in memory. For tests and the `sim` harness.
#[derive(Debug, Default)]
pub struct MemoryStore {
    map: Mutex<HashMap<Key, Vec<u8>>>,
}

impl MemoryStore {
    /// An empty store.
    pub fn new() -> Self {
        Self::default()
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<Key, Vec<u8>>> {
        // A panic elsewhere cannot leave the map half-written, so poisoning is safe to ignore.
        self.map.lock().unwrap_or_else(|e| e.into_inner())
    }
}

impl Store for MemoryStore {
    fn get(&self, key: &Key) -> Result<Option<Vec<u8>>> {
        Ok(self.lock().get(key).cloned())
    }

    fn put(&self, key: &Key, value: &[u8]) -> Result<()> {
        self.lock().insert(key.clone(), value.to_vec());
        Ok(())
    }

    fn delete(&self, key: &Key) -> Result<bool> {
        Ok(self.lock().remove(key).is_some())
    }
}

/// A store backed by files under a directory. Not built yet: every call returns
/// [`Error::NotImplemented`].
#[derive(Debug, Clone)]
pub struct FileStore {
    root: PathBuf,
}

impl FileStore {
    /// Points a store at `root`. Does not touch the disk.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// The directory this store will use.
    pub fn root(&self) -> &std::path::Path {
        &self.root
    }
}

impl Store for FileStore {
    fn get(&self, _key: &Key) -> Result<Option<Vec<u8>>> {
        Err(Error::NotImplemented {
            what: "file store read",
        })
    }

    fn put(&self, _key: &Key, _value: &[u8]) -> Result<()> {
        Err(Error::NotImplemented {
            what: "file store write",
        })
    }

    fn delete(&self, _key: &Key) -> Result<bool> {
        Err(Error::NotImplemented {
            what: "file store delete",
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn k(ns: &str, n: &str) -> Key {
        Key::new(ns, n).unwrap()
    }

    #[test]
    fn every_error_code_has_module_prefix() {
        let all = [
            (Error::NotImplemented { what: "x" }, "RLY-ST-001"),
            (Error::InvalidKey { part: "X".into() }, "RLY-ST-002"),
        ];
        for (e, code) in all {
            assert_eq!(e.code().to_string(), code);
            assert!(e.to_string().starts_with(code), "{e}");
        }
    }

    #[test]
    fn put_get_overwrite_delete() {
        let s = MemoryStore::new();
        let key = k("config", "device");
        assert_eq!(s.get(&key).unwrap(), None);
        s.put(&key, b"one").unwrap();
        assert_eq!(s.get(&key).unwrap().as_deref(), Some(&b"one"[..]));
        s.put(&key, b"two").unwrap();
        assert_eq!(s.get(&key).unwrap().as_deref(), Some(&b"two"[..]));
        assert!(s.delete(&key).unwrap());
        assert!(!s.delete(&key).unwrap());
        assert_eq!(s.get(&key).unwrap(), None);
    }

    #[test]
    fn namespaces_are_separate() {
        let s = MemoryStore::new();
        s.put(&k("a", "x"), b"1").unwrap();
        s.put(&k("b", "x"), b"2").unwrap();
        assert_eq!(s.get(&k("a", "x")).unwrap().unwrap(), b"1");
        assert_eq!(s.get(&k("b", "x")).unwrap().unwrap(), b"2");
        assert_eq!(k("a", "x").to_string(), "a/x");
    }

    #[test]
    fn empty_value_is_not_missing() {
        let s = MemoryStore::new();
        s.put(&k("a", "e"), b"").unwrap();
        assert_eq!(s.get(&k("a", "e")).unwrap(), Some(vec![]));
    }

    #[test]
    fn bad_keys_rejected() {
        for (ns, n) in [
            ("", "x"),
            ("a", ""),
            ("a/b", "x"),
            ("a", ".."),
            ("A", "x"),
            ("a", "x y"),
        ] {
            assert!(
                matches!(Key::new(ns, n), Err(Error::InvalidKey { .. })),
                "{ns}/{n}"
            );
        }
    }

    #[test]
    fn file_store_is_a_stub() {
        let s = FileStore::new("/nonexistent");
        let key = k("a", "b");
        assert!(matches!(s.get(&key), Err(Error::NotImplemented { .. })));
        assert!(matches!(
            s.put(&key, b""),
            Err(Error::NotImplemented { .. })
        ));
        assert!(matches!(s.delete(&key), Err(Error::NotImplemented { .. })));
        assert_eq!(s.root(), std::path::Path::new("/nonexistent"));
    }

    #[test]
    fn works_as_trait_object() {
        let s: Box<dyn Store> = Box::new(MemoryStore::new());
        s.put(&k("a", "b"), b"v").unwrap();
        assert!(s.get(&k("a", "b")).unwrap().is_some());
    }
}
