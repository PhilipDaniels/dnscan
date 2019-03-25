use crate::as_str::AsStr;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ProjectVersion {
    Unknown,

    /// The type of project that begins with `<Project Sdk="Microsoft.NET.Sdk">`.
    MicrosoftNetSdk,

    /// The type of project that begins with `<Project Sdk="Microsoft.NET.Sdk.Web">`.
    MicrosoftNetSdkWeb,

    /// The type of project that begins with `<?xml version="1.0" encoding="utf-8"?>`
    /// and includes the next line `<Project ToolsVersion="14.0"`
    OldStyle,
}

impl Default for ProjectVersion {
    fn default() -> Self {
        ProjectVersion::Unknown
    }
}

impl AsStr for ProjectVersion {
    fn as_str(&self) -> &'static str {
        match self {
            ProjectVersion::Unknown => "Unknown",
            ProjectVersion::MicrosoftNetSdk => "MicrosoftNetSdk",
            ProjectVersion::MicrosoftNetSdkWeb => "MicrosoftNetSdkWeb",
            ProjectVersion::OldStyle => "OldStyle",
        }
    }
}

const SDK_WEB_PROLOG: &str = "<Project Sdk=\"Microsoft.NET.Sdk.Web\">";
const SDK_PROLOG: &str = "<Project Sdk=\"Microsoft.NET.Sdk\">";
const OLD_PROLOG: &str = "<Project ToolsVersion=";

impl ProjectVersion {
    pub fn extract(project_file_contents: &str) -> Option<ProjectVersion> {
        if project_file_contents.contains(SDK_WEB_PROLOG) {
            Some(ProjectVersion::MicrosoftNetSdkWeb)
        } else if project_file_contents.contains(SDK_PROLOG) {
            Some(ProjectVersion::MicrosoftNetSdk)
        } else if project_file_contents.contains(OLD_PROLOG) {
            Some(ProjectVersion::OldStyle)
        } else {
            None
        }
    }
}