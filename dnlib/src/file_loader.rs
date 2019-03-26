use std::{io, fs};
use std::path::{Path, PathBuf};
use std::collections::HashMap;

/// A trait for disk IO, to allow us to mock out the filesystem.
pub trait FileLoader {
    fn read_to_string(&self, path: &Path) -> io::Result<String>;
}

/// A struct that passes FileLoader calls through to the
/// underlying OS file system.
#[derive(Debug, Default, Copy, Clone)]
pub struct DiskFileLoader;

impl FileLoader for DiskFileLoader {
    fn read_to_string(&self, path: &Path) -> io::Result<String> {
        fs::read_to_string(path)
    }
}

/// A struct that implements FileLoader by resolving calls from
/// an in-memory hash map of paths to file contents.
#[derive(Debug, Default, Clone)]
pub struct MemoryFileLoader {
    pub files: HashMap<PathBuf, String>
}

impl MemoryFileLoader {
    pub fn new() -> Self {
        Self::default()
    }
}

impl FileLoader for MemoryFileLoader {
    fn read_to_string(&self, path: &Path) -> io::Result<String> {
        self.files.get(path)
        .map_or(
            Err(io::Error::new(io::ErrorKind::NotFound, path.to_string_lossy())),
            |contents| Ok(contents.to_owned()))
    }
}
