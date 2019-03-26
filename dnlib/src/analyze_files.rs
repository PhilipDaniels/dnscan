use std::path::{Path, PathBuf};
use rayon::prelude::*;
use crate::file_loader::{FileLoader, DiskFileLoader};
use crate::dn_error::DnLibResult;
use crate::find_files::find_files;
use crate::file_info::FileInfo;
use crate::visual_studio_version::VisualStudioVersion;
use crate::git_info::GitInfo;
use crate::project::Project;

/// The set of all files found during analysis.
#[derive(Debug, Default)]
pub struct AnalyzedFiles {
    pub scanned_directories: Vec<SolutionDirectory>
}

pub enum SolutionMatchType {
    Linked,
    Orphaned
}

impl AnalyzedFiles {
    pub fn new<P>(path: P) -> DnLibResult<Self>
        where P: AsRef<Path>
    {
        let file_loader = DiskFileLoader::default();
        AnalyzedFiles::inner_new(path, &file_loader)
    }

    // pub fn sort(&mut self) {
    //     self.0.sort();
    //     for sd in &mut self.0 {
    //         sd.sort();
    //     }
    // }

    /// The actual guts of `new`, using a file loader so we can test it.
    fn inner_new<P>(path: P, file_loader:&FileLoader) -> DnLibResult<Self>
        where P: AsRef<Path>
    {
        // First find all the paths of interest.
        let pta = find_files(path)?;

        // Now group them into our structure.
        // Load and analyze each solution and place them into folders.
        let mut files = AnalyzedFiles::default();
        for sln_path in &pta.sln_files {
            files.add_solution(sln_path, file_loader);
        }

        // For each project, grab all the 'other' files in the same directory.
        // (This is very hacky. Assumes they are all in the project directory! Can fix by replacing
        // the '==' with a closure).
        // Then analyze each project.
        let analyzed_projects = pta.csproj_files.iter()
            .map(|proj_path| {
                let other_paths = pta.other_files.iter()
                    .filter(|&other_path| other_path.parent().unwrap() == proj_path.parent().unwrap())
                    .cloned()
                    .collect::<Vec<_>>();

                Project::new(proj_path, other_paths, file_loader)
            })
            .collect::<Vec<_>>();

        for proj in analyzed_projects {
            files.add_project(proj);
        }

        //files.sort();
        Ok(files)
    }

    fn add_solution(&mut self, path: &PathBuf, file_loader: &FileLoader) {
        let sln_dir = path.parent().unwrap();
        for item in &mut self.scanned_directories {
            if item.directory == sln_dir {
                item.sln_files.push(Solution::new(path, file_loader));
                return;
            }
        }
    }

    fn add_project(&mut self, project: Project) {
        match self.find_owning_solution(&project.file_info.path) {
            Some((SolutionMatchType::Linked, ref mut sln)) => sln.linked_projects.push(Project::default()),
            Some((SolutionMatchType::Orphaned, ref mut sln)) => sln.orphaned_projects.push(Project::default()),
            None => eprintln!("Could not associate project {:?} with a solution, ignoring.", &project.file_info.path),
        }
    }

    /// Scan all known solutions trying to find one that refers to the specified
    /// project path. If such a match is found, a Linked match is returned.
    /// If such a match cannot be found, attempt to locate the project with
    /// its closest matching solution by directory, and return an Orphaned match.
    /// If that fails, return None.
    pub fn find_owning_solution<P>(&self, project_path: P) -> Option<(SolutionMatchType, &mut Solution)>
        where P: AsRef<Path>
    {
        let project_path = project_path.as_ref();
        None
    }
}


#[derive(Debug, Default)]
/// Represents a directory that contains 1 or more solution files.
pub struct SolutionDirectory {
    /// The directory path, e.g. `C:\temp\my_solution`.
    pub directory: PathBuf,

    /// The sln files in this directory.
    pub sln_files: Vec<Solution>
}

impl SolutionDirectory {
    // fn sort(&mut self) {
    //     self.sln_files.sort();
    // }
}


#[derive(Debug, Default)]
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
    pub fn new<P>(path: P, file_loader: &FileLoader) -> Self
        where P: AsRef<Path>
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
        // self.linked_projects.sort();
        // self.orphaned_projects.sort();
    }
}


#[cfg(test)]
mod tests {
    use super::*;
}
