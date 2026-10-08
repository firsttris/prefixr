//! Helpers shared by the tests.

use std::ops::Deref;
use std::path::Path;

/// A fresh temporary directory, removed when the test ends — passed or
/// failed. Derefs to its path.
pub struct TestDir(tempfile::TempDir);

impl TestDir {
    pub fn new(name: &str) -> Self {
        let dir = tempfile::Builder::new()
            .prefix(&format!("prefixr-test-{name}-"))
            .tempdir()
            .expect("temporary directory");
        TestDir(dir)
    }
}

impl Deref for TestDir {
    type Target = Path;

    fn deref(&self) -> &Path {
        self.0.path()
    }
}

impl AsRef<Path> for TestDir {
    fn as_ref(&self) -> &Path {
        self.0.path()
    }
}
