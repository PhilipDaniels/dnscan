use crate::as_str::AsStr;

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

impl AsStr for ProjectOwnership {
    fn as_str(&self) -> &'static str {
        match self {
            ProjectOwnership::Unknown => "Unknown",
            ProjectOwnership::Linked => "Linked",
            ProjectOwnership::Orphaned => "Orphaned",
        }
    }
}
