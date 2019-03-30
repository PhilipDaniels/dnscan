use crate::dn_error::DnLibResult;
use crate::file_info::FileInfo;
use crate::file_loader::{DiskFileLoader, FileLoader};
use crate::find_files::find_files;
use crate::git_info::GitInfo;
use crate::project::Project;
use crate::find_files::PathsToAnalyze;
use crate::visual_studio_version::VisualStudioVersion;
use crate::path_extensions::PathExtensions;

use rayon::prelude::*;
use std::path::{Path, PathBuf};

/// The set of all files found during analysis.
#[derive(Debug, Default)]
pub struct AnalyzedFiles {
    pub solution_directories: Vec<SolutionDirectory>,
}

impl AnalyzedFiles {
    pub fn new<P>(path: P) -> DnLibResult<Self>
    where
        P: AsRef<Path>,
    {
        // First find all the paths of interest.
        let pta = find_files(&path)?;
        AnalyzedFiles::inner_new(path, pta, DiskFileLoader::default())
    }

    pub fn sort(&mut self) {
        self.solution_directories.sort();
        for sd in &mut self.solution_directories {
            sd.sort();
        }
    }

    /// The actual guts of `new`, using a file loader so we can test it.
    fn inner_new<P, L>(path: P, paths_to_analyze: PathsToAnalyze, file_loader: L) -> DnLibResult<Self>
    where
        P: AsRef<Path>,
        L: FileLoader,
    {
        // Now group them into our structure.
        // Load and analyze each solution and place them into folders.
        let mut files = AnalyzedFiles::default();
        for sln_path in &paths_to_analyze.sln_files {
            files.add_solution(sln_path, &file_loader);
        }

        // For each project, grab all the 'other' files in the same directory.
        // (This is very hacky. Assumes they are all in the project directory! Can fix by replacing
        // the '==' with a closure).
        // Then analyze each project.
        let analyzed_projects = paths_to_analyze
            .csproj_files
            .iter()
            .map(|proj_path| {
                let other_paths = paths_to_analyze
                    .other_files
                    .iter()
                    .filter(|&other_path| other_path.is_same_dir(proj_path))
                    .cloned()
                    .collect::<Vec<_>>();

                Project::new(proj_path, other_paths, &file_loader)
            })
            .collect::<Vec<_>>();

        for proj in analyzed_projects {
            files.add_project(proj);
        }

        files.sort();
        Ok(files)
    }

    fn add_solution<L: FileLoader>(&mut self, path: &PathBuf, file_loader: &L) {
        let sln = Solution::new(path, file_loader);
        let sln_dir = path.parent().unwrap();

        // let finder = self.scanned_directories
        //     .iter_mut()
        //     .find(|dir| dir.directory == sln_dir);
        // let mut sdx = match finder {
        //     Some(a) => a,
        //     None => SolutionDirectory::new(sln_dir)
        // };

        for item in &mut self.solution_directories {
            if item.directory == sln_dir {
                item.solutions.push(sln);
                return;
            }
        }

        let mut sd = SolutionDirectory::new(sln_dir);
        sd.solutions.push(sln); // TODO call this field 'Solutions'
        self.solution_directories.push(sd);
    }

    fn add_project(&mut self, project: Project) {
        if let Some(ref mut sln) = self.find_linked_solution(&project.file_info.path) {
            sln.linked_projects.push(project);
        } else if let Some(ref mut sln) = self.find_orphaned_solution(&project.file_info.path) {
            sln.orphaned_projects.push(project);
        } else {
            eprintln!("Could not associate project {:?} with a solution, ignoring.", &project.file_info.path);
        }
    }

    /// Scan all known solutions trying to find one that refers to the specified
    /// project path. Works as a pair with `find_orphaned_solution` - I had to
    /// create two functions to get around the borrow checker.
    fn find_linked_solution<P>(&mut self, project_path: P) -> Option<&mut Solution>
    where
        P: AsRef<Path>,
    {
        for sd in &mut self.solution_directories {
            let matching_sln = sd.solutions.iter_mut().find(|sln| sln.refers_to_project(&project_path));
            if matching_sln.is_some() { return matching_sln; }
        }

        None
    }

    fn find_orphaned_solution<P>(&mut self, project_path: P) -> Option<&mut Solution>
    where
        P: AsRef<Path>,
    {
        for sd in &mut self.solution_directories {
            let matching_sln = sd.solutions.iter_mut().find(|sln| sln.file_info.path.is_same_dir(&project_path));
            if matching_sln.is_some() { return matching_sln; }
        }

        None
    }
}


