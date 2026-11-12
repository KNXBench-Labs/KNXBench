//! The open project's file path, with a copy readable while a save holds the path.
//!
//! A Save keeps the path locked while it writes for seconds (#36, WP-31). Plain
//! reads that only ask which file the project has, and the background journal
//! writer, read [`StorePath::known`] instead, which never waits for that holder.
//! The copy changes only through [`StorePathGuard::set`], under the path lock.

use std::ops::Deref;
use std::path::PathBuf;
use std::sync::{Arc, LockResult, Mutex, MutexGuard, PoisonError, TryLockError};

/// The `.knxdb` file of the open project; `None` until it has one.
#[derive(Default)]
pub struct StorePath {
    path: Mutex<Option<PathBuf>>,
    /// A leaf lock: held only to copy the value in or out, never while
    /// another lock is taken.
    known: Arc<Mutex<Option<PathBuf>>>,
}

impl StorePath {
    /// Locks the path; whoever writes the file or replaces the project holds it.
    pub fn lock(&self) -> LockResult<StorePathGuard<'_>> {
        let wrap = |path| StorePathGuard {
            path,
            known: &self.known,
        };
        match self.path.lock() {
            Ok(path) => Ok(wrap(path)),
            Err(poisoned) => Err(PoisonError::new(wrap(poisoned.into_inner()))),
        }
    }

    /// The lock if nobody holds it, `None` while someone does (a Save writing).
    pub(crate) fn lock_if_free(&self) -> Option<StorePathGuard<'_>> {
        match self.path.try_lock() {
            Ok(path) => Some(StorePathGuard {
                path,
                known: &self.known,
            }),
            Err(TryLockError::WouldBlock) => None,
            Err(TryLockError::Poisoned(_)) => panic!("state mutex poisoned"),
        }
    }

    /// The path as last set, without waiting for a holder of the lock.
    pub fn known(&self) -> Option<PathBuf> {
        read(&self.known)
    }

    /// The copy behind [`Self::known`], for the background journal writer.
    pub(crate) fn known_cell(&self) -> Arc<Mutex<Option<PathBuf>>> {
        Arc::clone(&self.known)
    }
}

/// Reads a copy made by [`StorePath::known_cell`].
pub(crate) fn read(known: &Mutex<Option<PathBuf>>) -> Option<PathBuf> {
    known.lock().unwrap_or_else(PoisonError::into_inner).clone()
}

/// The locked path. It changes only through [`Self::set`], which updates the
/// copy at once, so no reader of the copy sees a value the lock never held.
pub struct StorePathGuard<'a> {
    path: MutexGuard<'a, Option<PathBuf>>,
    known: &'a Mutex<Option<PathBuf>>,
}

impl StorePathGuard<'_> {
    pub fn set(&mut self, path: Option<PathBuf>) {
        *self.known.lock().unwrap_or_else(PoisonError::into_inner) = path.clone();
        *self.path = path;
    }
}

impl Deref for StorePathGuard<'_> {
    type Target = Option<PathBuf>;

    fn deref(&self) -> &Self::Target {
        &self.path
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_copy_follows_every_set_and_answers_while_the_path_is_held() {
        let store = StorePath::default();
        assert_eq!(store.known(), None);
        let mut held = store.lock().unwrap();
        held.set(Some(PathBuf::from("a.knxdb")));
        // Still held: the copy already answers, and the guard reads the value.
        assert_eq!(store.known(), Some(PathBuf::from("a.knxdb")));
        assert_eq!(held.as_deref(), Some(std::path::Path::new("a.knxdb")));
        held.set(None);
        assert_eq!(store.known(), None);
        drop(held);
        assert_eq!(*store.lock().unwrap(), None);
    }
}
