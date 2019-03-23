use crate::package_class::PackageClass;

#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Package {
    pub name: String,
    pub version: String,
    pub development: bool,
    pub class: PackageClass
}


impl Package {
    pub fn new(name: &str, version: &str, development: bool, class: PackageClass) -> Self {
        Package {
            name: name.to_owned(),
            version: version.to_owned(),
            development,
            class
        }
    }

    pub fn is_preview(&self) -> bool {
        self.version.contains("-")
    }
}