#[derive(Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
/// Represents a directory that contains 1 or more solution files.
pub struct SolutionDirectory {
    /// The directory path, e.g. `C:\temp\my_solution`.
    pub directory: PathBuf,

    /// The sln files in this directory.
    pub solutions: Vec<Solution>,
}

impl SolutionDirectory {
    fn new<P: AsRef<Path>>(sln_directory: P) -> Self {
        SolutionDirectory {
            directory: sln_directory.as_ref().to_owned(),
            solutions: vec![]
        }
    }

    pub fn sort(&mut self) {
        self.solutions.sort();
        for sf in &mut self.solutions {
            sf.sort();
        }
    }
}

#[derive(Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
/// Represents a sln file and any projects that are associated with it.
pub struct Solution {
    pub file_info: FileInfo,
    pub version: VisualStudioVersion,
    pub git_info: GitInfo,

    /// The set of projects that are linked to this solution. The project files
    /// must exist on disk in the same directory or a subdirectory of the solution
    /// directory, and be referenced from inside the .sln file.
    pub linked_projects: Vec<Project>,

    /// The set of projects that are related to this solution, in that they exist
    /// exist on disk in the same directory or a subdirectory of the solution
    /// directory, but they are not referenced from inside the .sln file.
    /// (Probably they are projects that you forgot to delete).
    pub orphaned_projects: Vec<Project>,
}

impl Solution {
    pub fn new<P, L>(path: P, file_loader: &L) -> Self
    where
        P: AsRef<Path>,
        L: FileLoader,
    {
        let fi = FileInfo::new(path, file_loader);
        let ver = VisualStudioVersion::extract(&fi.contents).unwrap_or_default();

        Solution {
            file_info: fi,
            version: ver,
            ..Default::default()
        }
    }

    fn sort(&mut self) {
        self.linked_projects.sort();
        self.orphaned_projects.sort();
    }

    fn refers_to_project<P: AsRef<Path>>(&self, project_path: P) -> bool {
        false
    }
}

#[cfg(test)]
mod analyzed_files_tests {
    use super::*;
    use crate::file_loader::MemoryFileLoader;
    use crate::path_extensions::PathExtensions;

    // We have to use a real file system for these tests because of the directory walk (which
    // can be fairly easily factored out) and the PathExtensions tests (which cannot).

    fn analyze<P: AsRef<Path>>(paths: Vec<P>) -> AnalyzedFiles {
        let mut pta = PathsToAnalyze::default();
        for p in &paths {
            let p = p.as_ref().to_owned();
            let ext = p.extension().unwrap();
            if ext == "sln" {
                pta.sln_files.push(p);
            } else if ext == "csproj" {
                pta.csproj_files.push(p);
            } else {
                pta.other_files.push(p);
            }
        }

        println!("pta = {:#?}", pta);
        let mut file_loader = MemoryFileLoader::new();
        AnalyzedFiles::inner_new("C:\temp", pta, file_loader).unwrap()
    }

    // `tp` = translate path - makes tests work on Windows and Linux.
    #[cfg(windows)]
    fn tp(path: &str) -> PathBuf {
        PathBuf::from(path)
    }

    #[cfg(not(windows))]
    fn tp(mut path: &str) -> PathBuf {
        if path.starts_with(r"C:\") {
            path = &path[3..];
        }

        let path = path.replace('\\', "/");
        PathBuf::from(path)
    }

    #[test]
    pub fn for_one_sln_in_one_dir() {
        let analyzed_files = analyze(vec![
            tp(r"C:\temp\foo.sln")
        ]);
        println!("AF = {:#?}", analyzed_files);

        assert_eq!(analyzed_files.solution_directories.len(), 1);
        assert_eq!(analyzed_files.solution_directories[0].directory, tp(r"C:\temp"));
        assert_eq!(analyzed_files.solution_directories[0].solutions.len(), 1);
        assert_eq!(analyzed_files.solution_directories[0].solutions[0].file_info.path, tp(r"C:\temp\foo.sln"));
    }

    #[test]
    pub fn for_two_slns_in_one_dir() {
        let analyzed_files = analyze(vec![
            tp(r"C:\temp\foo.sln"),
            tp(r"C:\temp\foo2.sln")
        ]);
        println!("AF = {:#?}", analyzed_files);

        assert_eq!(analyzed_files.solution_directories.len(), 1);
        assert_eq!(analyzed_files.solution_directories[0].directory, tp(r"C:\temp"));
        assert_eq!(analyzed_files.solution_directories[0].solutions.len(), 2);
        assert_eq!(analyzed_files.solution_directories[0].solutions[0].file_info.path, tp(r"C:\temp\foo.sln"));
        assert_eq!(analyzed_files.solution_directories[0].solutions[1].file_info.path, tp(r"C:\temp\foo2.sln"));
    }

