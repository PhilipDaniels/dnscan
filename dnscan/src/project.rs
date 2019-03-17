use crate::find_files::{PathsToAnalyze, InterestingFile};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use regex::Regex;
use lazy_static::lazy_static;

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
    Exe,
    Library,
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
    pub project_json: FileStatus,
    pub packages_config: FileStatus,

    pub referenced_assemblies: Vec<String>,
    pub packages: Vec<Package>,
    pub referenced_projects: Vec<Arc<Project>>,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
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

#[derive(Debug, Default)]
pub struct Package {
    pub name: String,
    pub version: String,
    pub development: bool,
    pub class: PackageClass
}

const SDK_PROLOG: &str = "<Project Sdk=\"Microsoft.NET.Sdk\">";
const OLD_PROLOG: &str = "<Project ToolsVersion=";

impl Project {
    pub fn new(project_file_path: &Path, pta: &PathsToAnalyze) -> Self {
        let mut proj = Project::default();
        proj.file = project_file_path.to_owned();

        match std::fs::read_to_string(project_file_path) {
            Ok(s) => {
                proj.is_valid_utf8 = true;
                proj.analyze(pta, s);
            },
            Err(_) => {
                proj.is_valid_utf8 = false;
            }
        }

        proj
    }

    /// Factor the guts of the analysis out into a separate function so that it
    /// can be easily unit tested.
    fn analyze(&mut self, pta: &PathsToAnalyze, contents: String) {
        self.contents = contents;

        self.version = if self.contents.contains(SDK_PROLOG) {
             ProjectVersion::MicrosoftNetSdk
        } else if self.contents.contains(OLD_PROLOG) {
            ProjectVersion::OldStyle
        } else {
            ProjectVersion::Unknown
        };

        self.xml_doc = self.has_xml_doc();
        self.tt_file = self.has_tt_file();
        self.linked_solution_info = self.has_linked_solution_info();
        self.referenced_assemblies = self.get_referenced_assemblies();
        self.auto_generate_binding_redirects = self.has_auto_generate_binding_redirects();

        // pub web_config: FileStatus,
        // pub app_config: FileStatus,
        // pub app_settings_json: FileStatus,
        // pub project_json: FileStatus,
        self.packages_config = self.has_packages_config(pta);

        // pub packages: Vec<Package>,
        // pub referenced_projects: Vec<Arc<Project>>,
        // pub test_framework: String,

        if self.version == ProjectVersion::MicrosoftNetSdk {
            self.embedded_debugging = self.has_embedded_debugging();
            self.target_frameworks = self.sdk_get_target_frameworks();
        } else if self.version == ProjectVersion::OldStyle {
            self.embedded_debugging = false;
            self.target_frameworks = self.old_get_target_frameworks();
        }
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

        DEBUG_TYPE_REGEX.is_match(&self.contents) && EMBED_ALL_REGEX.is_match(&self.contents)
    }

