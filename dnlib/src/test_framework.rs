#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
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

impl AsRef<str> for TestFramework {
    fn as_ref(&self) -> &str {
        match self {
            TestFramework::None => "None",
            TestFramework::MSTest => "MSTest",
            TestFramework::XUnit => "XUnit",
            TestFramework::NUnit => "NUnit",
        }
    }
}
