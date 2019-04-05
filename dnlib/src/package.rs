#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Package {
    pub name: String,
    pub version: String,
    pub development: bool,
    pub class: String
}

impl Package {
    pub fn new<N, V, C>(name: N, version: V, development: bool, class: C) -> Self
    where N: Into<String>,
          V: Into<String>,
          C: Into<String>
    {
        Package {
            name: name.into(),
            version: version.into(),
            development,
            class: class.into()
        }
    }

    pub fn is_preview(&self) -> bool {
        self.version.contains('-')
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn demonstrate_can_call_with_differing_type_parameters() {
        let class = "class".to_owned();
        let pkg = Package::new("name", "ver", true, class);
    }
}
