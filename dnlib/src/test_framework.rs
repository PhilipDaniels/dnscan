use crate::as_str::AsStr;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum TestFramework {
    None,
    MSTest,
    XUnit,
    NUnit,
}

impl Default for TestFramework {
    fn default() -> Self {
        TestFramework::None
    }
}

impl AsStr for TestFramework {
    fn as_str(&self) -> &'static str {
        match self {
            TestFramework::None => "None",
            TestFramework::MSTest => "MSTest",
            TestFramework::XUnit => "XUnit",
            TestFramework::NUnit => "NUnit",
        }
    }
}
