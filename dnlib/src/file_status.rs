use crate::as_str::AsStr;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
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

impl AsStr for FileStatus {
    fn as_str(&self) -> &'static str {
        match self {
            FileStatus::Unknown => "Unknown",
            FileStatus::NotPresent => "NotPresent",
            FileStatus::InProjectFileOnly => "InProjectFileOnly",
            FileStatus::OnDiskOnly => "OnDiskOnly",
            FileStatus::InProjectFileAndOnDisk => "InProjectFileAndOnDisk",
        }
    }
}
