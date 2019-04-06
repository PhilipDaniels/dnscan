use lazy_static::lazy_static;
use regex::Regex;

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
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

impl AsRef<str> for XmlDoc {
    fn as_ref(&self) -> &str {
        match self {
            XmlDoc::Unknown => "Unknown",
            XmlDoc::None => "None",
            XmlDoc::Debug => "Debug",
            XmlDoc::Release => "Release",
            XmlDoc::Both => "Both",
        }
    }
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