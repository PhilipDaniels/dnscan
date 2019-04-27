use std::fmt;

use lazy_static::lazy_static;
use regex::Regex;
use strum_macros::{AsRefStr};
use smart_default::SmartDefault;

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, AsRefStr, SmartDefault)]
pub enum FileStatus {
    #[default]
    Unknown,
    NotPresent,
    InProjectFileOnly,
    OnDiskOnly,
    InProjectFileAndOnDisk
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum InterestingFile {
    /// The web.config file.
    WebConfig,

    /// The app.config file.
    AppConfig,

    /// The appsettings.json file.
    AppSettingsJson,

    /// The package.json file (required by npm).
    PackageJson,

    /// The packages.config file (obsolete, should be removed)
    PackagesConfig,

    /// The project.json (obsolete, should be removed)
    ProjectJson
}

impl AsRef<str> for InterestingFile {
    fn as_ref(&self) -> &str {
        match self {
            InterestingFile::WebConfig => "web.config",
            InterestingFile::AppConfig => "app.config",
            InterestingFile::AppSettingsJson => "appsettings.json",
            InterestingFile::PackageJson => "package.json",
            InterestingFile::PackagesConfig => "packages.config",
            InterestingFile::ProjectJson => "project.json"
        }
    }
}

impl std::str::FromStr for InterestingFile {
    type Err = ();

    fn from_str(s: &str) -> Result<InterestingFile, Self::Err> {
        let s = s.to_lowercase();
        match s.as_str() {
            "web.config" => Ok(InterestingFile::WebConfig),
            "app.config" => Ok(InterestingFile::AppConfig),
            "appsettings.json" => Ok(InterestingFile::AppSettingsJson),
            "package.json" => Ok(InterestingFile::PackageJson),
            "packages.config" => Ok(InterestingFile::PackagesConfig),
            "project.json" => Ok(InterestingFile::ProjectJson),
            _ => Err(())
        }
    }
}

impl fmt::Display for InterestingFile {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.as_ref())
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, AsRefStr, SmartDefault)]
pub enum OutputType {
    #[default]
    Unknown,

    /// The output is a library (DLL).
    Library,

    /// The output is a Windows EXE (e.g. a WinForms app).
    WinExe,

    /// The output is an EXE (e.g. a Console app).
    Exe,
}

impl OutputType {
    pub fn extract(project_file_contents: &str) -> OutputType {
        if project_file_contents.contains("<OutputType>Library</OutputType>") {
            OutputType::Library
        } else if project_file_contents.contains("<OutputType>Exe</OutputType>") {
            OutputType::Exe
        } else if project_file_contents.contains("<OutputType>WinExe</OutputType>") {
            OutputType::WinExe
        } else {
            // This appears to be the default, certainly for SDK-style projects anyway.
            OutputType::Library
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, AsRefStr, SmartDefault)]
pub enum ProjectOwnership {
    #[default]
    Unknown,
    Linked,
    Orphaned,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, AsRefStr, SmartDefault)]
pub enum ProjectVersion {
    #[default]
    Unknown,

    /// The type of project that begins with `<Project Sdk="Microsoft.NET.Sdk">`.
    MicrosoftNetSdk,

    /// The type of project that begins with `<Project Sdk="Microsoft.NET.Sdk.Web">`.
    MicrosoftNetSdkWeb,

    /// The type of project that begins with `<?xml version="1.0" encoding="utf-8"?>`
    /// and includes the next line `<Project ToolsVersion="14.0"`
    OldStyle,
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

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, AsRefStr, SmartDefault)]
pub enum TestFramework {
    #[default]
    None,
    MSTest,
    XUnit,
    NUnit,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, AsRefStr, SmartDefault)]
pub enum VisualStudioVersion {
    #[default]
    Unknown,
    VS2015,
    VS2017,
    VS2019,
}

impl VisualStudioVersion {
    pub fn extract(solution_file_contents: &str) -> Option<VisualStudioVersion> {
        if solution_file_contents.contains("# Visual Studio 14") {
            Some(VisualStudioVersion::VS2015)
        } else if solution_file_contents.contains("# Visual Studio 15") {
            Some(VisualStudioVersion::VS2017)
        } else if solution_file_contents.contains("# Visual Studio Version 16") {
            Some(VisualStudioVersion::VS2019)
        } else {
            None
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, AsRefStr, SmartDefault)]
pub enum XmlDoc {
    #[default]
    Unknown,

    /// No Debug or Release mode XML documentation is being generated.
    None,

    /// XML documentation is being generated for Debug mode only.
    Debug,

    /// XML documentation is being generated for Release mode only.
    Release,

    /// XML documentation is being generated for both Debug and Release mode.
    Both
}

impl XmlDoc {
    pub fn extract(project_file_contents: &str) -> XmlDoc {
        lazy_static! {
            static ref DEBUG_RE: Regex = Regex::new(r##"<DocumentationFile>bin\\[Dd]ebug\\.*?\.xml</DocumentationFile>"##).unwrap();
            static ref RELEASE_RE: Regex = Regex::new(r##"<DocumentationFile>bin\\[Rr]elease\\.*?\.xml</DocumentationFile>"##).unwrap();
        }

        match (DEBUG_RE.is_match(project_file_contents), RELEASE_RE.is_match(project_file_contents)) {
            (true, true) => XmlDoc::Both,
            (true, false) => XmlDoc::Debug,
            (false, true) => XmlDoc::Release,
            (false, false) => XmlDoc::None,
        }
    }
}
