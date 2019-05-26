use std::{io, fs};
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use std::str::FromStr;
use walkdir::{DirEntry, WalkDir};
use crate::errors::DnLibResult;
use crate::enums::InterestingFile;
use crate::{timer, finish};

/// A trait for disk IO, to allow us to mock out the filesystem.
pub trait FileLoader : Clone {
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

/// This struct is used to collect the raw directory walking results prior to further
/// analysis. It is basically just a list of paths of various types. No effort is made
/// to relate the csproj files to their owning sln files, for example (that requires
/// probing inside the file contents and is left to a later stage of analysis).
#[derive(Debug, Default)]
pub struct PathsToAnalyze {
    pub sln_files: Vec<PathBuf>,
    pub csproj_files: Vec<PathBuf>,
    pub other_files: Vec<PathBuf>
}

pub fn find_files<P>(path: P) -> DnLibResult<PathsToAnalyze>
    where P: AsRef<Path>
{
    let tmr = timer!("Find Files", "Dir={:?}", path.as_ref());

    let mut pta = PathsToAnalyze::default();
    let walker = WalkDir::new(path);

    for entry in walker.into_iter().filter_entry(|e| continue_walking(e)) {
        let entry = entry?;
        let path = entry.path();

        if path.is_sln_file() {
            pta.sln_files.push(path.to_owned());
        } else if path.is_csproj_file() {
            pta.csproj_files.push(path.to_owned());
        } else {
            let filename = path.filename_as_str();
            if is_file_of_interest(&filename) {
                pta.other_files.push(path.to_owned());
            }
        }
    }

    finish!(tmr,
        "NumSolutions={} NumCsProj={}, NumOtherFiles={}",
        pta.sln_files.len(),
        pta.csproj_files.len(),
        pta.other_files.len()
        );

    Ok(pta)
}

fn continue_walking(entry: &DirEntry) -> bool {
    let path = entry.path();
    if path.is_hidden_dir()
        || path.is_bin_or_obj_dir()
        || path.is_packages_dir()
        || path.is_test_results_dir()
        || path.is_node_modules_dir()
        || path.is_git_dir()
    {
        return false;
    }

    true
}

fn is_file_of_interest(filename: &str) -> bool {
    InterestingFile::from_str(filename).is_ok()
}

pub trait PathExtensions {
    // Returns the path as a str, or "" if it cannot be converted.
    fn as_str(&self) -> &str;
    // Returns the final filename component as a str, or "" if it cannot be converted.
    fn filename_as_str(&self) -> &str;
    // Returns the final filename component excluding extension as a str, or "" if it cannot be converted.
    fn file_stem_as_str(&self) -> &str;
    // Returns the directory as a str, or "" if it cannot be converted.
    fn directory_as_str(&self) -> &str;
    // Returns the extension as a str, or "" if it cannot be converted.
    fn extension_as_str(&self) -> &str;
    fn eq_ignoring_case<P: AsRef<Path>>(&self, other: P) -> bool;
    fn is_same_dir<P: AsRef<Path>>(&self, other: P) -> bool;
    fn is_hidden_dir(&self) -> bool;
    fn is_bin_or_obj_dir(&self) -> bool;
    fn is_packages_dir(&self) -> bool;
    fn is_test_results_dir(&self) -> bool;
    fn is_node_modules_dir(&self) -> bool;
    fn is_git_dir(&self) -> bool;
    fn is_solution_info_file(&self) -> bool;
    fn is_version_out_file(&self) -> bool;
    fn is_sln_file(&self) -> bool;
    fn is_csproj_file(&self) -> bool;
    fn is_suo_file(&self) -> bool;
    fn is_upgrade_log_file(&self) -> bool;
    fn is_git_orig_file(&self) -> bool;
    fn is_mef_cache_dir(&self) -> bool;
    fn is_jet_brains_cache_dir(&self) -> bool;
}

impl PathExtensions for Path {
    fn as_str(&self) -> &str {
        self.to_str().unwrap_or_default()
    }

    fn filename_as_str(&self) -> &str {
        match self.file_name() {
            None => "",
            Some(osstr) => match osstr.to_str() {
                None => "",
                Some(s) => s,
            },
        }
    }

    fn file_stem_as_str(&self) -> &str {
        match self.file_stem() {
            None => "",
            Some(osstr) => match osstr.to_str() {
                None => "",
                Some(s) => s,
            },
        }
    }

    fn directory_as_str(&self) -> &str {
        match self.parent() {
            None => "",
            Some(osstr) => match osstr.to_str() {
                None => "",
                Some(s) => s,
            },
        }
    }

