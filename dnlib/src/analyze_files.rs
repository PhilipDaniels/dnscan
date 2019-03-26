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
    pub scanned_directories: Vec<SolutionDirectory>,
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
        self.scanned_directories.sort();
        for sd in &mut self.scanned_directories {
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
        println!("Adding sln for path = {:?}", path);
        let sln = Solution::new(path, file_loader);
        println!("Adding sln for path = {:?}", path);
        let sln_dir = path.parent().unwrap();
        println!("Adding sln for sln_dir = {:?}", sln_dir);

        // let finder = self.scanned_directories
        //     .iter_mut()
        //     .find(|dir| dir.directory == sln_dir);
        // let mut sdx = match finder {
        //     Some(a) => a,
        //     None => SolutionDirectory::new(sln_dir)
        // };

        for item in &mut self.scanned_directories {
            if item.directory == sln_dir {
                println!("Pushing sln {:#?}", sln);
                item.sln_files.push(sln);
                return;
            }
        }

        let mut sd = SolutionDirectory::new(sln_dir);
        println!("Pushing sd {:#?} for sln_dir {:#?}", sd, sln_dir);
        sd.sln_files.push(sln); // TODO call this field 'Solutions'
        self.scanned_directories.push(sd);
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
        for sd in &mut self.scanned_directories {
            let matching_sln = sd.sln_files.iter_mut().find(|sln| sln.refers_to_project(&project_path));
            if matching_sln.is_some() { return matching_sln; }
        }

        None
    }

    fn find_orphaned_solution<P>(&mut self, project_path: P) -> Option<&mut Solution>
    where
        P: AsRef<Path>,
    {
        for sd in &mut self.scanned_directories {
            let matching_sln = sd.sln_files.iter_mut().find(|sln| sln.file_info.path.is_same_dir(&project_path));
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
    pub sln_files: Vec<Solution>,
}

impl SolutionDirectory {
    fn new<P: AsRef<Path>>(sln_directory: P) -> Self {
        SolutionDirectory {
            directory: sln_directory.as_ref().to_owned(),
            sln_files: vec![]
        }
    }

    pub fn sort(&mut self) {
        self.sln_files.sort();
        for sf in &mut self.sln_files {
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


    // TODO: How to make paths and tests work on Unix and Windows?!

    #[test]
    pub fn for_one_sln_in_one_dir() {
        let analyzed_files = analyze(vec![r"C:\temp\foo.sln"]);
        println!("AF = {:#?}", analyzed_files);

        assert_eq!(analyzed_files.scanned_directories.len(), 1);
        assert_eq!(analyzed_files.scanned_directories[0].directory, PathBuf::from(r"C:\temp"));
        assert_eq!(analyzed_files.scanned_directories[0].sln_files.len(), 1);
        assert_eq!(analyzed_files.scanned_directories[0].sln_files[0].file_info.path, PathBuf::from(r"C:\temp\foo.sln"));
    }

    #[test]
    pub fn for_two_slns_in_one_dir() {
        let analyzed_files = analyze(vec![r"C:\temp\foo.sln", r"C:\temp\foo2.sln"]);
        println!("AF = {:#?}", analyzed_files);

        assert_eq!(analyzed_files.scanned_directories.len(), 1);
        assert_eq!(analyzed_files.scanned_directories[0].directory, PathBuf::from(r"C:\temp"));
        assert_eq!(analyzed_files.scanned_directories[0].sln_files.len(), 2);
        assert_eq!(analyzed_files.scanned_directories[0].sln_files[0].file_info.path, PathBuf::from(r"C:\temp\foo.sln"));
        assert_eq!(analyzed_files.scanned_directories[0].sln_files[1].file_info.path, PathBuf::from(r"C:\temp\foo2.sln"));
    }

    #[test]
    pub fn for_three_slns_in_two_dirs_and_sorts_solution_directories() {
        let analyzed_files = analyze(vec![r"C:\temp\foo.sln", r"C:\temp\foo2.sln", r"C:\blah\foo3.sln"]);
        println!("AF = {:#?}", analyzed_files);

        assert_eq!(analyzed_files.scanned_directories.len(), 2);

        assert_eq!(analyzed_files.scanned_directories[0].directory, PathBuf::from(r"C:\blah"));
        assert_eq!(analyzed_files.scanned_directories[0].sln_files.len(), 1);
        assert_eq!(analyzed_files.scanned_directories[0].sln_files[0].file_info.path, PathBuf::from(r"C:\blah\foo3.sln"));

        assert_eq!(analyzed_files.scanned_directories[1].directory, PathBuf::from(r"C:\temp"));
        assert_eq!(analyzed_files.scanned_directories[1].sln_files.len(), 2);
        assert_eq!(analyzed_files.scanned_directories[1].sln_files[0].file_info.path, PathBuf::from(r"C:\temp\foo.sln"));
        assert_eq!(analyzed_files.scanned_directories[1].sln_files[1].file_info.path, PathBuf::from(r"C:\temp\foo2.sln"));
    }

    #[test]
    pub fn for_one_orphaned_project() {
        let analyzed_files = analyze(vec![r"C:\temp\foo.sln", r"C:\temp\p1.csproj"]);
        println!("AF = {:#?}", analyzed_files);

        assert_eq!(analyzed_files.scanned_directories.len(), 1);
        assert_eq!(analyzed_files.scanned_directories[0].directory, PathBuf::from(r"C:\temp"));
        assert_eq!(analyzed_files.scanned_directories[0].sln_files.len(), 1);
        assert_eq!(analyzed_files.scanned_directories[0].sln_files[0].file_info.path, PathBuf::from(r"C:\temp\foo.sln"));

        let sln_file = &analyzed_files.scanned_directories[0].sln_files[0];
        assert_eq!(sln_file.linked_projects.len(), 0);
        assert_eq!(sln_file.orphaned_projects.len(), 1);
    }
}
