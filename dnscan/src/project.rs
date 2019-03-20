use crate::find_files::{PathsToAnalyze, InterestingFile};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use regex::{Regex, RegexBuilder};
use lazy_static::lazy_static;
use dnlib::file_loader::{FileLoader};

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ProjectVersion {
    Unknown,

    /// The type of project that begins with `<Project Sdk="Microsoft.NET.Sdk">`.
    MicrosoftNetSdk,

    /// The type of project that begins with `<?xml version="1.0" encoding="utf-8"?>`
    /// and includes the next line `<Project ToolsVersion="14.0"`
    OldStyle,
}

impl Default for ProjectVersion {
    fn default() -> Self {
        ProjectVersion::Unknown
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum OutputType {
    Unknown,

    /// The output is a library (DLL).
    Library,

    /// The output is a Windows EXE (e.g. a WinForms app).
    WinExe,

    /// The output is an EXE.
    Exe,
}

impl Default for OutputType {
    fn default() -> Self {
        OutputType::Unknown
    }
}

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

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum XmlDoc {
    Unknown,
    None,
    Debug,
    Release,
    Both
}

impl Default for XmlDoc {
    fn default() -> Self {
        XmlDoc::Unknown
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum FileStatus {
    Unknown,
    NotPresent,
    InProjectFileOnly,
    OnDiskOnly,
    InProjectFileAndOnDisk
}

impl Default for FileStatus {
    fn default() -> Self {
        FileStatus::Unknown
    }
}

#[derive(Debug, Default)]
pub struct Project {
    pub file: PathBuf,
    pub is_valid_utf8: bool,
    pub contents: String,
    pub version: ProjectVersion,
    //pub last_modify_date: String
    //pub git_branch: String,
    //pub git_sha: String,

    pub output_type: OutputType,
    pub xml_doc: XmlDoc,
    pub tt_file: bool,
    pub target_frameworks: Vec<String>,
    pub embedded_debugging: bool,
    pub linked_solution_info: bool,
    pub auto_generate_binding_redirects: bool,
    pub test_framework: TestFramework,

    pub web_config: FileStatus,
    pub app_config: FileStatus,
    pub app_settings_json: FileStatus,
    pub package_json: FileStatus,
    pub packages_config: FileStatus,
    pub project_json: FileStatus,

    pub referenced_assemblies: Vec<String>,
    pub packages: Vec<Package>,
    pub referenced_projects: Vec<Arc<Project>>,
}

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

#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Package {
    pub name: String,
    pub version: String,
    pub development: bool,
    pub class: PackageClass
}

impl Package {
    fn new(name: &str, version: &str, development: bool) -> Self {
        Package {
            name: name.to_owned(),
            version: version.to_owned(),
            development,
            class: PackageClass::Unknown
        }
    }
}

const SDK_PROLOG: &str = "<Project Sdk=\"Microsoft.NET.Sdk\">";
const OLD_PROLOG: &str = "<Project ToolsVersion=";



impl Project {
    pub fn new(project_file_path: &Path, pta: &PathsToAnalyze, file_loader: &FileLoader) -> Self {
        let mut proj = Project::default();
        proj.file = project_file_path.to_owned();

        match file_loader.read_to_string(project_file_path) {
            Ok(s) => {
                proj.is_valid_utf8 = true;
                proj.analyze(pta, s, file_loader);
            },
            Err(_) => {
                proj.is_valid_utf8 = false;
            }
        }

        proj
    }

    /// Factor the guts of the analysis out into a separate function so that it
    /// can be easily unit tested.
    fn analyze(&mut self, pta: &PathsToAnalyze, contents: String, file_loader: &FileLoader) {
        self.contents = contents;

        self.version = if self.contents.contains(SDK_PROLOG) {
             ProjectVersion::MicrosoftNetSdk
        } else if self.contents.contains(OLD_PROLOG) {
            ProjectVersion::OldStyle
        } else {
            ProjectVersion::Unknown
        };

        self.output_type = self.get_output_type();
        self.xml_doc = self.has_xml_doc();
        self.tt_file = self.has_tt_file();
        self.linked_solution_info = self.has_linked_solution_info();
        self.referenced_assemblies = self.get_referenced_assemblies();
        self.auto_generate_binding_redirects = self.has_auto_generate_binding_redirects();
        self.web_config = self.has_file_of_interest(pta, InterestingFile::WebConfig);
        self.app_config = self.has_file_of_interest(pta, InterestingFile::AppConfig);
        self.app_settings_json = self.has_file_of_interest(pta, InterestingFile::AppSettingsJson);
        self.package_json = self.has_file_of_interest(pta, InterestingFile::PackageJson);
        self.packages_config = self.has_file_of_interest(pta, InterestingFile::PackagesConfig);
        self.project_json = self.has_file_of_interest(pta, InterestingFile::ProjectJson);
        self.embedded_debugging = self.has_embedded_debugging();
        self.target_frameworks = self.get_target_frameworks();
        self.packages = self.get_packages(pta, file_loader);
        self.test_framework = self.get_test_framework();
        // pub uses_specflow
        // pub referenced_projects: Vec<Arc<Project>>,
    }

    fn get_output_type(&self) -> OutputType {
        if self.contents.contains("<OutputType>Library</OutputType>") {
            OutputType::Library
        } else if self.contents.contains("<OutputType>Exe</OutputType>") {
            OutputType::Exe
        } else if self.contents.contains("<OutputType>WinExe</OutputType>") {
            OutputType::WinExe
        } else {
            // This appears to be the default, certainly for SDK-style projects anyway.
            OutputType::Library
        }
    }

    fn get_test_framework(&self) -> TestFramework {
        // Basically, we need to get the package references and then check for
        // libraries of a specific name:
        //      MSTest.TestAdapter or MSTest.TestFramework = MSTest
        //      xunit.* = XUnit
        //      nunit.* = NUnit
        // All should be matched case-insensitively.

        TestFramework::None
    }

    fn get_packages(&self, pta: &PathsToAnalyze, file_loader: &FileLoader) -> Vec<Package> {
        lazy_static! {
            static ref SDK_RE: Regex = Regex::new(r##"(?s)<PackageReference\s*?Include="(?P<name>.*?)"\s*?Version="(?P<version>.*?)"(?P<inner>.*?)(/>|</PackageReference>)"##).unwrap();
            static ref PKG_CONFIG_RE: Regex = Regex::new(r##"<package\s*?id="(?P<name>.*?)"\s*?version="(?P<version>.*?)"(?P<inner>.*?)\s*?/>"##).unwrap();
        }

        let mut packages = match self.version {
            ProjectVersion::MicrosoftNetSdk => {
                SDK_RE.captures_iter(&self.contents)
                    .map(|cap| Package::new(&cap["name"], &cap["version"], cap["inner"].contains("<PrivateAssets>")))
                    .collect()
            },
            ProjectVersion::OldStyle => {
                // Grab them from the actual packages.config file contents.
                pta.get_other_file(&self.file, InterestingFile::PackagesConfig)
                    .and_then(|pc_path| file_loader.read_to_string(pc_path).ok())
                    .map(|pc_contents|
                        PKG_CONFIG_RE.captures_iter(&pc_contents)
                            .map(|cap| Package::new(&cap["name"],&cap["version"], cap["inner"].contains(r##"developmentDependency="true""##)))
                            .collect()
                    ).unwrap_or_default()
            },
            _ => vec![]
        };

        // Sort, dedup.
        packages.sort();
        packages.dedup();
        // TODO: specify class of the packages.
        packages
    }

    fn has_xml_doc(&self) -> XmlDoc {
        lazy_static! {
            static ref DEBUG_RE: Regex = Regex::new(r##"<DocumentationFile>bin\\[Dd]ebug\\.*?\.xml</DocumentationFile>"##).unwrap();
            static ref RELEASE_RE: Regex = Regex::new(r##"<DocumentationFile>bin\\[Rr]elease\\.*?\.xml</DocumentationFile>"##).unwrap();
        }

        match (DEBUG_RE.is_match(&self.contents), RELEASE_RE.is_match(&self.contents)) {
            (true, true) => XmlDoc::Both,
            (true, false) => XmlDoc::Debug,
            (false, true) => XmlDoc::Release,
            (false, false) => XmlDoc::None,
        }
    }

    fn has_tt_file(&self) -> bool {
        lazy_static! {
            static ref TT_REGEX: Regex = Regex::new(r##"<None (Include|Update).*?\.tt">"##).unwrap();
            static ref NUSPEC_REGEX: Regex = Regex::new(r##"<None (Include|Update).*?\.nuspec">"##).unwrap();
        }

        TT_REGEX.is_match(&self.contents) && NUSPEC_REGEX.is_match(&self.contents)
    }

    fn has_embedded_debugging(&self) -> bool {
        lazy_static! {
            // We expect both for it to be correct.
            static ref DEBUG_TYPE_REGEX: Regex = Regex::new(r##"<DebugType>embedded</DebugType>"##).unwrap();
            static ref EMBED_ALL_REGEX: Regex = Regex::new(r##"<EmbedAllSources>true</EmbedAllSources>"##).unwrap();
        }

        if self.version == ProjectVersion::MicrosoftNetSdk {
            DEBUG_TYPE_REGEX.is_match(&self.contents) && EMBED_ALL_REGEX.is_match(&self.contents)
        } else {
            false
        }
    }

    fn has_linked_solution_info(&self) -> bool {
        lazy_static! {
            static ref SOLUTION_INFO_REGEX: Regex = Regex::new(r##"[ <]Link.*?SolutionInfo\.cs.*?(</|/>)"##).unwrap();
        }

        SOLUTION_INFO_REGEX.is_match(&self.contents)
    }

    fn get_target_frameworks(&self) -> Vec<String> {
        lazy_static! {
            static ref OLD_TF_REGEX: Regex = Regex::new(r##"<TargetFrameworkVersion>(?P<tf>.*?)</TargetFrameworkVersion>"##).unwrap();
            static ref SDK_SINGLE_TF_REGEX: Regex = Regex::new(r##"<TargetFramework>(?P<tf>.*?)</TargetFramework>"##).unwrap();
            static ref SDK_MULTI_TF_REGEX: Regex = Regex::new(r##"<TargetFrameworks>(?P<tfs>.*?)</TargetFrameworks>"##).unwrap();
        }

        match self.version {
            ProjectVersion::Unknown => vec![],
            ProjectVersion::OldStyle => OLD_TF_REGEX.captures_iter(&self.contents).map(|cap| cap["tf"].to_owned()).collect(),
            ProjectVersion::MicrosoftNetSdk => {
                // One or the other will match.
                let single: Vec<_> = SDK_SINGLE_TF_REGEX.captures_iter(&self.contents).map(|cap| cap["tf"].to_owned()).collect();
                if !single.is_empty() {
                    return single;
                }

                let mut result = vec![];

                for cap in SDK_MULTI_TF_REGEX.captures_iter(&self.contents) {
                    let tfs = cap["tfs"].split(";");
                    for tf in tfs {
                        result.push(tf.to_owned());
                    }
                }

                result
            }
        }
    }

    fn get_referenced_assemblies(&self) -> Vec<String> {
        // TODO: Necessary to exclude those references that come from NuGet packages?
        // Actually the regex seems good enough, at least for the example files
        // in this project.
        lazy_static! {
            static ref ASM_REF_REGEX: Regex = Regex::new(r##"<Reference Include="(?P<name>.*?)"\s*?/>"##).unwrap();
        }

        let mut result = ASM_REF_REGEX.captures_iter(&self.contents)
            .map(|cap| cap["name"].to_owned())
            .collect::<Vec<_>>();

        result.sort();
        result.dedup();
        result
    }

    fn has_auto_generate_binding_redirects(&self) -> bool {
        self.contents.contains("<AutoGenerateBindingRedirects>true</AutoGenerateBindingRedirects>")
    }

    fn has_file_of_interest(&self, pta: &PathsToAnalyze, interesting_file: InterestingFile) -> FileStatus {
        // TODO: An optimisation would be to scan for all of these at once rather than separately.
        lazy_static! {
            static ref WEB_CONFIG_RE: Regex = RegexBuilder::new(
                    &format!("\\sInclude=\"{}\"\\s*?/>", InterestingFile::WebConfig.as_str()))
                    .case_insensitive(true).build().unwrap();

            static ref APP_CONFIG_RE: Regex = RegexBuilder::new(
                    &format!("\\sInclude=\"{}\"\\s*?/>", InterestingFile::AppConfig.as_str()))
                    .case_insensitive(true).build().unwrap();

            static ref APP_SETTINGS_JSON_RE: Regex = RegexBuilder::new(
                    &format!("\\sInclude=\"{}\"\\s*?/>", InterestingFile::AppSettingsJson.as_str()))
                    .case_insensitive(true).build().unwrap();

            static ref PACKAGE_JSON_RE: Regex = RegexBuilder::new(
                    &format!("\\sInclude=\"{}\"\\s*?/>", InterestingFile::PackageJson.as_str()))
                    .case_insensitive(true).build().unwrap();

            static ref PACKAGES_CONFIG_RE: Regex = RegexBuilder::new(
                    &format!("\\sInclude=\"{}\"\\s*?/>", InterestingFile::PackagesConfig.as_str()))
                    .case_insensitive(true).build().unwrap();

            static ref PROJECT_JSON_RE: Regex = RegexBuilder::new(
                    &format!("\\sInclude=\"{}\"\\s*?/>", InterestingFile::ProjectJson.as_str()))
                    .case_insensitive(true).build().unwrap();
        }

        let re: &Regex = match interesting_file {
            InterestingFile::WebConfig => &WEB_CONFIG_RE,
            InterestingFile::AppConfig => &APP_CONFIG_RE,
            InterestingFile::AppSettingsJson => &APP_SETTINGS_JSON_RE,
            InterestingFile::PackageJson => &PACKAGE_JSON_RE,
            InterestingFile::PackagesConfig => &PACKAGES_CONFIG_RE,
            InterestingFile::ProjectJson => &PROJECT_JSON_RE,
        };

        match (re.is_match(&self.contents), pta.project_has_other_file(&self.file, interesting_file)) {
            (true, true) => FileStatus::InProjectFileAndOnDisk,
            (true, false) => FileStatus::InProjectFileOnly,
            (false, true) => FileStatus::OnDiskOnly,
            (false, false) => FileStatus::NotPresent
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dnlib::file_loader::MemoryFileLoader;

    #[derive(Default)]
    struct ProjectBuilder {
        csproj_contents: String,
        project_version: ProjectVersion,
        packages_config_contents: Option<String>,
        paths_to_analyze: PathsToAnalyze
    }

    impl ProjectBuilder {
        fn new(csproj_contents: &str) -> Self {
            ProjectBuilder {
                csproj_contents: csproj_contents.to_owned(),
                .. ProjectBuilder::default()
            }
        }

        fn with_packages_config(mut self, packages_config_contents: &str) -> Self {
            self.packages_config_contents = Some(packages_config_contents.to_owned());
            self
        }

        fn with_paths(mut self, pta: PathsToAnalyze) -> Self {
            self.paths_to_analyze = pta;
            self
        }

        fn sdk(mut self) -> Self {
            self.project_version = ProjectVersion::MicrosoftNetSdk;
            self
        }

        fn old(mut self) -> Self {
            self.project_version = ProjectVersion::OldStyle;
            self
        }

        fn build(mut self) -> Project {
            self.csproj_contents = match self.project_version {
                ProjectVersion::OldStyle => Self::add_old_prolog(&self.csproj_contents),
                ProjectVersion::MicrosoftNetSdk => Self::add_sdk_prolog(&self.csproj_contents),
                _ => self.csproj_contents
            };

            // Always construct a pta entry for the project itself.
            let mut file_loader = MemoryFileLoader::default();
            let project_path = PathBuf::from("/temp/x.csproj");
            file_loader.files.insert(project_path.clone(), self.csproj_contents);

            // If there is a packages.config, add a pta entry for it and put the contents into the file loader.
            if self.packages_config_contents.is_some() {
                let pc_path = PathBuf::from("/temp/packages.config");
                self.paths_to_analyze.other_files.push(pc_path.clone());
                let pcc = self.packages_config_contents.unwrap();
                file_loader.files.insert(pc_path, pcc);
            }

            Project::new(&project_path, &self.paths_to_analyze, &file_loader)
        }

        fn add_sdk_prolog(contents: &str) -> String {
            format!("{}\n{}", SDK_PROLOG, contents)
        }

        fn add_old_prolog(contents: &str) -> String {
            format!("{}\n{}", OLD_PROLOG, contents)
        }
    }


    #[test]
    pub fn has_xml_doc_works() {
        let project = ProjectBuilder::new(r##""##).build();
        assert_eq!(project.xml_doc, XmlDoc::None);

        let project = ProjectBuilder::new(r##"blah<DocumentationFile>bin\Debug\WorkflowService.Client.xml</DocumentationFile>blah"##).build();
        assert_eq!(project.xml_doc, XmlDoc::Debug);

        let project = ProjectBuilder::new(r##"blah<DocumentationFile>bin\Release\WorkflowService.Client.xml</DocumentationFile>blah"##).build();
        assert_eq!(project.xml_doc, XmlDoc::Release);

        let project = ProjectBuilder::new(r##"blah<DocumentationFile>bin\Release\WorkflowService.Client.xml</DocumentationFile>
            <DocumentationFile>bin\Debug\WorkflowService.Client.xml</DocumentationFile>blah"##).build();
        assert_eq!(project.xml_doc, XmlDoc::Both);
    }

    #[test]
    pub fn has_tt_file_works() {
        let project = ProjectBuilder::new(r##""##).build();
        assert!(!project.tt_file);

        let project = ProjectBuilder::new(r##"blah<None Update="NuSpecTemplate.tt">blah"##).build();
        assert!(!project.tt_file);

        let project = ProjectBuilder::new(r##"blah<None Update="NuSpecTemplate.nuspec">blah"##).build();
        assert!(!project.tt_file);

        let project = ProjectBuilder::new(r##"blah<None Update="NuSpecTemplate.nuspec">blah
            <None Update="NuSpecTemplate.tt">blah"##).build();
        assert!(project.tt_file);

        let project = ProjectBuilder::new(r##"blah<None Include="NuSpecTemplate.nuspec">blah
            <None Include="NuSpecTemplate.tt">blah"##).build();
        assert!(project.tt_file);
    }

    #[test]
    pub fn has_embedded_debugging_works() {
        let project = ProjectBuilder::new(r##""##).build();
        assert!(!project.embedded_debugging);

        let project = ProjectBuilder::new(r##"blah<DebugType>embedded</DebugType>blah"##).build();
        assert!(!project.embedded_debugging);

        let project = ProjectBuilder::new(r##"blah<EmbedAllSources>true</EmbedAllSources>blah"##).build();
        assert!(!project.embedded_debugging);

        let project = ProjectBuilder::new(r##"blah<DebugType>embedded</DebugType>blah"
            <EmbedAllSources>true</EmbedAllSources>blah"##).sdk().build();
        assert!(project.embedded_debugging);
    }

    #[test]
    pub fn has_linked_solution_info_works() {
        let project = ProjectBuilder::new(r##""##).build();
        assert!(!project.linked_solution_info);

        // SDK style.
        let project = ProjectBuilder::new(r##"blah<ItemGroup>
            <Compile Include="..\SolutionInfo.cs" Link="Properties\SolutionInfo.cs" />blah
            </ItemGroup>blah"##).build();
        assert!(project.linked_solution_info);

        // Old style.
        let project = ProjectBuilder::new(r##"blah<Compile Include="..\SolutionInfo.cs">
            <Link>Properties\SolutionInfo.cs</Link>blah
            </Compile>blah"##).build();
        assert!(project.linked_solution_info);
    }

    #[test]
    pub fn sdk_get_target_frameworks_works() {
        let project = ProjectBuilder::new(r##""##).build();
        assert!(project.target_frameworks.is_empty());

        let project = ProjectBuilder::new(r##"blah<TargetFramework>net462</TargetFramework>blah"##).sdk().build();
        assert_eq!(project.target_frameworks, vec!["net462"]);

        // I don't believe this happens, but this is what we get.
        let project = ProjectBuilder::new(r##"blah<TargetFramework>net462</TargetFramework>blah<TargetFramework>net472</TargetFramework>"##).sdk().build();
        assert_eq!(project.target_frameworks, vec!["net462", "net472"]);

        let project = ProjectBuilder::new(r##"blah<TargetFrameworks>net462;net472</TargetFrameworks>blah"##).sdk().build();
        assert_eq!(project.target_frameworks, vec!["net462", "net472"]);
    }

    #[test]
    pub fn old_get_target_frameworks_works() {
        let project = ProjectBuilder::new(r##""##).build();
        assert!(project.target_frameworks.is_empty());

        let project = ProjectBuilder::new(r##"blah<TargetFrameworkVersion>v4.6.2</TargetFrameworkVersion>blah"##).old().build();
        assert_eq!(project.target_frameworks, vec!["v4.6.2"]);

        let project = ProjectBuilder::new(r##"blah<TargetFrameworkVersion>v4.6.2</TargetFrameworkVersion>blah
            <TargetFrameworkVersion>v4.7.2</TargetFrameworkVersion>"##).old().build();
        assert_eq!(project.target_frameworks, vec!["v4.6.2", "v4.7.2"]);
    }

    #[test]
    pub fn get_referenced_assemblies_works() {
        let project = ProjectBuilder::new(r##""##).build();
        assert!(project.referenced_assemblies.is_empty());

        let project = ProjectBuilder::new(r##"blah<Reference Include="System.Windows" />blah"##).build();
        assert_eq!(project.referenced_assemblies, vec!["System.Windows"]);

        let project = ProjectBuilder::new(r##"blah<Reference Include="System.Windows" />blah
            blah<Reference Include="System.Windows" />blah"##).build();
        assert_eq!(project.referenced_assemblies, vec!["System.Windows"]);

        let project = ProjectBuilder::new(r##"blah<Reference Include="System.Windows" />blah
            blah<Reference Include="System.Data" />blah"##).build();
        assert_eq!(project.referenced_assemblies, vec!["System.Data", "System.Windows"]);
    }

    #[test]
    pub fn has_auto_generate_binding_redirects_works() {
        let project = ProjectBuilder::new(r##""##).build();
        assert!(!project.auto_generate_binding_redirects);

        let project = ProjectBuilder::new(r##"blah<AutoGenerateBindingRedirects>true</AutoGenerateBindingRedirects>blah"##).build();
        assert!(project.auto_generate_binding_redirects);

        let project = ProjectBuilder::new(r##"blah<AutoGenerateBindingRedirects>false</AutoGenerateBindingRedirects>blah"##).build();
        assert!(!project.auto_generate_binding_redirects);
    }

    #[test]
    pub fn has_packages_config_not_present() {
        let project = ProjectBuilder::new(r##""##).build();
        assert_eq!(project.packages_config, FileStatus::NotPresent);
    }

    #[test]
    pub fn has_packages_config_on_disk() {
        let project = ProjectBuilder::new(r##""##).with_packages_config("contents").build();
        assert_eq!(project.packages_config, FileStatus::OnDiskOnly);
    }

    #[test]
    pub fn has_packages_config_in_project_file_only() {
        let project = ProjectBuilder::new(r##" Include="packages.config" />"##).build();
        assert_eq!(project.packages_config, FileStatus::InProjectFileOnly);
    }

    #[test]
    pub fn has_packages_config_in_project_file_and_on_disk() {
        let project = ProjectBuilder::new(r##" Include="packages.config" />"##).with_packages_config("contents").build();
        assert_eq!(project.packages_config, FileStatus::InProjectFileAndOnDisk);
    }

    #[test]
    pub fn get_packages_sdk_one_line() {
        let project = ProjectBuilder::new(r##""##).sdk().build();
        assert!(project.packages.is_empty());

        let project = ProjectBuilder::new(r##"blah<PackageReference Include="Unity" Version="4.0.1" />blah"##).sdk().build();
        assert_eq!(project.packages, vec![Package::new("Unity", "4.0.1", false)]);
    }

    #[test]
    pub fn get_packages_sdk_one_line_sorts() {
        let project = ProjectBuilder::new(
            r##"
            blah<PackageReference Include="Unity" Version="4.0.1" />blah
            blah<PackageReference Include="Automapper" Version="3.1.4" />blah
            "##
            ).sdk().build();
        assert_eq!(project.packages, vec![
            Package::new("Automapper", "3.1.4", false),
            Package::new("Unity", "4.0.1", false)
            ]);

        // Dedup & sort by secondary key (version).
        let project = ProjectBuilder::new(
            r##"
            blah<PackageReference Include="Automapper" Version="3.1.5" />blah
            blah<PackageReference Include="Unity" Version="4.0.1" />blah
            blah<PackageReference Include="Automapper" Version="3.1.4" />blah
            blah<PackageReference Include="Unity" Version="4.0.1" />blah
            "##
            ).sdk().build();
        assert_eq!(project.packages, vec![
            Package::new("Automapper", "3.1.4", false),
            Package::new("Automapper", "3.1.5", false),
            Package::new("Unity", "4.0.1", false)
            ]);
    }

    #[test]
    pub fn get_packages_sdk_one_line_dedups() {
        // Dedup & sort by secondary key (i.e. the version).
        let project = ProjectBuilder::new(
            r##"
            blah<PackageReference Include="Automapper" Version="3.1.5" />blah
            blah<PackageReference Include="Unity" Version="4.0.1" />blah
            blah<PackageReference Include="Automapper" Version="3.1.4" />blah
            blah<PackageReference Include="Unity" Version="4.0.1" />blah
            "##
            ).sdk().build();
        assert_eq!(project.packages, vec![
            Package::new("Automapper", "3.1.4", false),
            Package::new("Automapper", "3.1.5", false),
            Package::new("Unity", "4.0.1", false)
            ]);
    }

    #[test]
    pub fn get_packages_sdk_multi_line() {
        let project = ProjectBuilder::new(
            r##"
            blah<PackageReference Include="Unity" Version="4.0.1">
                </PackageReference>
            "##
        ).sdk().build();
        assert_eq!(project.packages, vec![
            Package::new("Unity", "4.0.1", false)
            ]);
    }

    #[test]
    pub fn get_packages_sdk_multi_line_private_assets() {
        let project = ProjectBuilder::new(
            r##"
            blah<PackageReference Include="Unity" Version="4.0.1">
                <PrivateAssets>
                </PackageReference>
            "##
        ).sdk().build();
        assert_eq!(project.packages, vec![
            Package::new("Unity", "4.0.1", true)
            ]);
    }

    #[test]
    pub fn get_packages_sdk_multi_line_flip_flop() {
        // This flip-flop of styles discovered problems in the regex when it
        // was not terminating early enough.
        let project = ProjectBuilder::new(
            r##"
            blah<PackageReference Include="Unity" Version="4.0.1">
                </PackageReference>

                <PackageReference Include="EntityFramework" Version="2.4.6" />

                <PackageReference Include="Automapper" Version="3.1.4">
                    <PrivateAssets>
                </PackageReference>

                <PackageReference Include="Versioning.Bamboo" Version="8.8.9" />
            "##
        ).sdk().build();
        assert_eq!(project.packages, vec![
            Package::new("Automapper", "3.1.4", true),
            Package::new("EntityFramework", "2.4.6", false),
            Package::new("Unity", "4.0.1", false),
            Package::new("Versioning.Bamboo", "8.8.9", false)
            ]);
    }

    #[test]
    pub fn get_packages_old_including_sort_and_dedup() {
        let project = ProjectBuilder::new(r##" Include="packages.config" />"##).old()
            .with_packages_config(r##"
            <package id="Clarius.TransformOnBuild" version="1.1.12" targetFramework="net462" developmentDependency="true" />
            <package id="Castle.Core" version="4.3.1" targetFramework="net462" />
            <package id="Owin" version="1.0" targetFramework="net462" />
            <package id="Castle.Core" version="4.3.1" targetFramework="net462" />
            "##).build();
        assert_eq!(project.packages, vec![
            Package::new("Castle.Core", "4.3.1", false),
            Package::new("Clarius.TransformOnBuild", "1.1.12", true),
            Package::new("Owin", "1.0", false),
        ]);
    }


    /// These tests run against the embedded example SDK-style project.
    /// They are an extra sanity-check that we really got it right "in the real world".
    mod sdk_tests {
        use super::*;

        fn get_sdk_project() -> Project {
            ProjectBuilder::new(include_str!("sdk1.csproj.xml")).sdk().build()
        }

        #[test]
        pub fn can_detect_version() {
            let project = get_sdk_project();
            assert_eq!(project.version, ProjectVersion::MicrosoftNetSdk);
        }

        #[test]
        pub fn can_detect_xml_doc() {
            let project = get_sdk_project();
            assert_eq!(project.xml_doc, XmlDoc::Both);
        }

        #[test]
        pub fn can_detect_tt_file() {
            let project = get_sdk_project();
            assert!(project.tt_file);
        }

        #[test]
        pub fn can_detect_embedded_debugging() {
            let project = get_sdk_project();
            assert!(project.embedded_debugging);
        }

        #[test]
        pub fn can_detect_linked_solution_info() {
            let project = get_sdk_project();
            assert!(project.linked_solution_info);
        }

        #[test]
        pub fn can_detect_target_framework() {
            let project = get_sdk_project();
            assert_eq!(project.target_frameworks, vec!["net462"]);
        }

        #[test]
        pub fn can_detect_referenced_assemblies() {
            let project = get_sdk_project();
            assert_eq!(project.referenced_assemblies, vec!["System.Configuration", "System.Windows"]);
        }

        #[test]
        pub fn can_detect_has_auto_generate_binding_redirects() {
            let project = get_sdk_project();
            assert!(project.auto_generate_binding_redirects);
        }

        #[test]
        pub fn can_detect_web_config() {
            let project = get_sdk_project();
            assert_eq!(project.web_config, FileStatus::NotPresent);
        }

        #[test]
        pub fn can_detect_app_config() {
            let project = get_sdk_project();
            assert_eq!(project.app_config, FileStatus::NotPresent);
        }

        #[test]
        pub fn can_detect_app_settings_json() {
            let project = get_sdk_project();
            assert_eq!(project.app_settings_json, FileStatus::NotPresent);
        }

        #[test]
        pub fn can_detect_package_json() {
            let project = get_sdk_project();
            assert_eq!(project.package_json, FileStatus::NotPresent);
        }

        #[test]
        pub fn can_detect_packages_config() {
            let project = get_sdk_project();
            assert_eq!(project.packages_config, FileStatus::NotPresent);
        }

        #[test]
        pub fn can_detect_project_json() {
            let project = get_sdk_project();
            assert_eq!(project.project_json, FileStatus::NotPresent);
        }

        #[test]
        pub fn can_detect_output_type() {
            let project = get_sdk_project();
            assert_eq!(project.output_type, OutputType::Library);
        }
    }



    /// These tests run against the embedded example old-style project.
    /// They are an extra sanity-check that we really got it right "in the real world".
    mod old_style_tests {
        use super::*;

        fn get_old_project() -> Project {
            ProjectBuilder::new(include_str!("old1.csproj.xml")).old().build()
        }

        #[test]
        pub fn can_detect_version() {
            let project = get_old_project();
            assert_eq!(project.version, ProjectVersion::OldStyle);
        }

        #[test]
        pub fn can_detect_xml_doc() {
            let project = get_old_project();
            assert_eq!(project.xml_doc, XmlDoc::Both);
        }

        #[test]
        pub fn can_detect_tt_file() {
            let project = get_old_project();
            assert!(project.tt_file);
        }

        #[test]
        pub fn embedded_debugging_is_always_false() {
            let project = get_old_project();
            assert!(!project.embedded_debugging);
        }

        #[test]
        pub fn can_detect_linked_solution_info() {
            let project = get_old_project();
            assert!(project.linked_solution_info);
        }

        #[test]
        pub fn can_detect_target_framework() {
            let project = get_old_project();
            assert_eq!(project.target_frameworks, vec!["v4.6.2"]);
        }

        #[test]
        pub fn can_detect_referenced_assemblies() {
            let project = get_old_project();
            assert_eq!(project.referenced_assemblies,
                    vec!["PresentationCore", "PresentationFramework", "System", "System.Activities",
                            "System.Core", "System.Net.Http", "System.Xml", "System.configuration",
                            "WindowsBase"]);
        }

        #[test]
        pub fn can_detect_has_auto_generate_binding_redirects() {
            let project = get_old_project();
            assert!(!project.auto_generate_binding_redirects);
        }

        #[test]
        pub fn can_detect_web_config() {
            let project = get_old_project();
            assert_eq!(project.web_config, FileStatus::NotPresent);
        }

        #[test]
        pub fn can_detect_app_config() {
            let project = get_old_project();
            assert_eq!(project.app_config, FileStatus::InProjectFileOnly);
        }

        #[test]
        pub fn can_detect_app_settings_json() {
            let project = get_old_project();
            assert_eq!(project.app_settings_json, FileStatus::NotPresent);
        }

        #[test]
        pub fn can_detect_package_json() {
            let project = get_old_project();
            assert_eq!(project.package_json, FileStatus::NotPresent);
        }

        #[test]
        pub fn can_detect_packages_config() {
            let project = get_old_project();
            assert_eq!(project.packages_config, FileStatus::InProjectFileOnly);
        }

        #[test]
        pub fn can_detect_project_json() {
            let project = get_old_project();
            assert_eq!(project.project_json, FileStatus::NotPresent);
        }

        #[test]
        pub fn can_detect_output_type() {
            let project = get_old_project();
            assert_eq!(project.output_type, OutputType::Library);
        }
    }
}
