use crate::as_str::AsStr;
use crate::dn_error::DnLibError;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
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

impl AsStr for InterestingFile {
    fn as_str(&self) -> &'static str {
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
    type Err = DnLibError;

    fn from_str(s: &str) -> Result<InterestingFile, Self::Err> {
        let s = s.to_lowercase();
        match s.as_str() {
            "web.config" => Ok(InterestingFile::WebConfig),
            "app.config" => Ok(InterestingFile::AppConfig),
            "appsettings.json" => Ok(InterestingFile::AppSettingsJson),
            "package.json" => Ok(InterestingFile::PackageJson),
            "packages.config" => Ok(InterestingFile::PackagesConfig),
            "project.json" => Ok(InterestingFile::ProjectJson),
            _ => Err(DnLibError::InvalidInterestingFile(s)),
        }
    }
}
