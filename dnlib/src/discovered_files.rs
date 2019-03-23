use std::path::{Path, PathBuf};
use std::str::FromStr;
use walkdir::{DirEntry, WalkDir};
use crate::file_loader::{FileLoader, DiskFileLoader};
use crate::path_extensions::PathExtensions;
use crate::dn_error::DnLibResult;
use crate::interesting_file::InterestingFile;


/// The set of all files found during the directory walk-phase of analysis.
#[derive(Debug, Default, Clone, PartialOrd, Ord, PartialEq, Eq)]
pub struct DiscoveredFiles(Vec<SolutionDirectory>);

#[derive(Debug, Default, Clone, PartialOrd, Ord, PartialEq, Eq)]
/// Represents a directory that contains 1 or more solution files.
pub struct SolutionDirectory {
    /// The directory path, e.g. `C:\temp\my_solution`.
    pub directory: PathBuf,
    /// The sln files in this directory.
    pub sln_files: Vec<SolutionFile>
}

#[derive(Debug, Default, Clone, PartialOrd, Ord, PartialEq, Eq)]
/// Represents a sln file and any projects that are associated with it.
pub struct SolutionFile {
    /// The path of the sln file, e.g. `C:\temp\my_solution\foo.sln`.
    pub path: PathBuf,
    /// The set of projects that are linked to this solution. The project files
    /// must exist on disk in the same directory or a subdirectory of the solution
    /// directory, and be referenced from inside the .sln file.
    pub linked_projects: Vec<ProjectFile>,
    /// The set of projects that are related to this solution, in that they exist
    /// exist on disk in the same directory or a subdirectory of the solution
    /// directory, but they are not referenced from inside the .sln file.
    /// (Probably they are projects that you forgot to delete).
    pub orphaned_projects: Vec<ProjectFile>,
}

#[derive(Debug, Default, Clone, PartialOrd, Ord, PartialEq, Eq)]
/// Represents a single project file.
pub struct ProjectFile {
    /// The full path of the project file, e.g. `C:\temp\my_solution\project1\project1.csproj`.
    pub path: PathBuf,
    /// The set of other files that we scan for because we consider them interesting.
    /// Includes such things as `web.config` and `packages.config`.
    /// See `InterestingFiles` for the full list.
    pub other_files: Vec<PathBuf>
}


impl ProjectFile {
    fn sort(&mut self) {
        self.other_files.sort();
    }
}

impl SolutionDirectory {
    fn sort(&mut self) {
        self.sln_files.sort();
    }
}

impl SolutionFile {
    pub fn new<P>(path: P) -> Self
        where P: AsRef<Path>
    {
        SolutionFile {
            path: path.as_ref().to_owned(),
            ..Default::default()
        }
    }

    fn sort(&mut self) {
        self.linked_projects.sort();
        self.orphaned_projects.sort();
    }
}

impl DiscoveredFiles {
    pub fn new<P>(path: P) -> DnLibResult<Self>
        where P: AsRef<Path>
    {
        let file_loader = DiskFileLoader::new();
        DiscoveredFiles::inner_new(path, file_loader)
    }

    pub fn sort(&mut self) {
        self.0.sort();
        for sd in &mut self.0 {
            sd.sort();
        }
    }

    /// The actual guts of `new`, using a file loader so we can test it.
    fn inner_new<P, L>(path: P, file_loader: L) -> DnLibResult<Self>
        where P: AsRef<Path>,
              L: FileLoader
    {
        // First find all the paths of interest.
        let mut sln_files = vec![];
        let mut proj_files = vec![];
        let mut other_files = vec![];
        let walker = WalkDir::new(path);

        for entry in walker.into_iter().filter_entry(|e| continue_walking(e)) {
            let entry = entry?;
            let path = entry.path();

            if path.is_sln_file() {
                sln_files.push(path.to_owned());
            } else if path.is_csproj_file() {
                proj_files.push(path.to_owned());
            } else {
                let filename = path.filename_as_str();
                if is_file_of_interest(&filename) {
                    other_files.push(path.to_owned());
                }
            }
        }

        // Now group them into our structure.
        let mut files = DiscoveredFiles::default();
        for sln in sln_files {
            files.add_solution(sln);
        }

        for proj in proj_files {
            files.add_project(proj);
        }

        files.sort();
        Ok(files)
    }

    fn add_solution(&mut self, path: PathBuf) {
        let sln_dir = path.parent().unwrap();
        for item in &mut self.0 {
            if item.directory == sln_dir {
                item.sln_files.push(SolutionFile::new(path));
                return;
            }
        }
    }

    fn add_project(&mut self, path: PathBuf) {
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    // 0 solutions
    // 2 solutions in different dirs
    // 2 solutions in the same dir
}