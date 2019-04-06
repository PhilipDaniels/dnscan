use crate::dn_error::DnLibResult;
use crate::file_info::FileInfo;
use crate::git_info::GitInfo;
use crate::project::Project;
use crate::enums::*;
use crate::io::{PathExtensions, PathsToAnalyze, DiskFileLoader, find_files, FileLoader};
use crate::configuration::Configuration;

use lazy_static::lazy_static;
use regex::{Regex, RegexBuilder};
use rayon::prelude::*;
use std::path::{Path, PathBuf};
use std::time::{self, Duration};

/// The set of all files found during analysis.
#[derive(Debug, Default)]
pub struct Analysis {
    pub paths_analyzed: PathsToAnalyze,
    pub solution_directories: Vec<SolutionDirectory>,
    pub disk_walk_duration: Duration,
    pub solution_load_duration: Duration,
    pub project_load_duration: Duration,
}

impl Analysis {
    pub fn new<P>(path: P, configuration: &Configuration) -> DnLibResult<Self>
    where
        P: AsRef<Path>,
    {
        let disk_walk_start_time = time::Instant::now();
        let pta = find_files(&path)?;

        let mut af = Self {
            paths_analyzed: pta,
            disk_walk_duration: disk_walk_start_time.elapsed(),
            ..Default::default()
        };

        let fs_loader = DiskFileLoader::default();
        af.analyze(configuration, fs_loader)?;
        Ok(af)
    }

    pub fn sort(&mut self) {
        self.solution_directories.sort();
        for sd in &mut self.solution_directories {
            sd.sort();
        }
    }

    pub fn is_empty(&self) -> bool {
        self.solution_directories.is_empty()
    }

    pub fn num_solutions(&self) -> usize {
        self.solution_directories.iter()
            .map(|sln_dir| sln_dir.num_solutions())
            .sum()
    }

    pub fn num_linked_projects(&self) -> usize {
        self.solution_directories.iter()
            .map(|sln_dir| sln_dir.num_linked_projects())
            .sum()
    }

    pub fn num_orphaned_projects(&self) -> usize {
        self.solution_directories.iter()
            .map(|sln_dir| sln_dir.num_orphaned_projects())
            .sum()
    }

    /// The actual guts of `new`, using a file loader so we can test it.
    fn analyze<L>(&mut self, configuration: &Configuration, file_loader: L) -> DnLibResult<()>
    where L: FileLoader + std::marker::Sync
    {
        // Load and analyze each solution and place them into folders.
        let solution_analysis_start_time = time::Instant::now();

        let solutions = self.paths_analyzed.sln_files.par_iter()
            .map(|sln_path| {
                Solution::new(sln_path, &file_loader.clone())
            }).collect::<Vec<_>>();

        for sln in solutions {
            self.add_solution(sln);
        }

        self.solution_load_duration = solution_analysis_start_time.elapsed();


        // For each project, grab all the 'other' files in the same directory.
        // (This is very hacky. Assumes they are all in the project directory! Can fix by replacing
        // the '==' with a closure). Then analyze the project itself.
        let project_analysis_start_time = time::Instant::now();

        let projects = self.paths_analyzed.csproj_files.par_iter()
            .map(|proj_path| {
                let other_paths = self.paths_analyzed.other_files.iter()
                    .filter(|&other_path| other_path.is_same_dir(proj_path))
                    .cloned()
                    .collect::<Vec<_>>();

                Project::new(proj_path, other_paths, &file_loader.clone(), configuration)
            })
            .collect::<Vec<_>>();

        for proj in projects {
            self.add_project(proj);
        }

        self.project_load_duration = project_analysis_start_time.elapsed();


        self.sort();
        Ok(())
    }

    fn add_solution(&mut self, sln: Solution) {
        let sln_dir = sln.file_info.path.parent().unwrap();

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

    fn add_project(&mut self, mut project: Project) {
        if let Some(ref mut sln) = self.find_linked_solution(&project.file_info.path) {
            project.ownership = ProjectOwnership::Linked;
            sln.projects.push(project);
        } else if let Some(ref mut sln) = self.find_orphaned_solution(&project.file_info.path) {
            project.ownership = ProjectOwnership::Orphaned;
            sln.projects.push(project);
        } else if let Some(ref mut sln) = self.find_orphaned_solution_in_parent_dir(&project.file_info.path) {
            project.ownership = ProjectOwnership::Orphaned;
            sln.projects.push(project);
        } else {
            eprintln!("Could not associate project {:?} with a solution, ignoring.", &project.file_info.path);
        }
    }

    /// Scan all known solutions trying to find one that refers to the specified
    /// project path. Works as a pair with `find_orphaned_solution` - I had to
    /// create three functions to get around the borrow checker.
    /// TODO: Merge this into 1 function.
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
        // 'is_same_dir' takes the parent of both paths and checks that they are both actually
        // directories before comparing the parents by case.
        // Therefore this will fail for all tests if there is no file on disk!

        // Try and associate orphaned projects with any solutions that are in the same directory.
        for sd in &mut self.solution_directories {
            let matching_sln = sd.solutions.iter_mut().find(|sln| sln.file_info.path.is_same_dir(&project_path));
            if matching_sln.is_some() { return matching_sln; }
        }

        None
    }