    fn has_linked_solution_info(&self) -> bool {
        lazy_static! {
            static ref SOLUTION_INFO_REGEX: Regex = Regex::new(r##"[ <]Link.*?SolutionInfo\.cs.*?(</|/>)"##).unwrap();
        }

        SOLUTION_INFO_REGEX.is_match(&self.contents)
    }

    fn sdk_get_target_frameworks(&self) -> Vec<String> {
        lazy_static! {
            static ref SINGLE_TF_REGEX: Regex = Regex::new(r##"<TargetFramework>(?P<tf>.*?)</TargetFramework>"##).unwrap();
            static ref MULTI_TF_REGEX: Regex = Regex::new(r##"<TargetFrameworks>(?P<tfs>.*?)</TargetFrameworks>"##).unwrap();
        }

        // One or the other will match.
        let single: Vec<_> = SINGLE_TF_REGEX.captures_iter(&self.contents).map(|cap| cap["tf"].to_owned()).collect();
        if !single.is_empty() {
            return single;
        }

        let mut result = vec![];

        for cap in MULTI_TF_REGEX.captures_iter(&self.contents) {
             let tfs = cap["tfs"].split(";");
             for tf in tfs {
                 result.push(tf.to_owned());
             }
        }

        result

        // TODO: This won't compile.
        // MULTI_TF_REGEX.captures_iter(contents)
        //     .flat_map(|cap| cap["tfs"].split(";"))
        //     .map(|s| s.to_owned())
        //     .collect()
    }

    fn old_get_target_frameworks(&self) -> Vec<String> {
        lazy_static! {
            static ref TF_REGEX: Regex = Regex::new(r##"<TargetFrameworkVersion>(?P<tf>.*?)</TargetFrameworkVersion>"##).unwrap();
        }

        TF_REGEX.captures_iter(&self.contents).map(|cap| cap["tf"].to_owned()).collect()
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

    fn has_packages_config(&self, pta: &PathsToAnalyze) -> FileStatus {
        lazy_static! {
            static ref PKG_CONFIG_RE: Regex = Regex::new(r##"\sInclude="[Pp]ackages.[Cc]onfig"\s*?/>"##).unwrap();
        }

        match (PKG_CONFIG_RE.is_match(&self.contents), pta.project_has_other_file(&self.file, InterestingFile::PackagesConfig)) {
            (true, true) => FileStatus::InProjectFileAndOnDisk,
            (true, false) => FileStatus::InProjectFileOnly,
            (false, true) => FileStatus::OnDiskOnly,
            (false, false) => FileStatus::NotPresent
        }
    }
}

#[cfg(test)]
mod general_tests {
    use super::*;
    use std::str::FromStr;

    fn add_sdk_prolog(contents: &str) -> String {
        format!("{}\n{}", SDK_PROLOG, contents)
    }

    fn add_old_prolog(contents: &str) -> String {
        format!("{}\n{}", OLD_PROLOG, contents)
    }

    fn analyze(csproj_contents: &str) -> Project {
        let mut proj = Project::default();
        proj.analyze(&PathsToAnalyze::default(), csproj_contents.to_owned());
        proj
    }

    fn analyze_with_paths(pta: PathsToAnalyze, csproj_contents: &str) -> Project {
        let mut proj = Project::default();
        proj.analyze(&pta, csproj_contents.to_owned());
        proj
    }

    #[test]
    pub fn has_xml_doc_works() {
        let result = analyze(r##""##);
        assert_eq!(result.xml_doc, XmlDoc::None);

        let result = analyze(r##"blah<DocumentationFile>bin\Debug\WorkflowService.Client.xml</DocumentationFile>blah"##);
        assert_eq!(result.xml_doc, XmlDoc::Debug);

        let result = analyze(r##"blah<DocumentationFile>bin\Release\WorkflowService.Client.xml</DocumentationFile>blah"##);
        assert_eq!(result.xml_doc, XmlDoc::Release);

        let result = analyze(r##"blah<DocumentationFile>bin\Release\WorkflowService.Client.xml</DocumentationFile>
            <DocumentationFile>bin\Debug\WorkflowService.Client.xml</DocumentationFile>blah"##);
        assert_eq!(result.xml_doc, XmlDoc::Both);
    }

    #[test]
    pub fn has_tt_file_works() {
        let result = analyze(r##""##);
        assert_eq!(result.tt_file, false);

        let result = analyze(r##"blah<None Update="NuSpecTemplate.tt">blah"##);
        assert_eq!(result.tt_file, false);

        let result = analyze(r##"blah<None Update="NuSpecTemplate.nuspec">blah"##);
        assert_eq!(result.tt_file, false);

        let result = analyze(r##"blah<None Update="NuSpecTemplate.nuspec">blah
            <None Update="NuSpecTemplate.tt">blah"##);
        assert_eq!(result.tt_file, true);

        let result = analyze(r##"blah<None Include="NuSpecTemplate.nuspec">blah
            <None Include="NuSpecTemplate.tt">blah"##);
        assert_eq!(result.tt_file, true);
    }

    #[test]
    pub fn has_embedded_debugging_works() {
        let result = analyze(r##""##);
        assert_eq!(result.has_embedded_debugging(), false);

        let result = analyze(r##"blah<DebugType>embedded</DebugType>blah"##);
        assert_eq!(result.has_embedded_debugging(), false);

        let result = analyze(r##"blah<EmbedAllSources>true</EmbedAllSources>blah"##);
        assert_eq!(result.has_embedded_debugging(), false);

        // TODO: I think this should be failing because not detected as an SDK style project!
        let result = analyze(r##"blah<DebugType>embedded</DebugType>blah"
            <EmbedAllSources>true</EmbedAllSources>blah"##);
        assert_eq!(result.has_embedded_debugging(), true);
    }

    #[test]
    pub fn has_linked_solution_info_works() {
        let result = analyze(r##""##);
        assert_eq!(result.has_linked_solution_info(), false);

        // SDK style.
        let result = analyze(r##"blah<ItemGroup>
            <Compile Include="..\SolutionInfo.cs" Link="Properties\SolutionInfo.cs" />blah
            </ItemGroup>blah"##);
        assert_eq!(result.has_linked_solution_info(), true);

        // Old style.
        let result = analyze(r##"blah<Compile Include="..\SolutionInfo.cs">
            <Link>Properties\SolutionInfo.cs</Link>blah
            </Compile>blah"##);
        assert_eq!(result.has_linked_solution_info(), true);
    }

    #[test]
    pub fn sdk_get_target_frameworks_works() {
        let result = analyze(r##""##);
        assert!(result.target_frameworks.is_empty());

        let result = analyze(&add_sdk_prolog(r##"blah<TargetFramework>net462</TargetFramework>blah"##));
        assert_eq!(result.target_frameworks, vec!["net462"]);

        // I don't believe this happens, but this is what we get.
        let result = analyze(&add_sdk_prolog(r##"blah<TargetFramework>net462</TargetFramework>blah<TargetFramework>net472</TargetFramework>"##));
        assert_eq!(result.target_frameworks, vec!["net462", "net472"]);

        let result = analyze(&add_sdk_prolog(r##"blah<TargetFrameworks>net462;net472</TargetFrameworks>blah"##));
        assert_eq!(result.target_frameworks, vec!["net462", "net472"]);
    }

    #[test]
    pub fn old_get_target_frameworks_works() {
        let result = analyze(r##""##);
        assert!(result.target_frameworks.is_empty());

        let result = analyze(&add_old_prolog(r##"blah<TargetFrameworkVersion>v4.6.2</TargetFrameworkVersion>blah"##));
        assert_eq!(result.target_frameworks, vec!["v4.6.2"]);

        let result = analyze(&add_old_prolog(r##"blah<TargetFrameworkVersion>v4.6.2</TargetFrameworkVersion>blah
            <TargetFrameworkVersion>v4.7.2</TargetFrameworkVersion>"##));
        assert_eq!(result.target_frameworks, vec!["v4.6.2", "v4.7.2"]);
    }

    #[test]
    pub fn get_referenced_assemblies_works() {
        let result = analyze(r##""##);
        assert!(result.referenced_assemblies.is_empty());

        let result = analyze(r##"blah<Reference Include="System.Windows" />blah"##);
        assert_eq!(result.referenced_assemblies, vec!["System.Windows"]);

        let result = analyze(r##"blah<Reference Include="System.Windows" />blah
        blah<Reference Include="System.Windows" />blah"##);
        assert_eq!(result.referenced_assemblies, vec!["System.Windows"]);

        let result = analyze(r##"blah<Reference Include="System.Windows" />blah
        blah<Reference Include="System.Data" />blah"##);
        assert_eq!(result.referenced_assemblies, vec!["System.Data", "System.Windows"]);
    }

    #[test]
    pub fn has_auto_generate_binding_redirects_works() {
        let result = analyze(r##""##);
        assert!(result.auto_generate_binding_redirects == false);

        let result = analyze(r##"blah<AutoGenerateBindingRedirects>true</AutoGenerateBindingRedirects>blah"##);
        assert!(result.auto_generate_binding_redirects == true);

        let result = analyze(r##"blah<AutoGenerateBindingRedirects>false</AutoGenerateBindingRedirects>blah"##);
        assert!(result.auto_generate_binding_redirects == false);
    }


    #[test]
    pub fn has_packages_config_not_present() {
        let result = analyze_with_paths(PathsToAnalyze::default(), "");
        assert_eq!(result.packages_config, FileStatus::NotPresent);
    }

    #[test]
    pub fn has_packages_config_on_disk() {
        let mut pta = PathsToAnalyze::default();
        pta.other_files.push(PathBuf::from_str("/temp/packages.config").unwrap());

        let mut proj = Project::default();
        proj.file = PathBuf::from_str("/temp/foo.csproj").unwrap();

        proj.analyze(&pta, "".to_owned());

        assert_eq!(proj.packages_config, FileStatus::OnDiskOnly);
    }

    #[test]
    pub fn has_packages_config_in_project_file_only() {
        let mut proj = Project::default();
        proj.file = PathBuf::from_str("/temp/foo.csproj").unwrap();

        proj.analyze(&PathsToAnalyze::default(), r##" Include="packages.config" />"##.to_owned());

        assert_eq!(proj.packages_config, FileStatus::InProjectFileOnly);
    }

    #[test]
    pub fn has_packages_config_in_project_file_and_on_disk() {
        let mut pta = PathsToAnalyze::default();
        pta.other_files.push(PathBuf::from_str("/temp/packages.config").unwrap());

        let mut proj = Project::default();
        proj.file = PathBuf::from_str("/temp/foo.csproj").unwrap();

        proj.analyze(&pta, r##" Include="packages.config" />"##.to_owned());

        assert_eq!(proj.packages_config, FileStatus::InProjectFileAndOnDisk);
    }
}

#[cfg(test)]
mod sdk_tests {
    use super::*;

    fn sdk_csproj() -> String {
        include_str!("sdk1.csproj.xml").to_owned()
    }

    fn pta() -> PathsToAnalyze {
        PathsToAnalyze::default()
    }

    fn analyze() -> Project {
        let mut proj = Project::default();
        let pta = pta();
        proj.analyze(&pta, sdk_csproj());
        proj
    }

    #[test]
    pub fn can_detect_version() {
        let proj = analyze();
        assert_eq!(proj.version, ProjectVersion::MicrosoftNetSdk);
    }

    #[test]
    pub fn can_detect_xml_doc() {
        let proj = analyze();
        assert_eq!(proj.xml_doc, XmlDoc::Both);
    }

    #[test]
    pub fn can_detect_tt_file() {
        let proj = analyze();
        assert_eq!(proj.tt_file, true);
    }

    #[test]
    pub fn can_detect_embedded_debugging() {
        let proj = analyze();
        assert_eq!(proj.embedded_debugging, true);
    }

    #[test]
    pub fn can_detect_linked_solution_info() {
        let proj = analyze();
        assert_eq!(proj.linked_solution_info, true);
    }

    #[test]
    pub fn can_detect_target_framework() {
        let proj = analyze();
        assert_eq!(proj.target_frameworks, vec!["net462"]);
    }

    #[test]
    pub fn can_detect_referenced_assemblies() {
        let proj = analyze();
        assert_eq!(proj.referenced_assemblies, vec!["System.Configuration", "System.Windows"]);
    }

    #[test]
    pub fn can_detect_has_auto_generate_binding_redirects() {
        let proj = analyze();
        assert!(proj.auto_generate_binding_redirects == true);
    }

    #[test]
    pub fn can_detect_packages_config() {
        let proj = analyze();
        assert_eq!(proj.packages_config, FileStatus::NotPresent);
    }
}

#[cfg(test)]
mod old_style_tests {
    use super::*;

    fn old_style_csproj() -> String {
        include_str!("old1.csproj.xml").to_owned()
    }

    fn pta() -> PathsToAnalyze {
        PathsToAnalyze::default()
    }

    fn analyze() -> Project {
        let mut proj = Project::default();
        let pta = pta();
        proj.analyze(&pta, old_style_csproj());
        proj
    }

    #[test]
    pub fn can_detect_version() {
        let proj = analyze();
        assert_eq!(proj.version, ProjectVersion::OldStyle);
    }

    #[test]
    pub fn can_detect_xml_doc() {
        let proj = analyze();
        assert_eq!(proj.xml_doc, XmlDoc::Both);
    }

    #[test]
    pub fn can_detect_tt_file() {
        let proj = analyze();
        assert_eq!(proj.tt_file, true);
    }

    #[test]
    pub fn embedded_debugging_is_always_false() {
        let proj = analyze();
        assert_eq!(proj.embedded_debugging, false);
    }

    #[test]
    pub fn can_detect_linked_solution_info() {
        let proj = analyze();
        assert_eq!(proj.linked_solution_info, true);
    }

    #[test]
    pub fn can_detect_target_framework() {
        let proj = analyze();
        assert_eq!(proj.target_frameworks, vec!["v4.6.2"]);
    }

    #[test]
    pub fn can_detect_referenced_assemblies() {
        let proj = analyze();
        assert_eq!(proj.referenced_assemblies,
                   vec!["PresentationCore", "PresentationFramework", "System", "System.Activities",
                        "System.Core", "System.Net.Http", "System.Xml", "System.configuration",
                        "WindowsBase"]);
    }

    #[test]
    pub fn can_detect_has_auto_generate_binding_redirects() {
        let proj = analyze();
        assert!(proj.auto_generate_binding_redirects == false);
    }

    #[test]
    pub fn can_detect_packages_config() {
        let proj = analyze();
        assert_eq!(proj.packages_config, FileStatus::InProjectFileOnly);
    }
}
