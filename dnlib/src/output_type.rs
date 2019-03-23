use crate::as_str::AsStr;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
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

impl AsStr for OutputType {
    fn as_str(&self) -> &'static str {
        match self {
            OutputType::Unknown => "Unknown",
            OutputType::Library => "Library",
            OutputType::WinExe => "WinExe",
            OutputType::Exe => "Exe",
        }
    }
}