    fn find_orphaned_solution_in_parent_dir<P>(&mut self, project_path: P) -> Option<&mut Solution>
    where
        P: AsRef<Path>,
    {
        // Try and associate orphaned projects with any solutions that are in the parent directory.
        let parent_dir = project_path.as_ref().parent().unwrap();
        for sd in &mut self.solution_directories {
            let matching_sln = sd.solutions.iter_mut().find(|sln| sln.file_info.path.is_same_dir(&parent_dir));
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

impl<P> From<P> for SolutionDirectory
where P: Into<PathBuf>
{
    fn from(sln_directory: P) -> Self {
        SolutionDirectory {
            directory: sln_directory.into(),
            solutions: vec![]
        }
    }
}


#[cfg(test)]
mod tests2 {
    use super::*;
    #[test]
    pub fn x() {
        let p = PathBuf::from("s");
        let s: SolutionDirectory = "somepath".into();
    }
}

impl SolutionDirectory {
    fn new<P: Into<PathBuf>>(sln_directory: P) -> Self {
        SolutionDirectory {
            directory: sln_directory.into(),
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

    pub fn num_linked_projects(&self) -> usize {
        self.solutions.iter()
            .map(|sln| sln.linked_projects().count())
            .sum()
    }

    pub fn num_orphaned_projects(&self) -> usize {
        self.solutions.iter()
            .map(|sln| sln.orphaned_projects().count())
            .sum()
    }
}

#[derive(Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
/// Represents a sln file and any projects that are associated with it.
pub struct Solution {
    pub file_info: FileInfo,
    pub version: VisualStudioVersion,
    pub git_info: GitInfo,

    // The set of projects that we found during the disk walk and have loaded and
    // associated with this solution (either by explicit linkage because they are
    // mentioned in the .sln file, or by assumed-orphanship because they are in
    // the same directory, but no longer in the solution).
    pub projects: Vec<Project>,

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
        let fi = FileInfo::new(path.as_ref(), file_loader);
        let ver = VisualStudioVersion::extract(&fi.contents).unwrap_or_default();
        let sln_dir = fi.path.parent().unwrap().to_owned();
        let mp = Self::extract_mentioned_projects(sln_dir, &fi.contents);

        Solution {
            file_info: fi,
            version: ver,
            mentioned_projects: mp,
            ..Default::default()
        }
    }

    fn sort(&mut self) {
        self.projects.sort();
    }

    pub fn linked_projects(&self) -> impl Iterator<Item = &Project> {
        self.projects.iter().filter(|p| p.ownership == ProjectOwnership::Linked)
    }

    pub fn orphaned_projects(&self) -> impl Iterator<Item = &Project> {
        self.projects.iter().filter(|p| p.ownership == ProjectOwnership::Orphaned)
    }

    /// Extracts the projects from the contents of the solution file. Note that there is
    /// a potential problem here, in that the paths constructed will be in the format
    /// of the system that the solution was created on (e.g. Windows) and not the
    /// format of the system the program is running on (e.g. Linux).
    /// See also `refers_to_project` where this surfaces.
    fn extract_mentioned_projects(sln_dir: PathBuf, contents: &str) -> Vec<PathBuf> {
        lazy_static! {
            static ref PROJECT_RE: Regex = RegexBuilder::new(r#""(?P<projpath>[^"]+csproj)"#)
                .case_insensitive(true).build().unwrap();
        }

        let mut project_paths = PROJECT_RE.captures_iter(contents)
            .map(|cap| {
                let mut path = sln_dir.clone();
                let x = Self::norm_mentioned_path(&cap["projpath"]);
                path.push(x);
                path
            })
            .collect::<Vec<_>>();

        project_paths.sort();
        project_paths.dedup();
        project_paths
    }

    fn refers_to_project<P: AsRef<Path>>(&self, project_path: P) -> bool {
        let project_path = project_path.as_ref();
        self.mentioned_projects.iter().any(|mp| mp.eq_ignoring_case(project_path))
    }

    /// Convert this extracted path to a form that matches what is in use on
    /// the operating system the program is running on. Mentioned paths are
    /// always of the form "Dir\Foo.csproj" (in other words, even on Linux
    /// they use Windows-style slashes)
    #[cfg(windows)]
    fn norm_mentioned_path(mp: &str) -> String {
        mp.to_owned()
    }

    #[cfg(not(windows))]
    fn norm_mentioned_path(mp: &str) -> String {
        mp.replace('\\', "/").to_owned()
    }
}

#[cfg(test)]
mod analyzed_files_tests {
    use super::*;
    use tempfile;
    use std::io::{self, Write};
    use std::fs::{self, File};
    use crate::io::PathExtensions;

    fn make_temporary_directory() -> io::Result<tempfile::TempDir> {
        let root = tempfile::Builder::new()
            .prefix("dnlib-temp-")
            .rand_bytes(5)
            .tempdir()?;

        let file_path = root.path().join("car.sln");
        let mut file = File::create(&file_path)?;

        // Slns always use Windows-style paths, even when using 'dotnet' on Linux.
        writeln!(file, r#"
                        "ford.csproj"
                        "sub\toyota.csproj"
                        "#)?;

        let file_path = root.path().join("ford.csproj");
        File::create(&file_path)?;
        let file_path = root.path().join("bmw.csproj");
        File::create(&file_path)?;

        let sub_dir = root.path().join("sub");
        fs::create_dir_all(&sub_dir)?;
        let file_path = sub_dir.join("toyota.csproj");
        File::create(file_path)?;

        // Trucks.
        let truck_dir = root.path().join("trucks");
        fs::create_dir_all(&truck_dir)?;

        let file_path = truck_dir.join("truck.sln");
        let mut file = File::create(&file_path)?;
        writeln!(file, r#"  "volvo.csproj"  "#)?;

        let file_path = truck_dir.join("volvo.csproj");
        File::create(&file_path)?;

        let file_path = truck_dir.join("mercedes.csproj");
        File::create(&file_path)?;

        let file_path = truck_dir.join("renault.csproj");
        File::create(&file_path)?;

        Ok(root)
    }

    #[test]
    pub fn test_disk_scanning_and_project_association() {
        let temp_files = make_temporary_directory().unwrap();
        let root_dir = temp_files.path();

        let analyzed_files = Analysis::new(
            root_dir,
            &Configuration::default()
            ).unwrap();

        //println!("AF = {:#?}", analyzed_files);

        assert_eq!(analyzed_files.solution_directories.len(), 2);

        let car_sln_dir = &analyzed_files.solution_directories[0];
        println!("car_sln_dir = {:#?}", car_sln_dir);
        assert_eq!(car_sln_dir.directory, root_dir);
        assert_eq!(car_sln_dir.num_solutions(), 1);
        assert_eq!(car_sln_dir.num_linked_projects(), 2);
        assert_eq!(car_sln_dir.num_orphaned_projects(), 1);
        let car_sln = &car_sln_dir.solutions[0];
        assert_eq!(car_sln.file_info.filename_as_str(), "car.sln");
        assert_eq!(car_sln.linked_projects().nth(0).unwrap().file_info.path.filename_as_str(), "ford.csproj");
        assert_eq!(car_sln.linked_projects().nth(1).unwrap().file_info.path.filename_as_str(), "toyota.csproj");
        // BMW is orphaned because not actually mentioned in the sln file.
        assert_eq!(car_sln.orphaned_projects().nth(0).unwrap().file_info.path.filename_as_str(), "bmw.csproj");


        let truck_sln_dir = &analyzed_files.solution_directories[1];
        println!("truck_sln_dir = {:#?}", truck_sln_dir);
        let expected_truck_dir = root_dir.join("trucks");
        assert_eq!(truck_sln_dir.directory, expected_truck_dir);
        assert_eq!(truck_sln_dir.num_solutions(), 1);
        assert_eq!(truck_sln_dir.num_linked_projects(), 1);
        assert_eq!(truck_sln_dir.num_orphaned_projects(), 2);
        let truck_sln = &truck_sln_dir.solutions[0];
        assert_eq!(truck_sln.file_info.filename_as_str(), "truck.sln");
        assert_eq!(truck_sln.linked_projects().nth(0).unwrap().file_info.path.filename_as_str(), "volvo.csproj");
        assert_eq!(truck_sln.orphaned_projects().nth(0).unwrap().file_info.path.filename_as_str(), "mercedes.csproj");
        assert_eq!(truck_sln.orphaned_projects().nth(1).unwrap().file_info.path.filename_as_str(), "renault.csproj");
    }
}