    #[test]
    pub fn for_three_slns_in_two_dirs_and_sorts_solution_directories() {
        let analyzed_files = analyze(vec![
            tp(r"C:\temp\foo.sln"),
            tp(r"C:\temp\foo2.sln"),
            tp(r"C:\blah\foo3.sln")
        ]);
        println!("AF = {:#?}", analyzed_files);

        assert_eq!(analyzed_files.solution_directories.len(), 2);

        assert_eq!(analyzed_files.solution_directories[0].directory, tp(r"C:\blah"));
        assert_eq!(analyzed_files.solution_directories[0].solutions.len(), 1);
        assert_eq!(analyzed_files.solution_directories[0].solutions[0].file_info.path, tp(r"C:\blah\foo3.sln"));

        assert_eq!(analyzed_files.solution_directories[1].directory, tp(r"C:\temp"));
        assert_eq!(analyzed_files.solution_directories[1].solutions.len(), 2);
        assert_eq!(analyzed_files.solution_directories[1].solutions[0].file_info.path, tp(r"C:\temp\foo.sln"));
        assert_eq!(analyzed_files.solution_directories[1].solutions[1].file_info.path, tp(r"C:\temp\foo2.sln"));
    }

    #[test]
    pub fn for_one_orphaned_project() {
        let analyzed_files = analyze(vec![
            tp(r"C:\temp\foo.sln"),
            tp(r"C:\temp\p1.csproj")
        ]);
        println!("AF = {:#?}", analyzed_files);

        assert_eq!(analyzed_files.solution_directories.len(), 1);
        assert_eq!(analyzed_files.solution_directories[0].directory, tp(r"C:\temp"));
        assert_eq!(analyzed_files.solution_directories[0].solutions.len(), 1);
        assert_eq!(analyzed_files.solution_directories[0].solutions[0].file_info.path, tp(r"C:\temp\foo.sln"));

        let sln_file = &analyzed_files.solution_directories[0].solutions[0];
        assert_eq!(sln_file.linked_projects.len(), 0);
        assert_eq!(sln_file.orphaned_projects.len(), 1);
        assert_eq!(sln_file.orphaned_projects[0].file_info.path, tp(r"C:\temp\p1.csproj"));
    }

    #[test]
    pub fn for_multiple_orphaned_projects_including_sub_dirs() {
        let analyzed_files = analyze(vec![
            tp(r"C:\temp\foo.sln"),
            tp(r"C:\temp\p1.csproj"),
            tp(r"C:\temp\sub\sub.sln"),
            tp(r"C:\temp\sub\p2.csproj")
        ]);
        println!("AF = {:#?}", analyzed_files);

        assert_eq!(analyzed_files.solution_directories.len(), 2);
        assert_eq!(analyzed_files.solution_directories[0].directory, tp(r"C:\temp"));
        assert_eq!(analyzed_files.solution_directories[0].solutions.len(), 1);
        assert_eq!(analyzed_files.solution_directories[0].solutions[0].file_info.path, tp(r"C:\temp\foo.sln"));

        let sln_file = &analyzed_files.solution_directories[0].solutions[0];
        assert_eq!(sln_file.linked_projects.len(), 0);
        assert_eq!(sln_file.orphaned_projects.len(), 1);
        assert_eq!(sln_file.orphaned_projects[0].file_info.path, tp(r"C:\temp\p1.csproj"));

        assert_eq!(analyzed_files.solution_directories[1].directory, tp(r"C:\temp\sub"));
        assert_eq!(analyzed_files.solution_directories[1].solutions.len(), 1);
        assert_eq!(analyzed_files.solution_directories[1].solutions[0].file_info.path, tp(r"C:\temp\sub\sub.sln"));

        let sln_file = &analyzed_files.solution_directories[1].solutions[0];
        assert_eq!(sln_file.linked_projects.len(), 0);
        assert_eq!(sln_file.orphaned_projects.len(), 1);
        assert_eq!(sln_file.orphaned_projects[0].file_info.path, tp(r"C:\temp\sub\p2.csproj"));
    }

    // TODO: Need tests for csprojs that are mentioned in the solutions.
}
