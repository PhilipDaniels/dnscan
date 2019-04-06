#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum ProjectOwnership {
    Unknown,
    Linked,
    Orphaned,
}

impl Default for ProjectOwnership {
    fn default() -> Self {
        ProjectOwnership::Unknown
    }
}

impl AsRef<str> for ProjectOwnership {
    fn as_ref(&self) -> &str {
        match self {
            ProjectOwnership::Unknown => "Unknown",
            ProjectOwnership::Linked => "Linked",
            ProjectOwnership::Orphaned => "Orphaned",
        }
    }
}
