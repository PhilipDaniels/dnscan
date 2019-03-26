use std::path::Path;

#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord)]
/// Represents information about the Git repository a file is in.
pub struct GitInfo {
    //pub git_branch: String,
    //pub git_sha: String,
    //pub last_modify_date: String
    //pub remote_url: String
}

impl GitInfo {
    pub fn new<P>(path: P) -> Self
        where P: AsRef<Path>
    {
        Default::default()
    }
}
