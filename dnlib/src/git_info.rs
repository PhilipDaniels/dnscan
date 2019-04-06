use std::path::Path;
use std::ffi::OsStr;
use crate::errors::DnLibResult;

#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord)]
/// Represents information about the Git repository.
pub struct GitInfo {
    pub git_branch: String,
    pub git_sha: String,
    pub last_modify_date: String,
    pub remote_url: String
}

impl GitInfo {
    /// Gets the git information about a particular path. Searches for a git
    /// repository in that directory, or its parents, until it finds one or
    /// it reaches the `ceiling_dir`.
    pub fn new<D, C>(directory: D, ceiling_dir: C) -> DnLibResult<Self>
    where D: AsRef<Path>,
          C: AsRef<OsStr>
    {
        Ok(Default::default())
    }
}
