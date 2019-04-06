#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum FileStatus {
    Unknown,
    NotPresent,
    InProjectFileOnly,
    OnDiskOnly,
    InProjectFileAndOnDisk
}

impl Default for FileStatus {
    fn default() -> Self {
        FileStatus::Unknown
    }
}

impl AsRef<str> for FileStatus {
    fn as_ref(&self) -> &str {
        match self {
            FileStatus::Unknown => "Unknown",
            FileStatus::NotPresent => "NotPresent",
            FileStatus::InProjectFileOnly => "InProjectFileOnly",
            FileStatus::OnDiskOnly => "OnDiskOnly",
            FileStatus::InProjectFileAndOnDisk => "InProjectFileAndOnDisk",
        }
    }
}
