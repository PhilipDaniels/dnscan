use std::path::{Path, PathBuf};
use crate::file_info::FileInfo;
use crate::visual_studio_version::VisualStudioVersion;
use crate::file_loader::FileLoader;

/// The results of analyzing a project file.
#[derive(Debug, Default)]
pub struct Project2 {
    pub file_info: FileInfo,
    //pub linked_projects: Vec<Arc<Project>>,
    //pub orphaned_projects: Vec<Project>
}

impl Project2 {
    pub fn new<P>(path: P, other_files: Vec<&PathBuf>, file_loader: &FileLoader) -> Self
        where P: AsRef<Path>
    {
        let fi = FileInfo::new(path, file_loader);
        let ver = VisualStudioVersion::extract(&fi.contents).unwrap_or_default();

        Project2 {
            file_info: fi,
            ..Default::default()
        }
    }
}