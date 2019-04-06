use std::fmt;

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
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

impl AsRef<str> for ProjectVersion {
    fn as_ref(&self) -> &str {
        match self {
            ProjectVersion::Unknown => "Unknown",
            ProjectVersion::MicrosoftNetSdk => "MicrosoftNetSdk",
            ProjectVersion::MicrosoftNetSdkWeb => "MicrosoftNetSdkWeb",
            ProjectVersion::OldStyle => "OldStyle",
        }
    }
}

pub(crate) const SDK_WEB_PROLOG: &str = r#"<Project Sdk="Microsoft.NET.Sdk.Web">"#;
pub(crate) const SDK_PROLOG: &str = r#"<Project Sdk="Microsoft.NET.Sdk">"#;
pub(crate) const OLD_PROLOG: &str = "<Project ToolsVersion=";

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

impl fmt::Display for ProjectVersion {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}",
            match self {
                ProjectVersion::Unknown => "Unknown",
                ProjectVersion::MicrosoftNetSdk => "MicrosoftNetSdk",
                ProjectVersion::MicrosoftNetSdkWeb => "MicrosoftNetSdkWeb",
                ProjectVersion::OldStyle => "OldStyle",
            })
    }
}
