use crate::options::Options;
use crate::errors::AnalysisError;
use dnlib::path_extensions::PathExtensions;
use std::path::{Path, PathBuf};
use walkdir::{DirEntry, WalkDir};
use std::str::FromStr;

/// This struct is used to collect the raw directory walking results prior to further
/// analysis. It is basically just a list of paths of various types. No effort is made
/// to relate the csproj files to their owning sln files, for example (that requires)
/// probing inside the file contents and is left to a later stage of analysis).
#[derive(Debug, Default)]
pub struct PathsToAnalyze {
    pub sln_files: Vec<PathBuf>,
    pub csproj_files: Vec<PathBuf>,
    pub other_files: Vec<PathBuf>
}

impl PathsToAnalyze {
    pub fn is_empty(&self) -> bool {
        self.sln_files.is_empty() &&
        self.csproj_files.is_empty() &&
        self.other_files.is_empty()
    }
}

impl PathsToAnalyze {
    pub fn sort(&mut self) {
        self.sln_files.sort();
        self.csproj_files.sort();
        self.other_files.sort();
    }

    /// Checks to see whether a project has another file associated with it
    /// (i.e. that the other file actually exists on disk). This check is based on
    /// the directory of the project and the 'other_files'; we do not use the
    /// XML contents of the project file for this check. We are looking for actual
    /// physical files "in the expected places". This allows us to spot orphaned
    /// files that should have been deleted as part of project migration.
    pub fn project_has_other_file(&self, project: &Path, other_file: InterestingFile) -> bool {
        if let Some(project_dir) = project.parent() {
            let other_file = other_file.as_str();
            let possible_other_files = self.get_other_files_in_dir(project_dir);
            return possible_other_files.iter()
                .any(|other| other.filename_as_str().to_lowercase() == other_file);
        }

        false
    }

    pub fn get_other_files_in_dir(&self, directory: &Path) -> Vec<&PathBuf> {
        self.other_files.iter().filter(|path| match path.parent() {
            Some(dir) => dir == directory,
            None => false
        }).collect()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterestingFile {
    /// The web.config file.
    WebConfig,

    /// The app.config file.
    AppConfig,

    /// The appsettings.json file.
    AppSettingsJson,

    /// The package.json file (required by npm).
    PackageJson,

    /// The packages.config file (obsolete, should be removed)
    PackagesConfig,

    /// The project.json (obsolete, should be removed)
    ProjectJson
}

impl std::str::FromStr for InterestingFile {
    type Err = AnalysisError;

    fn from_str(s: &str) -> Result<InterestingFile, Self::Err> {
        let s = s.to_lowercase();
        match s.as_str() {
            "web.config" => Ok(InterestingFile::WebConfig),
            "app.config" => Ok(InterestingFile::AppConfig),
            "appsettings.json" => Ok(InterestingFile::AppSettingsJson),
            "package.json" => Ok(InterestingFile::PackageJson),
            "packages.config" => Ok(InterestingFile::PackagesConfig),
            "project.json" => Ok(InterestingFile::ProjectJson),
            _ => Err(AnalysisError::InvalidInterestingFile(s)),
        }
    }
}

impl InterestingFile {
    pub fn as_str(self) -> &'static str {
        match self {
            InterestingFile::WebConfig => "web.config",
            InterestingFile::AppConfig => "app.config",
            InterestingFile::AppSettingsJson => "appsettings.json",
            InterestingFile::PackageJson => "package.json",
            InterestingFile::PackagesConfig => "packages.config",
            InterestingFile::ProjectJson => "project.json"
        }
    }
}

fn is_file_of_interest(filename: &str) -> bool {
    InterestingFile::from_str(filename).is_ok()
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
        } else {
            let filename = path.filename_as_str();
            if is_file_of_interest(&filename) {
                paths.other_files.push(path.to_owned());
            }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn make_pta() -> PathsToAnalyze {
        let mut input = PathsToAnalyze::default();
        input.csproj_files.push(Path::new("/temp/foo.csproj").to_owned());
        input.other_files.push(Path::new("/temp/app.config").to_owned());
        input.other_files.push(Path::new("/temp/web.config").to_owned());
        input.other_files.push(Path::new("/wherever/web.config").to_owned());
        input
    }

    #[test]
    pub fn get_other_files_in_dir_for_empty() {
        let input = PathsToAnalyze::default();
        let result = input.get_other_files_in_dir(Path::new("/temp"));
        assert!(result.is_empty());
    }

    #[test]
    pub fn get_other_files_in_dir_for_no_other_files() {
        let mut input = PathsToAnalyze::default();
        input.csproj_files.push(Path::new("/temp/foo.csproj").to_owned());
        let result = input.get_other_files_in_dir(Path::new("/temp"));
        assert!(result.is_empty());
    }

    #[test]
    pub fn get_other_files_in_dir_for_some_other_files() {
        let input = make_pta();
        let result = input.get_other_files_in_dir(Path::new("/temp"));
        assert_eq!(result, vec![Path::new("/temp/app.config"), Path::new("/temp/web.config")])
    }

    #[test]
    pub fn project_has_other_file_for_no_other_file() {
        let input = PathsToAnalyze::default();
        let result = input.project_has_other_file(Path::new("/temp"), InterestingFile::AppConfig);
        assert!(!result);
    }

    #[test]
    pub fn project_has_other_file_for_other_file_of_same_case() {
        let input = make_pta();
        let result = input.project_has_other_file(Path::new("/temp/foo.csproj"), InterestingFile::AppConfig);
        assert!(result);
    }

    #[test]
    pub fn project_has_other_file_for_other_file_of_different_case() {
        let mut input = make_pta();
        input.other_files.remove(0); // nasty, assumes app.config is first item.
        input.other_files.push(Path::new("/temp/App.config").to_owned());
        let result = input.project_has_other_file(Path::new("/temp/foo.csproj"), InterestingFile::AppConfig);
        assert!(result);
    }
}
