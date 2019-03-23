use crate::as_str::AsStr;

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum PackageClass {
    Unknown,
    Ours,
    Microsoft,
    ThirdParty,
}

impl Default for PackageClass {
    fn default() -> Self {
        PackageClass::Unknown
    }
}

impl AsStr for PackageClass {
    fn as_str(&self) -> &'static str {
        match self {
            PackageClass::Unknown => "Unknown",
            PackageClass::Ours => "Ours",
            PackageClass::Microsoft => "Microsoft",
            PackageClass::ThirdParty => "ThirdParty",
        }
    }
}
