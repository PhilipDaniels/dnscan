use std::path::Path;
use std::ffi::OsStr;
use crate::errors::DnLibResult;
use git2::{Repository, RepositoryOpenFlags, Remote};

#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord)]
/// Represents information about the Git repository.
pub struct GitInfo {
    pub branch: String,
    pub sha: String,
    pub summary: String,
    pub commit_time: String,
    pub author: String,
    pub author_email: String,
    pub remote_name: String,
    pub remote_url: String,
}

impl GitInfo {
    /// Gets the git information about a particular path. Searches for a git
    /// repository in that directory, or its parents, until it finds one or
    /// it reaches the `ceiling_dir`.
    pub fn new<D, C>(directory: D, ceiling_dir: C) -> DnLibResult<Self>
    where D: AsRef<Path>,
          C: AsRef<OsStr>
    {

        let repo = Repository::open_ext(directory,
            RepositoryOpenFlags::empty(),
            vec![ceiling_dir])?;

        let head = repo.head()?;
        let head_commit = head.peel_to_commit()?;

        let mut gi = Self::default();
        gi.branch = Self::get_current_branch(&repo).unwrap_or_default();
        gi.sha = head_commit.id().to_string();
        gi.summary = head_commit.summary().unwrap_or_default().to_owned();
        gi.commit_time = Self::git_time_to_string(head_commit.time().seconds());
        gi.author = head_commit.author().name().unwrap_or_default().to_owned();
        gi.author_email = head_commit.author().email().unwrap_or_default().to_owned();

        if let Some(remote) = Self::get_remote(&repo) {
            gi.remote_name = remote.name().unwrap_or_default().to_owned();
            gi.remote_url = remote.url().unwrap_or_default().to_owned();
        }

        Ok(gi)
    }

    fn git_time_to_string(seconds_from_epoch: i64) -> String {
        use chrono::prelude::DateTime;
        use chrono::{Utc};
        use std::time::{UNIX_EPOCH, Duration};

        // Creates a new SystemTime from the specified number of whole seconds
        let d = UNIX_EPOCH + Duration::from_secs(seconds_from_epoch as u64);
        // Create DateTime from SystemTime
        let datetime = DateTime::<Utc>::from(d);
        // Formats the combined date and time with the specified format string.
        datetime.format("%Y-%m-%d %H:%M:%S").to_string()
    }

    fn get_current_branch(repo: &Repository) -> DnLibResult<String> {
        for branch in repo.branches(None)? {
            let (branch, _) = branch?;
            if branch.is_head() {
                return Ok(branch.name()?.unwrap_or("").to_owned());
            }
        }

        Ok("".to_owned())
    }

    fn get_remote(repo: &Repository) -> Option<Remote> {
        if let Ok(remote_names) = repo.remotes() {
            for remote_name in &remote_names {
                if let Some(remote_name) = remote_name {
                    if let Ok(remote) = repo.find_remote(remote_name) {
                        return Some(remote);
                    }
                }
            }
        }

        None
    }
}
