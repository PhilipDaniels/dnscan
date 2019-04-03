use std::path::Path;
use std::{io, fs, env};

use regex::Regex;
use serde::{Serialize, Deserialize};
use serde_json;
use serde_regex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageGroup {
    pub name: String,
    #[serde(with = "serde_regex")]
    pub regex: Regex,
}

impl PackageGroup {
    fn new<S>(name: S, regex: S) -> Self
    where S: AsRef<str>
    {
        PackageGroup {
            name: name.as_ref().to_owned(),
            regex: Regex::new(regex.as_ref()).unwrap(),
        }
    }
}

/// Represents the contents of our configuration file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Configuration {
    pub package_groups: Vec<PackageGroup>
}

impl Default for Configuration {
    fn default() -> Self {
        Configuration {
            package_groups: vec![
                // The order matters here. Attempts are made to match package names in the order that these
                // elements appear in (which matters if patterns are not mutually exclusive).
                // A catch all assigns 'Third Party' to anything not yet matched.
                PackageGroup::new("Third Party", r#"^System\.IO\.Abstractions.*|^Owin\.Metrics|^EntityFramework6\.Npgsql"#),
                PackageGroup::new("VRM", r#"^VRM\..*|^WorkflowService\..*|^WorkflowRunner\.."#),
                PackageGroup::new("ValHub", r#"^AuthZ.*|^Landmark\..*|^DataMaintenance.*|^ValuationHub\..*|^CaseService\..*|^CaseActivities\..*|^NotificationService\..*|^Unity\.WF|^QIFCore"#),
                PackageGroup::new("Microsoft", r#"^CommonServiceLocator|^NETStandard\..*|^EntityFramework*|^Microsoft\..*|^MSTest.*|^Owin.*|^System\..*|^AspNet\..*|^WindowsAzure\..*|^EnterpriseLibrary.*"#),
                PackageGroup::new("Third Party", r#".*"#),
            ]
        }
    }
}

impl Configuration {
    pub fn new<P>(directory_to_scan: P) -> Self
    where P: AsRef<Path>
    {
        const CONFIG_FILE: &str = ".dnscan.json";

        // Look for a config file in the path to scan.
        let mut dir_to_scan = directory_to_scan.as_ref().to_owned();
        dir_to_scan.push(CONFIG_FILE);
        if let Some(cfg) = Self::load_from_file(&dir_to_scan) {
            return cfg;
        }

        // If not found, look for a file in the same directory as the exe.
        if let Ok(exe_path) = env::current_exe() {
            let mut exe_dir = exe_path.parent().unwrap().to_owned();
            exe_dir.push(CONFIG_FILE);
            if let Some(cfg) = Self::load_from_file(&exe_dir) {
                return cfg;
            }
        }

        // If not found, look for a file in the home dir.
        if let Some(mut home_dir) = dirs::home_dir() {
            home_dir.push(CONFIG_FILE);
            if let Some(cfg) = Self::load_from_file(&home_dir) {
                return cfg;
            }
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
                Ok(r) => Some(r),
                Err(e) => { eprintln!("Could not parse JSON {:?}", e); None },
            },
            Err(ref e) if e.kind() == io::ErrorKind::NotFound => None,
            Err(e) => panic!("Error opening config file {:?}", e)
        }
    }
}
