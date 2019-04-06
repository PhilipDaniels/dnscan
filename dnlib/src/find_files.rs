use std::path::{Path, PathBuf};
use std::str::FromStr;
use walkdir::{DirEntry, WalkDir};
use crate::path_extensions::PathExtensions;
use crate::dn_error::DnLibResult;
use crate::enums::InterestingFile;

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
