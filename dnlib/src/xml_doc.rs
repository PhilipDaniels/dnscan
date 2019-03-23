use crate::as_str::AsStr;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum XmlDoc {
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

impl Default for XmlDoc {
    fn default() -> Self {
        XmlDoc::Unknown
    }
}

impl AsStr for XmlDoc {
    fn as_str(&self) -> &'static str {
        match self {
            XmlDoc::Unknown => "Unknown",
            XmlDoc::None => "None",
            XmlDoc::Debug => "Debug",
            XmlDoc::Release => "Release",
            XmlDoc::Both => "Both",
        }
    }
}
