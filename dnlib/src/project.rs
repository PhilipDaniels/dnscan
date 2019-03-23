use std::path::Path;
use crate::file_info::FileInfo;
use crate::visual_studio_version::VisualStudioVersion;
use crate::file_loader::FileLoader;
use crate::git_info::GitInfo;

/// The results of analyzing a project file.
#[derive(Debug, Default)]
pub struct Project2 {
    pub file_info: FileInfo,
    //pub linked_projects: Vec<Arc<Project>>,
    //pub orphaned_projects: Vec<Project>
}

impl Project2 {
    pub fn new<P, L>(path: P, file_loader: &L) -> Self
        where P: AsRef<Path>,
              L: FileLoader
    {
        let fi = FileInfo::new(path, file_loader);
        let ver = VisualStudioVersion::extract(&fi.contents).unwrap_or_default();

        Solution {
            file_info: fi,
            version: ver,
            ..Default::default()
        }
    }
}