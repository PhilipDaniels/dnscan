use std::path::{Path, PathBuf};
use crate::file_loader::FileLoader;
use crate::path_extensions::PathExtensions;

#[derive(Debug, Default, Clone, PartialEq, Eq)]
/// Represents information about a .sln or .csproj file.
pub struct FileInfo {
    pub path: PathBuf,
    pub contents: String,
    pub is_valid_utf8: bool,
}

impl FileInfo {
    pub fn new<P, L>(path: P, file_loader: &L) -> Self
        where P: AsRef<Path>,
              L: FileLoader
    {
        let mut fi = FileInfo::default();
        fi.path = path.as_ref().to_owned();
        let file_contents_result = file_loader.read_to_string(&fi.path);
        fi.is_valid_utf8 = file_contents_result.is_ok();
        fi.contents = file_contents_result.unwrap_or_default();
        fi
    }

    /// Returns the whole path as a str, or "" if it cannot be converted.
    pub fn path_as_str(&self) -> &str {
        self.path.as_str()
    }

    /// Returns the final filename component as a str, or "" if it cannot be converted.
    pub fn filename_as_str(&self) -> &str {
        self.path.filename_as_str()
    }

    /// Returns the directory component as a str, or "" if it cannot be converted.
    pub fn directory_as_str(&self) -> &str {
        self.path.directory_as_str()
    }
}
