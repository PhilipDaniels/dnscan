use crate::dn_error::DnLibResult;
use crate::file_info::FileInfo;
use crate::file_loader::{DiskFileLoader, FileLoader};
use crate::find_files::find_files;
use crate::git_info::GitInfo;
use crate::project::Project;
use crate::find_files::PathsToAnalyze;
use crate::visual_studio_version::VisualStudioVersion;
use crate::path_extensions::PathExtensions;

use lazy_static::lazy_static;
use regex::{Regex, RegexBuilder};
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
        let pta = find_files(&path)?;
        AnalyzedFiles::inner_new(path, pta, DiskFileLoader::default())
    }

    pub fn sort(&mut self) {
        self.solution_directories.sort();
        for sd in &mut self.solution_directories {
            sd.sort();
        }
    }

    pub fn num_solutions(&self) -> usize {
        self.solution_directories.iter()
            .map(|sln_dir| sln_dir.num_solutions())
            .sum()
    }

    pub fn num_projects(&self) -> usize {
        self.solution_directories.iter()
            .map(|sln_dir| sln_dir.num_projects())
            .sum()
    }

    pub fn is_empty(&self) -> bool {
        self.solution_directories.is_empty()
    }

    /// The actual guts of `new`, using a file loader so we can test it.
    fn inner_new<P, L>(path: P, paths_to_analyze: PathsToAnalyze, file_loader: L) -> DnLibResult<Self>
    where
        P: AsRef<Path>,
        L: FileLoader,
    {
        println!("PTA = {:#?}", paths_to_analyze);

        // Group the files from the disk walk into our structure.
        // Load and analyze each solution and place them into folders.
        let mut files = AnalyzedFiles::default();
        for sln_path in &paths_to_analyze.sln_files {
            files.add_solution(sln_path, &file_loader);
        }

        // For each project, grab all the 'other' files in the same directory.
        // (This is very hacky. Assumes they are all in the project directory! Can fix by replacing
        // the '==' with a closure).
        // Then analyze each project.
        // TODO: This needs to be in parallel.

    // let file_loader = DiskFileLoader::new();

    // let (elapsed, solutions) = measure_time(|| {
    //     paths.sln_files.par_iter().map(|path| {
    //         Solution::new(path, &file_loader)
    //     }).collect::<Vec<_>>()
    // });

    // if options.verbose {
    //     println!("{} Solutions loaded and analyzed in {}", solutions.len(), elapsed);
    // }

    // let (elapsed, projects) = measure_time(|| {
    //     paths.csproj_files.par_iter().map(|path| {
    //         Project::new(path, &paths, &file_loader)
    //     }).collect::<Vec<_>>()
    // });

        let analyzed_projects = paths_to_analyze.csproj_files
            .iter()
            .map(|proj_path| {
                let other_paths = paths_to_analyze.other_files.iter()
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

        for item in &mut self.solution_directories {
            if item.directory == sln_dir {
                item.solutions.push(sln);
                return;
            }
        }

        let mut sd = SolutionDirectory::new(sln_dir);
        sd.solutions.push(sln);
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

    pub fn num_solutions(&self) -> usize {
        self.solutions.len()
    }

    pub fn num_projects(&self) -> usize {
        self.solutions.iter()
            .map(|sln| sln.num_projects())
            .sum()
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
    /// This field is populated from the directory walk, not the solution contents.
    pub linked_projects: Vec<Project>,

    /// The set of projects that are related to this solution, in that they exist
    /// exist on disk in the same directory or a subdirectory of the solution
    /// directory, but they are not referenced from inside the .sln file.
    /// (Probably they are projects that you forgot to delete).
    /// This field is populated from the directory walk, not the solution contents.
    pub orphaned_projects: Vec<Project>,

    /// The set of projects that is mentioned inside the sln file.
    /// This is populated by reading the solution file and normalizing
    /// the extracted paths.
    mentioned_projects: Vec<PathBuf>
}

impl Solution {
    pub fn new<P, L>(path: P, file_loader: &L) -> Self
    where
        P: AsRef<Path>,
        L: FileLoader,
    {
        let fi = FileInfo::new(path, file_loader);
        let ver = VisualStudioVersion::extract(&fi.contents).unwrap_or_default();
        let mp = Self::extract_mentioned_projects(&fi.path, &fi.contents);

        Solution {
            file_info: fi,
            version: ver,
            mentioned_projects: mp,
            ..Default::default()
        }
    }

    fn sort(&mut self) {
        self.linked_projects.sort();
        self.orphaned_projects.sort();
    }

    fn num_projects(&self) -> usize {
        self.linked_projects.len() + self.orphaned_projects.len()
    }

    /// Extracts the projects from the contents of the solution file. Note that there is
    /// a potential problem here, in that the paths constructed will be in the format
    /// of the system that the solution was created on (e.g. Windows) and not the
    /// format of the system the program is running on (e.g. Linux).
    /// See also `refers_to_project` where this surfaces.
    fn extract_mentioned_projects<P: AsRef<Path>>(path: P, contents: &str) -> Vec<PathBuf> {
        lazy_static! {
            static ref PROJECT_RE: Regex = RegexBuilder::new(r##""(?P<projpath>[^"]+csproj)"##)
                                         .case_insensitive(true).build().unwrap();
        }

        let mut project_paths = PROJECT_RE
            .captures_iter(contents)
            .map(|cap| {
                let mut path = path.as_ref().parent().unwrap().to_owned();
                path.push(cap["projpath"].to_owned());
                path
            })
            .collect::<Vec<_>>();

        project_paths.sort();
        project_paths.dedup();
        project_paths
    }

    fn refers_to_project<P: AsRef<Path>>(&self, project_path: P) -> bool {
        let project_path = project_path.as_ref();
        self.mentioned_projects.iter().any(|mp| mp == project_path)
    }
}

#[cfg(test)]
mod analyzed_files_tests {
    use super::*;
    use crate::file_loader::MemoryFileLoader;

    // We have to use a real file system for these tests because of the directory walk (which
    // can be fairly easily factored out) and the PathExtensions tests (which cannot).
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

    /// This function can be used when we are just dealing with paths and their relationships.
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
        let file_loader = MemoryFileLoader::new();
        AnalyzedFiles::inner_new("C:\temp", pta, file_loader).unwrap()
    }

    /// This function can be used when the tests need the files to have some contents.
    fn analyze2<P: AsRef<Path>>(paths: Vec<(P, &str)>) -> AnalyzedFiles {
        let mut pta = PathsToAnalyze::default();
        let mut file_loader = MemoryFileLoader::new();

        for p in &paths {
            let contents = p.1.to_owned();
            let p = p.0.as_ref().to_owned();
            file_loader.files.insert(p.clone(), contents);

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
        AnalyzedFiles::inner_new("C:\temp", pta, file_loader).unwrap()
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

    #[test]
    pub fn for_one_mentioned_project() {
        let analyzed_files = analyze2(vec![
            (tp(r"C:\temp\foo.sln"), r##""p1.csproj""##),
            (tp(r"C:\temp\p1.csproj"), ""),
        ]);
        println!("AF = {:#?}", analyzed_files);

        assert_eq!(analyzed_files.solution_directories.len(), 1);
        assert_eq!(analyzed_files.solution_directories[0].directory, tp(r"C:\temp"));
        assert_eq!(analyzed_files.solution_directories[0].solutions.len(), 1);
        assert_eq!(analyzed_files.solution_directories[0].solutions[0].file_info.path, tp(r"C:\temp\foo.sln"));

        let sln_file = &analyzed_files.solution_directories[0].solutions[0];
        assert_eq!(sln_file.orphaned_projects.len(), 0);
        assert_eq!(sln_file.linked_projects.len(), 1);
        assert_eq!(sln_file.linked_projects[0].file_info.path, tp(r"C:\temp\p1.csproj"));
    }

    #[test]
    pub fn for_two_mentioned_projects() {
        let analyzed_files = analyze2(vec![
            (tp(r"C:\temp\foo.sln"), r##""p1.csproj"
                                         "p2.csproj"
                                     "##),
            (tp(r"C:\temp\p1.csproj"), ""),
            (tp(r"C:\temp\p2.csproj"), ""),
        ]);
        println!("AF = {:#?}", analyzed_files);

        assert_eq!(analyzed_files.solution_directories.len(), 1);
        assert_eq!(analyzed_files.solution_directories[0].directory, tp(r"C:\temp"));
        assert_eq!(analyzed_files.solution_directories[0].solutions.len(), 1);
        assert_eq!(analyzed_files.solution_directories[0].solutions[0].file_info.path, tp(r"C:\temp\foo.sln"));

        let sln_file = &analyzed_files.solution_directories[0].solutions[0];
        assert_eq!(sln_file.orphaned_projects.len(), 0);
        assert_eq!(sln_file.linked_projects.len(), 2);
        assert_eq!(sln_file.linked_projects[0].file_info.path, tp(r"C:\temp\p1.csproj"));
        assert_eq!(sln_file.linked_projects[1].file_info.path, tp(r"C:\temp\p2.csproj"));
    }

    #[cfg(not(Windows))]
    #[test]
    pub fn for_two_mentioned_projects_on_linux() {
        let analyzed_files = analyze2(vec![
            (r"/temp/foo.sln", r##""p1.csproj"
                                   "sub/sub/p2.csproj"
                               "##),
            (r"/temp/p1.csproj", ""),
            (r"/temp/sub/sub/p2.csproj", ""),
        ]);
        println!("AF = {:#?}", analyzed_files);

        assert_eq!(analyzed_files.solution_directories.len(), 1);
        assert_eq!(analyzed_files.solution_directories[0].directory, tp(r"/temp"));
        assert_eq!(analyzed_files.solution_directories[0].solutions.len(), 1);
        assert_eq!(analyzed_files.solution_directories[0].solutions[0].file_info.path, tp(r"/temp\foo.sln"));

        let sln_file = &analyzed_files.solution_directories[0].solutions[0];
        assert_eq!(sln_file.orphaned_projects.len(), 0);
        assert_eq!(sln_file.linked_projects.len(), 2);
        assert_eq!(sln_file.linked_projects[0].file_info.path, tp(r"/temp/p1.csproj"));
        assert_eq!(sln_file.linked_projects[1].file_info.path, tp(r"/temp/sub/sub/p2.csproj"));
    }
}
