use std::path::{Path, PathBuf};
use std::sync::Arc;
use crate::Project;
use strum_macros::IntoStaticStr;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum VisualStudioVersion {
    Unknown,
    VS2015,
    VS2017,
    VS2019,
}

impl VisualStudioVersion {
    pub fn as_str(self) -> &'static str {
        match self {
            VisualStudioVersion::Unknown => "Unknown",
            VisualStudioVersion::VS2015 => "VS2015",
            VisualStudioVersion::VS2017 => "VS2017",
            VisualStudioVersion::VS2019 => "VS2019",
        }
    }
}

impl Default for VisualStudioVersion {
    fn default() -> Self {
        VisualStudioVersion::Unknown
    }
}

/// The results of analyzing a solution file.
#[derive(Debug, Default)]
pub struct Solution {
    pub version: VisualStudioVersion,
    pub file: PathBuf,
    pub contents: String,
    pub is_valid_utf8: bool,
    //pub last_modify_date: String
    //pub git_branch: String,
    //pub git_sha: String,
    pub linked_projects: Vec<Arc<Project>>,
    pub orphaned_projects: Vec<Project>
}

impl Solution {
    pub fn new(path: &Path) -> Self {
        let mut sln = Solution::default();
        sln.file = path.to_owned();

        match std::fs::read_to_string(path) {
            Ok(s) => {
                sln.is_valid_utf8 = true;
                sln.contents = s;
            },
            Err(_) => sln.is_valid_utf8 = false,
        }

        sln.version = if sln.contents.contains("# Visual Studio 14") {
             VisualStudioVersion::VS2015
        } else if sln.contents.contains("# Visual Studio 15") {
            VisualStudioVersion::VS2017
        } else if sln.contents.contains("# Visual Studio Version 16") {
            VisualStudioVersion::VS2019
        } else {
            VisualStudioVersion::Unknown
        };

        sln
    }
}
