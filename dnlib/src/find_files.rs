use std::path::{Path, PathBuf};
use std::str::FromStr;
use walkdir::{DirEntry, WalkDir};
use crate::path_extensions::PathExtensions;
use crate::dn_error::DnLibResult;
use crate::interesting_file::InterestingFile;

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

#[cfg(test)]
mod tests {
    use super::*;

    // 0 solutions
    // 2 solutions in different dirs
    // 2 solutions in the same dir


    // fn make_pta() -> PathsToAnalyze {
    //     let mut input = PathsToAnalyze::default();
    //     input.csproj_files.push(Path::new("/temp/foo.csproj").to_owned());
    //     input.other_files.push(Path::new("/temp/app.config").to_owned());
    //     input.other_files.push(Path::new("/temp/web.config").to_owned());
    //     input.other_files.push(Path::new("/wherever/web.config").to_owned());
    //     input
    // }

    // #[test]
    // pub fn get_other_files_in_dir_for_empty() {
    //     let input = PathsToAnalyze::default();
    //     let result = input.get_other_files_in_dir(Path::new("/temp"));
    //     assert!(result.is_empty());
    // }

    // #[test]
    // pub fn get_other_files_in_dir_for_no_other_files() {
    //     let mut input = PathsToAnalyze::default();
    //     input.csproj_files.push(Path::new("/temp/foo.csproj").to_owned());
    //     let result = input.get_other_files_in_dir(Path::new("/temp"));
    //     assert!(result.is_empty());
    // }

    // #[test]
    // pub fn get_other_files_in_dir_for_some_other_files() {
    //     let input = make_pta();
    //     let result = input.get_other_files_in_dir(Path::new("/temp"));
    //     assert_eq!(result, vec![Path::new("/temp/app.config"), Path::new("/temp/web.config")])
    // }

    // #[test]
    // pub fn project_has_other_file_for_no_other_file() {
    //     let input = PathsToAnalyze::default();
    //     let result = input.project_has_other_file(Path::new("/temp"), InterestingFile::AppConfig);
    //     assert!(!result);
    // }

    // #[test]
    // pub fn project_has_other_file_for_other_file_of_same_case() {
    //     let input = make_pta();
    //     let result = input.project_has_other_file(Path::new("/temp/foo.csproj"), InterestingFile::AppConfig);
    //     assert!(result);
    // }

    // #[test]
    // pub fn project_has_other_file_for_other_file_of_different_case() {
    //     let mut input = make_pta();
    //     input.other_files.remove(0); // nasty, assumes app.config is first item.
    //     input.other_files.push(Path::new("/temp/App.config").to_owned());
    //     let result = input.project_has_other_file(Path::new("/temp/foo.csproj"), InterestingFile::AppConfig);
    //     assert!(result);
    // }
}