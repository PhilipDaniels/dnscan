use std::path::{Path, PathBuf};
use std::{io, fs};
use std::collections::HashMap;

use regex::Regex;
use serde::{Serialize, Deserialize};
use serde_json;
use serde_regex;
use log::warn;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageGroup {
    pub name: String,
    #[serde(with = "serde_regex")]
    pub regex: Regex,
}

impl PackageGroup {
    fn new<N, R>(name: N, regex: R) -> Self
    where N: Into<String>,
          R: AsRef<str>
    {
        PackageGroup {
            name: name.into(),
            regex: Regex::new(regex.as_ref()).unwrap(),
        }
    }
}

/// Represents the contents of our configuration file.
#[derive(Clone, Serialize, Deserialize)]
pub struct Configuration {
    pub package_groups: Vec<PackageGroup>,
    pub abbreviations: HashMap<String, Vec<String>>,
    pub input_directory: PathBuf,
    pub output_directory: PathBuf,
}

impl Default for Configuration {
    fn default() -> Self {
        let mut abbrevs = HashMap::<String, Vec<String>>::new();
        abbrevs.insert("MS".to_string(), vec!["Microsoft".to_string()]);

        Configuration {
            package_groups: vec![
                // The order matters here. Attempts are made to match package names in the order that these
                // elements appear in (which matters if patterns are not mutually exclusive).
                // A catch all assigns 'Third Party' to anything not yet matched.
                PackageGroup::new("Third Party", r#"^System\.IO\.Abstractions.*|^Owin\.Metrics|^EntityFramework6\.Npgsql"#),
                PackageGroup::new("Microsoft", r#"^CommonServiceLocator|^NETStandard\..*|^EntityFramework*|^Microsoft\..*|^MSTest.*|^Owin.*|^System\..*|^AspNet\..*|^WindowsAzure\..*|^EnterpriseLibrary.*"#),
                PackageGroup::new("Third Party", r#".*"#),
            ],
            abbreviations: abbrevs,
            output_directory: "dnscan-output".into(),
            input_directory: "".into()
        }
    }
}

impl Configuration {
    pub fn new<P>(directory_to_scan: P) -> Self
    where P: Into<PathBuf>
    {
        const CONFIG_FILE: &str = ".dnscan.json";

        // Look for a config file in the path to scan.
        let mut dir_to_scan = directory_to_scan.into();
        dir_to_scan.push(CONFIG_FILE);
        if let Some(cfg) = Self::load_from_file(&dir_to_scan) {
            return cfg;
        }

        // We really need a home-dir, that is where we will store the NuGet package metadata.
        // I feel it's reasonable to bomb out if there isn't one.
        let mut home_dir = dirs::home_dir().expect("Cannot determine home dir; required for storage of NuGet metadata.");

        // If we have one, look for our standard config directory.
        home_dir.push(".dnscan");
        home_dir.push(CONFIG_FILE);
        if let Some(cfg) = Self::load_from_file(&home_dir) {
            return cfg;
        }

        // If not found, use default settings.
        Configuration::default()
    }

    pub fn to_string(&self) -> String {
        serde_json::to_string_pretty(self).unwrap()
    }

    /// Writes the default settings to stdout.
    pub fn dump_defaults() {
        use std::io::Write;

        let serialized = Configuration::default().to_string();
        println!("{}", serialized);
        io::stdout().flush().unwrap();
    }

    fn load_from_file(path: &Path) -> Option<Configuration> {
        match fs::File::open(path) {
            Ok(f) => match serde_json::from_reader(f) {
                Ok(r) => {
                    println!("Loaded configuration from {}", path.display());
                    Some(r)
                },
                Err(e) => { warn!("Could not parse JSON, falling back to default configuration. {:?}", e); None },
            },
            Err(ref e) if e.kind() == io::ErrorKind::NotFound => None,
            Err(e) => panic!("Error opening config file {:?}", e)
        }
    }
}