    fn extension_as_str(&self) -> &str {
        //self.is_file() && self.extension().map_or(false, |s| s == "suo")

        match self.extension() {
            None => "",
            Some(osstr) => match osstr.to_str() {
                None => "",
                Some(s) => s,
            },
        }
    }

    /// Due to the awful situation on Windows, where paths embedded in project and solution files are
    /// often different in case to what is actually on disk, we perform most comparisons in a
    /// case-insensitive manner.
    fn eq_ignoring_case<P: AsRef<Path>>(&self, other: P) -> bool {
        unicase::eq_ascii(self.as_str(), other.as_ref().as_str())
    }

    fn is_same_dir<P: AsRef<Path>>(&self, other: P) -> bool {
        let p1 = self.parent().unwrap();
        let p2 = other.as_ref().parent().unwrap();
        p1.is_dir() && p2.is_dir() && p1.eq_ignoring_case(p2)
    }

    fn is_hidden_dir(&self) -> bool {
        self.is_dir() && self.filename_as_str().starts_with('.')
    }

    fn is_bin_or_obj_dir(&self) -> bool {
        let last_part = self.filename_as_str();
        self.is_dir() && (
            unicase::eq_ascii(last_part, "obj")
            || unicase::eq_ascii(last_part, "bin")
        )
    }

    fn is_test_results_dir(&self) -> bool {
        let last_part = self.filename_as_str();
        self.is_dir() && unicase::eq_ascii(last_part, "TestResults")
    }

    fn is_packages_dir(&self) -> bool {
        let last_part = self.filename_as_str();
        self.is_dir() && unicase::eq_ascii(last_part, "packages")
    }

    fn is_node_modules_dir(&self) -> bool {
        let last_part = self.filename_as_str();
        self.is_dir() && unicase::eq_ascii(last_part, "node_modules")
    }

    fn is_git_dir(&self) -> bool {
        let last_part = self.filename_as_str();
        self.is_dir() && unicase::eq_ascii(last_part, ".git")
    }

    fn is_solution_info_file(&self) -> bool {
        let last_part = self.filename_as_str();
        self.is_file() && unicase::eq_ascii(last_part, "SolutionInfo.cs")
    }

    fn is_version_out_file(&self) -> bool {
        let last_part = self.filename_as_str();
        self.is_file() && unicase::eq_ascii(last_part, "VERSION.txt.out")
    }

    fn is_sln_file(&self) -> bool {
        let ext = self.extension_as_str();
        self.is_file() && unicase::eq_ascii(ext, "sln")
    }

    fn is_csproj_file(&self) -> bool {
        let ext = self.extension_as_str();
        self.is_file() && unicase::eq_ascii(ext, "csproj")
    }

    fn is_suo_file(&self) -> bool {
        let ext = self.extension_as_str();
        self.is_file() && unicase::eq_ascii(ext, "suo")
    }

    fn is_upgrade_log_file(&self) -> bool {
        let last_part = self.filename_as_str();
        self.is_file() && unicase::eq_ascii(last_part, "UpgradeLog.htm")
    }

    fn is_git_orig_file(&self) -> bool {
        let ext = self.extension_as_str();
        self.is_file() && unicase::eq_ascii(ext, "orig")
    }

    fn is_mef_cache_dir(&self) -> bool {
        let last_part = self.filename_as_str();
        self.is_dir() && unicase::eq_ascii(last_part, "ComponentModelCache")
    }

    fn is_jet_brains_cache_dir(&self) -> bool {
        let last_part = self.filename_as_str();
        self.is_dir() && unicase::eq_ascii(last_part, "SolutionCaches")
    }
}

/// Return the home directory. Ok to panic if we cannot determine it.
/// Note that we do this lazily (not all code paths call this function).
pub fn home_dir() -> PathBuf {
    dirs::home_dir().expect("Cannot determine your home directory")
}

pub fn make_path_under_home_dir(sub_path: &str) -> PathBuf {
    let mut p = home_dir();
    p.push(sub_path);
    p
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    pub fn eq_ignoring_case() {
        let p1 = PathBuf::from("");
        let p2 = PathBuf::from("");
        assert!(p1.eq_ignoring_case(p2));

        let p1 = PathBuf::from("a");
        let p2 = PathBuf::from("A");
        assert!(p1.eq_ignoring_case(p2));

        let p1 = PathBuf::from("a");
        let p2 = PathBuf::from("b");
        assert!(!p1.eq_ignoring_case(p2));

        let p1 = PathBuf::from(r"a\b\c");
        let p2 = PathBuf::from(r"A\B\c");
        assert!(p1.eq_ignoring_case(p2));
    }
}
