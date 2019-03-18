use crate::find_files::{PathsToAnalyze, InterestingFile};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use regex::{Regex, RegexBuilder};
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
        self.packages = self.get_packages();
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

    fn get_packages(&self) -> Vec<Package> {
        lazy_static! {
            static ref SDK_RE: Regex = Regex::new(r##"(?s)<PackageReference\s*?Include="(?P<name>.*?)"\s*?Version="(?P<version>.*?)"(?P<inner>.*?)(/>|</PackageReference>)"##).unwrap();
        }

        let mut packages = match self.version {
            ProjectVersion::MicrosoftNetSdk => {
                // for c in SDK_RE.captures_iter(&self.contents) {
                //     println!("Single line Got c = {:#?}", c);
                // }

                SDK_RE.captures_iter(&self.contents)
                    .map(|cap| Package::new(
                        &cap["name"],
                        &cap["version"],
                        cap["inner"].contains("<PrivateAssets>")
                    ))
                    .collect()
            },
            ProjectVersion::OldStyle => vec![],
            _ => vec![]
        };

        // Sort, dedup, specify class.
        packages.sort();
        packages.dedup();
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

        let result = analyze(&add_sdk_prolog(r##"blah<DebugType>embedded</DebugType>blah"
            <EmbedAllSources>true</EmbedAllSources>blah"##));
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

    #[test]
    pub fn get_packages_sdk_simple() {
        let result = analyze(&add_sdk_prolog(r##""##));
        assert!(result.packages.is_empty());

        let result = analyze(&add_sdk_prolog(r##"blah<PackageReference Include="Unity" Version="4.0.1" />blah"##));
        assert_eq!(result.packages, vec![Package::new("Unity", "4.0.1", false)]);

        // Sort test.
        let result = analyze(&add_sdk_prolog(
            r##"
            blah<PackageReference Include="Unity" Version="4.0.1" />blah
            blah<PackageReference Include="Automapper" Version="3.1.4" />blah
            "##
        ));
        assert_eq!(result.packages, vec![
            Package::new("Automapper", "3.1.4", false),
            Package::new("Unity", "4.0.1", false)
            ]);

        // Dedup & sort by secondary key (version).
        let result = analyze(&add_sdk_prolog(
            r##"
            blah<PackageReference Include="Automapper" Version="3.1.5" />blah
            blah<PackageReference Include="Unity" Version="4.0.1" />blah
            blah<PackageReference Include="Automapper" Version="3.1.4" />blah
            blah<PackageReference Include="Unity" Version="4.0.1" />blah
            "##
        ));
        assert_eq!(result.packages, vec![
            Package::new("Automapper", "3.1.4", false),
            Package::new("Automapper", "3.1.5", false),
            Package::new("Unity", "4.0.1", false)
            ]);
    }

    #[test]
    pub fn get_packages_sdk_complex() {
        let result = analyze(&add_sdk_prolog(
            r##"
            blah<PackageReference Include="Unity" Version="4.0.1">
                <PrivateAssets>
                </PackageReference>
            "##
        ));
        assert_eq!(result.packages, vec![
            Package::new("Unity", "4.0.1", true)
            ]);


        let result = analyze(&add_sdk_prolog(
            r##"
            blah<PackageReference Include="Unity" Version="4.0.1">
                </PackageReference>
            "##
        ));
        assert_eq!(result.packages, vec![
            Package::new("Unity", "4.0.1", false)
            ]);


        // This flip-flop of styles discovered problems in the regex when it
        // was not terminating early enough.
        let result = analyze(&add_sdk_prolog(
            r##"
            blah<PackageReference Include="Unity" Version="4.0.1">
                </PackageReference>

                <PackageReference Include="EntityFramework" Version="2.4.6" />

                <PackageReference Include="Automapper" Version="3.1.4">
                    <PrivateAssets>
                </PackageReference>

                <PackageReference Include="Versioning.Bamboo" Version="8.8.9" />
            "##
        ));
        assert_eq!(result.packages, vec![
            Package::new("Automapper", "3.1.4", true),
            Package::new("EntityFramework", "2.4.6", false),
            Package::new("Unity", "4.0.1", false),
            Package::new("Versioning.Bamboo", "8.8.9", false)
            ]);
    }

    #[test]
    pub fn get_packages_old_simple() {

    }

    #[test]
    pub fn get_packages_old_complex() {

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
    pub fn can_detect_web_config() {
        let proj = analyze();
        assert_eq!(proj.web_config, FileStatus::NotPresent);
    }

    #[test]
    pub fn can_detect_app_config() {
        let proj = analyze();
        assert_eq!(proj.app_config, FileStatus::NotPresent);
    }

    #[test]
    pub fn can_detect_app_settings_json() {
        let proj = analyze();
        assert_eq!(proj.app_settings_json, FileStatus::NotPresent);
    }

    #[test]
    pub fn can_detect_package_json() {
        let proj = analyze();
        assert_eq!(proj.package_json, FileStatus::NotPresent);
    }

    #[test]
    pub fn can_detect_packages_config() {
        let proj = analyze();
        assert_eq!(proj.packages_config, FileStatus::NotPresent);
    }

    #[test]
    pub fn can_detect_project_json() {
        let proj = analyze();
        assert_eq!(proj.project_json, FileStatus::NotPresent);
    }

    #[test]
    pub fn can_detect_output_type() {
        let proj = analyze();
        assert_eq!(proj.output_type, OutputType::Library);
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
    pub fn can_detect_web_config() {
        let proj = analyze();
        assert_eq!(proj.web_config, FileStatus::NotPresent);
    }

    #[test]
    pub fn can_detect_app_config() {
        let proj = analyze();
        assert_eq!(proj.app_config, FileStatus::InProjectFileOnly);
    }

    #[test]
    pub fn can_detect_app_settings_json() {
        let proj = analyze();
        assert_eq!(proj.app_settings_json, FileStatus::NotPresent);
    }

    #[test]
    pub fn can_detect_package_json() {
        let proj = analyze();
        assert_eq!(proj.package_json, FileStatus::NotPresent);
    }

    #[test]
    pub fn can_detect_packages_config() {
        let proj = analyze();
        assert_eq!(proj.packages_config, FileStatus::InProjectFileOnly);
    }

    #[test]
    pub fn can_detect_project_json() {
        let proj = analyze();
        assert_eq!(proj.project_json, FileStatus::NotPresent);
    }

    #[test]
    pub fn can_detect_output_type() {
        let proj = analyze();
        assert_eq!(proj.output_type, OutputType::Library);
    }
}
