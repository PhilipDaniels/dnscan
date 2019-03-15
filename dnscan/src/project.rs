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
    Unknown,
    MSTest,
    XUnit,
    NUnit,
}

impl Default for TestFramework {
    fn default() -> Self {
        TestFramework::Unknown
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

    pub xml_doc: XmlDoc,
    pub tt_file: bool,
    pub target_frameworks: Vec<String>,
    pub embedded_debugging: bool,
    pub linked_solution_info: bool,
    pub packages_config: FileStatus,
    pub project_json: FileStatus,
    pub packages: Vec<Package>,
    pub referenced_projects: Vec<Arc<Project>>,
    pub referenced_assemblies: Vec<String>,
    pub auto_generate_binding_redirects: bool,
    pub is_test_project: bool,
    pub test_framework: String,
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

impl Project {
    pub fn new(path: &Path) -> Self {
        let mut proj = Project::default();
        proj.file = path.to_owned();

        match std::fs::read_to_string(path) {
            Ok(s) => {
                proj.is_valid_utf8 = true;
                proj.analyze(s);
            },
            Err(e) => {
                proj.is_valid_utf8 = false;
            }
        }

        proj
    }

    /// Factor the guts of the analysis out into a separate function so that it
    /// can be easily unit tested.
    fn analyze(&mut self, contents: String) {
        self.contents = contents;

        self.version = if self.contents.contains("<Project Sdk=\"Microsoft.NET.Sdk\">") {
             ProjectVersion::MicrosoftNetSdk
        } else if self.contents.contains("<Project ToolsVersion=") {
            ProjectVersion::OldStyle
        } else {
            ProjectVersion::Unknown
        };

        self.xml_doc = Self::has_xml_doc(&self.contents);
        self.tt_file = Self::has_tt_file(&self.contents);
        self.linked_solution_info = Self::has_linked_solution_info(&self.contents);
        self.referenced_assemblies = Self::get_referenced_assemblies(&self.contents);
        self.auto_generate_binding_redirects = Self::has_auto_generate_binding_redirects(&self.contents);
        self.packages_config = Self::has_packages_config(&self.contents, &self.file);

        // pub packages_config: bool,
        // pub project_json: bool,
        // pub packages: Vec<Package>,
        // pub referenced_projects: Vec<Arc<Project>>,
        // pub is_test_project: bool,
        // pub test_framework: String,
        //app.config, web.config, appsettings.json

        if self.version == ProjectVersion::MicrosoftNetSdk {
            self.analyze_sdk_project();
        } else if self.version == ProjectVersion::OldStyle {
            self.analyze_old_style_project();
        }
    }

    fn analyze_sdk_project(&mut self) {
        self.embedded_debugging = Self::has_embedded_debugging(&self.contents);
        self.target_frameworks = Self::sdk_get_target_frameworks(&self.contents);
    }

    fn analyze_old_style_project(&mut self) {
        self.embedded_debugging = false;
        self.target_frameworks = Self::old_get_target_frameworks(&self.contents);
    }

    fn has_xml_doc(contents: &str) -> XmlDoc {
        lazy_static! {
            static ref DEBUG_RE: Regex = Regex::new(r##"<DocumentationFile>bin\\[Dd]ebug\\.*?\.xml</DocumentationFile>"##).unwrap();
            static ref RELEASE_RE: Regex = Regex::new(r##"<DocumentationFile>bin\\[Rr]elease\\.*?\.xml</DocumentationFile>"##).unwrap();
        }

        match (DEBUG_RE.is_match(contents), RELEASE_RE.is_match(contents)) {
            (true, true) => XmlDoc::Both,
            (true, false) => XmlDoc::Debug,
            (false, true) => XmlDoc::Release,
            (false, false) => XmlDoc::None,
        }
    }

    fn has_tt_file(contents: &str) -> bool {
        lazy_static! {
            static ref TT_REGEX: Regex = Regex::new(r##"<None (Include|Update).*?\.tt">"##).unwrap();
            static ref NUSPEC_REGEX: Regex = Regex::new(r##"<None (Include|Update).*?\.nuspec">"##).unwrap();
        }

        TT_REGEX.is_match(contents) && NUSPEC_REGEX.is_match(contents)
    }

    fn has_embedded_debugging(contents: &str) -> bool {
        lazy_static! {
            // We expect both for it to be correct.
            static ref DEBUG_TYPE_REGEX: Regex = Regex::new(r##"<DebugType>embedded</DebugType>"##).unwrap();
            static ref EMBED_ALL_REGEX: Regex = Regex::new(r##"<EmbedAllSources>true</EmbedAllSources>"##).unwrap();
        }

        DEBUG_TYPE_REGEX.is_match(contents) && EMBED_ALL_REGEX.is_match(contents)
    }

    fn has_linked_solution_info(contents: &str) -> bool {
        lazy_static! {
            static ref SOLUTION_INFO_REGEX: Regex = Regex::new(r##"[ <]Link.*?SolutionInfo\.cs.*?(</|/>)"##).unwrap();
        }

        SOLUTION_INFO_REGEX.is_match(contents)
    }

    fn sdk_get_target_frameworks(contents: &str) -> Vec<String> {
        lazy_static! {
            static ref SINGLE_TF_REGEX: Regex = Regex::new(r##"<TargetFramework>(?P<tf>.*?)</TargetFramework>"##).unwrap();
            static ref MULTI_TF_REGEX: Regex = Regex::new(r##"<TargetFrameworks>(?P<tfs>.*?)</TargetFrameworks>"##).unwrap();
        }

        // One or the other will match.
        let single: Vec<_> = SINGLE_TF_REGEX.captures_iter(contents).map(|cap| cap["tf"].to_owned()).collect();
        if !single.is_empty() {
            return single;
        }

        let mut result = vec![];

        for cap in MULTI_TF_REGEX.captures_iter(contents) {
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

    fn old_get_target_frameworks(contents: &str) -> Vec<String> {
        lazy_static! {
            static ref TF_REGEX: Regex = Regex::new(r##"<TargetFrameworkVersion>(?P<tf>.*?)</TargetFrameworkVersion>"##).unwrap();
        }

        TF_REGEX.captures_iter(contents).map(|cap| cap["tf"].to_owned()).collect()
    }

    fn get_referenced_assemblies(contents: &str) -> Vec<String> {
        // TODO: Necessary to exclude those references that come from NuGet packages?
        // Actually the regex seems good enough, at least for the example files
        // in this project.
        lazy_static! {
            static ref ASM_REF_REGEX: Regex = Regex::new(r##"<Reference Include="(?P<name>.*?)"\s*?/>"##).unwrap();
        }

        let mut result = ASM_REF_REGEX.captures_iter(contents)
            .map(|cap| cap["name"].to_owned())
            .collect::<Vec<_>>();

        result.sort();
        result.dedup();
        result
    }

    fn has_auto_generate_binding_redirects(contents: &str) -> bool {
        contents.contains("<AutoGenerateBindingRedirects>true</AutoGenerateBindingRedirects>")
    }

    fn has_packages_config(contents: &str, proj_file_path: &Path) -> FileStatus {
        lazy_static! {
            static ref PKG_CONFIG_RE: Regex = Regex::new(r##"\sInclude="[Pp]ackages.[Cc]onfig"\s*?/>"##).unwrap();
        }

        match (PKG_CONFIG_RE.is_match(contents), 1 == 1) {
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

    #[test]
    pub fn has_xml_doc() {
        let result = Project::has_xml_doc(r##""##);
        assert_eq!(result, XmlDoc::None);

        let result = Project::has_xml_doc(r##"blah<DocumentationFile>bin\Debug\WorkflowService.Client.xml</DocumentationFile>blah"##);
        assert_eq!(result, XmlDoc::Debug);

        let result = Project::has_xml_doc(r##"blah<DocumentationFile>bin\Release\WorkflowService.Client.xml</DocumentationFile>blah"##);
        assert_eq!(result, XmlDoc::Release);

        let result = Project::has_xml_doc(r##"blah<DocumentationFile>bin\Release\WorkflowService.Client.xml</DocumentationFile>
            <DocumentationFile>bin\Debug\WorkflowService.Client.xml</DocumentationFile>blah"##);
        assert_eq!(result, XmlDoc::Both);
    }

    #[test]
    pub fn has_tt_file() {
        let result = Project::has_tt_file(r##""##);
        assert_eq!(result, false);

        let result = Project::has_tt_file(r##"blah<None Update="NuSpecTemplate.tt">blah"##);
        assert_eq!(result, false);

        let result = Project::has_tt_file(r##"blah<None Update="NuSpecTemplate.nuspec">blah"##);
        assert_eq!(result, false);

        let result = Project::has_tt_file(r##"blah<None Update="NuSpecTemplate.nuspec">blah
            <None Update="NuSpecTemplate.tt">blah"##);
        assert_eq!(result, true);

        let result = Project::has_tt_file(r##"blah<None Include="NuSpecTemplate.nuspec">blah
            <None Include="NuSpecTemplate.tt">blah"##);
        assert_eq!(result, true);
    }

    #[test]
    pub fn has_embedded_debugging() {
        let result = Project::has_embedded_debugging(r##""##);
        assert_eq!(result, false);

        let result = Project::has_embedded_debugging(r##"blah<DebugType>embedded</DebugType>blah"##);
        assert_eq!(result, false);

        let result = Project::has_embedded_debugging(r##"blah<EmbedAllSources>true</EmbedAllSources>blah"##);
        assert_eq!(result, false);

        let result = Project::has_embedded_debugging(r##"blah<DebugType>embedded</DebugType>blah"
            <EmbedAllSources>true</EmbedAllSources>blah"##);
        assert_eq!(result, true);
    }

    #[test]
    pub fn has_linked_solution_info() {
        let result = Project::has_linked_solution_info(r##""##);
        assert_eq!(result, false);

        // SDK style.
        let result = Project::has_linked_solution_info(r##"blah<ItemGroup>
            <Compile Include="..\SolutionInfo.cs" Link="Properties\SolutionInfo.cs" />blah
            </ItemGroup>blah"##);
        assert_eq!(result, true);

        // Old style.
        let result = Project::has_linked_solution_info(r##"blah<Compile Include="..\SolutionInfo.cs">
            <Link>Properties\SolutionInfo.cs</Link>blah
            </Compile>blah"##);
        assert_eq!(result, true);
    }

    #[test]
    pub fn can_get_sdk_target_frameworks() {
        let result = Project::sdk_get_target_frameworks(r##""##);
        assert!(result.is_empty());

        let result = Project::sdk_get_target_frameworks(r##"blah<TargetFramework>net462</TargetFramework>blah"##);
        assert_eq!(result, vec!["net462"]);

        // I don't believe this happens, but this is what we get.
        let result = Project::sdk_get_target_frameworks(r##"blah<TargetFramework>net462</TargetFramework>blah<TargetFramework>net472</TargetFramework>"##);
        assert_eq!(result, vec!["net462", "net472"]);

        let result = Project::sdk_get_target_frameworks(r##"blah<TargetFrameworks>net462;net472</TargetFrameworks>blah"##);
        assert_eq!(result, vec!["net462", "net472"]);
    }

    #[test]
    pub fn can_get_old_target_frameworks() {
        let result = Project::old_get_target_frameworks(r##""##);
        assert!(result.is_empty());

        let result = Project::old_get_target_frameworks(r##"blah<TargetFrameworkVersion>v4.6.2</TargetFrameworkVersion>blah"##);
        assert_eq!(result, vec!["v4.6.2"]);

        let result = Project::old_get_target_frameworks(r##"blah<TargetFrameworkVersion>v4.6.2</TargetFrameworkVersion>blah
            <TargetFrameworkVersion>v4.7.2</TargetFrameworkVersion>"##);
        assert_eq!(result, vec!["v4.6.2", "v4.7.2"]);
    }

    #[test]
    pub fn can_get_referenced_assemblies() {
        let result = Project::get_referenced_assemblies(r##""##);
        assert!(result.is_empty());

        let result = Project::get_referenced_assemblies(r##"blah<Reference Include="System.Windows" />blah"##);
        assert_eq!(result, vec!["System.Windows"]);

        let result = Project::get_referenced_assemblies(r##"blah<Reference Include="System.Windows" />blah
        blah<Reference Include="System.Windows" />blah"##);
        assert_eq!(result, vec!["System.Windows"]);

        let result = Project::get_referenced_assemblies(r##"blah<Reference Include="System.Windows" />blah
        blah<Reference Include="System.Data" />blah"##);
        assert_eq!(result, vec!["System.Data", "System.Windows"]);
    }

    #[test]
    pub fn can_get_has_auto_generate_binding_redirects() {
        let result = Project::has_auto_generate_binding_redirects(r##""##);
        assert!(result == false);

        let result = Project::has_auto_generate_binding_redirects(r##"blah<AutoGenerateBindingRedirects>true</AutoGenerateBindingRedirects>blah"##);
        assert!(result == true);

        let result = Project::has_auto_generate_binding_redirects(r##"blah<AutoGenerateBindingRedirects>false</AutoGenerateBindingRedirects>blah"##);
        assert!(result == false);
    }
}

#[cfg(test)]
mod sdk_tests {
    use super::*;

    fn sdk_csproj() -> String {
        include_str!("sdk1.csproj.xml").to_owned()
    }

    fn analyze() -> Project {
        let mut proj = Project::default();
        proj.analyze(sdk_csproj());
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
}

#[cfg(test)]
mod old_style_tests {
    use super::*;

    fn old_style_csproj() -> String {
        include_str!("old1.csproj.xml").to_owned()
    }

    fn analyze() -> Project {
        let mut proj = Project::default();
        proj.analyze(old_style_csproj());
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
}
