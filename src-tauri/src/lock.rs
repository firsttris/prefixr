use std::sync::{Mutex, MutexGuard, PoisonError};

/// `Mutex::lock` that also takes a poisoned lock. A command that panics
/// while holding one of the app's mutexes (only possible in a dev build: the
/// release profile aborts on a panic) would otherwise make every later
/// command that needs it fail until a restart, with nothing the user could
/// do about it. Nothing behind these locks can be left half-updated in a way
/// that matters: the config is plain data whose file is only ever replaced
/// atomically (see `save_config`), the rest are maps and sets of the games
/// in flight.
pub trait LockExt<T> {
    fn locked(&self) -> MutexGuard<'_, T>;
}

impl<T> LockExt<T> for Mutex<T> {
    fn locked(&self) -> MutexGuard<'_, T> {
        self.lock().unwrap_or_else(PoisonError::into_inner)
    }
}
