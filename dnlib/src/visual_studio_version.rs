use crate::as_str::AsStr;

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum VisualStudioVersion {
    Unknown,
    VS2015,
    VS2017,
    VS2019,
}

impl Default for VisualStudioVersion {
    fn default() -> Self {
        VisualStudioVersion::Unknown
    }
}

impl AsStr for VisualStudioVersion {
    fn as_str(&self) -> &'static str {
        match self {
            VisualStudioVersion::Unknown => "Unknown",
            VisualStudioVersion::VS2015 => "VS2015",
            VisualStudioVersion::VS2017 => "VS2017",
            VisualStudioVersion::VS2019 => "VS2019",
        }
    }
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
