#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum OutputType {
    Unknown,

    /// The output is a library (DLL).
    Library,

    /// The output is a Windows EXE (e.g. a WinForms app).
    WinExe,

    /// The output is an EXE (e.g. a Console app).
    Exe,
}

impl Default for OutputType {
    fn default() -> Self {
        OutputType::Unknown
    }
}

impl AsRef<str> for OutputType {
    fn as_ref(&self) -> &str {
        match self {
            OutputType::Unknown => "Unknown",
            OutputType::Library => "Library",
            OutputType::WinExe => "WinExe",
            OutputType::Exe => "Exe",
        }
    }
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
