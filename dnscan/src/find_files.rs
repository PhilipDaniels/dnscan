use crate::options::Options;
use dnlib::path_extensions::PathExtensions;
use std::path::PathBuf;
use walkdir::{DirEntry, WalkDir};

/// This struct is used to collect the raw directory walking results prior to further
/// analysis. It is basically just a list of paths of various types. No effort is made
/// to relate the csproj files to their owning sln files, for example (that requires)
/// probing inside the file contents and is left to a later stage of analysis).
#[derive(Debug, Default)]
pub struct PathsToAnalyze {
    pub sln_files: Vec<PathBuf>,
    pub csproj_files: Vec<PathBuf>,
}

impl PathsToAnalyze {
    pub fn is_empty(&self) -> bool {
        self.sln_files.is_empty() && self.csproj_files.is_empty()
    }
}

impl PathsToAnalyze {
    pub fn sort(&mut self) {
        self.sln_files.sort();
        self.csproj_files.sort();
    }
}

pub fn get_paths_of_interest(options: &Options) -> PathsToAnalyze {
    let mut paths = PathsToAnalyze::default();
    let walker = WalkDir::new(&options.dir);
    for entry in walker.into_iter().filter_entry(|e| continue_walking(e)) {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_sln_file() {
            paths.sln_files.push(path.to_owned());
        } else if path.is_csproj_file() {
            paths.csproj_files.push(path.to_owned());
        }
    }

    paths.sort();
    paths
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
