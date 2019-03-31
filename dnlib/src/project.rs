use crate::as_str::AsStr;
use crate::file_info::FileInfo;
use crate::file_loader::FileLoader;
use crate::file_status::FileStatus;
use crate::interesting_file::InterestingFile;
use crate::output_type::OutputType;
use crate::package::Package;
use crate::package_class::PackageClass;
use crate::path_extensions::PathExtensions;
use crate::project_version::ProjectVersion;
use crate::test_framework::TestFramework;
use crate::xml_doc::XmlDoc;
use lazy_static::lazy_static;
use regex::{Regex, RegexBuilder};
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// The results of analyzing a project file.
#[derive(Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Project {
    pub file_info: FileInfo,
    pub other_files: Vec<PathBuf>,
    pub version: ProjectVersion,
    pub output_type: OutputType,
    pub xml_doc: XmlDoc,
    pub tt_file: bool,
    pub embedded_debugging: bool,
    pub linked_solution_info: bool,
    pub auto_generate_binding_redirects: bool,
    pub referenced_assemblies: Vec<String>,
    pub target_frameworks: Vec<String>,
    pub web_config: FileStatus,
    pub app_config: FileStatus,
    pub app_settings_json: FileStatus,
    pub package_json: FileStatus,
    pub packages_config: FileStatus,
    pub project_json: FileStatus,

    pub packages: Vec<Package>,
    pub test_framework: TestFramework,
    pub uses_specflow: bool,

    // TODO: Filled in later.
    pub referenced_projects: Vec<Arc<Project>>,

    // TODO: packages_require_consolidation, redundant_packages_count, redundant_projects_count
}

impl Project {
    pub fn new<P, L>(path: P, other_files: Vec<PathBuf>, file_loader: &L) -> Self
    where
        P: AsRef<Path>,
        L: FileLoader,
    {
        let mut proj = Project::default();
        proj.other_files = other_files;
        proj.file_info = FileInfo::new(path, file_loader);
        if !proj.file_info.is_valid_utf8 {
            return proj;
        }

        proj.version = ProjectVersion::extract(&proj.file_info.contents).unwrap_or_default();
        proj.output_type = OutputType::extract(&proj.file_info.contents);
        proj.xml_doc = XmlDoc::extract(&proj.file_info.contents);
        proj.tt_file = proj.extract_tt_file();
        proj.embedded_debugging = proj.extract_embedded_debugging();
        proj.linked_solution_info = proj.extract_linked_solution_info();
        proj.auto_generate_binding_redirects = proj.extract_auto_generate_binding_redirects();
        proj.referenced_assemblies = proj.extract_referenced_assemblies();
        proj.target_frameworks = proj.extract_target_frameworks();
        proj.web_config = proj.has_file_of_interest(InterestingFile::WebConfig);
        proj.app_config = proj.has_file_of_interest(InterestingFile::AppConfig);
        proj.app_settings_json = proj.has_file_of_interest(InterestingFile::AppSettingsJson);
        proj.package_json = proj.has_file_of_interest(InterestingFile::PackageJson);
        proj.packages_config = proj.has_file_of_interest(InterestingFile::PackagesConfig);
        proj.project_json = proj.has_file_of_interest(InterestingFile::ProjectJson);

        // The things after here are dependent on having first determined the packages
        // that the project uses.
        proj.packages = proj.extract_packages(file_loader);
        proj.test_framework = proj.extract_test_framework();
        proj.uses_specflow = proj.extract_uses_specflow();

        proj
    }

    fn extract_tt_file(&self) -> bool {
        lazy_static! {
            static ref TT_REGEX: Regex = Regex::new(r##"<None (Include|Update).*?\.tt">"##).unwrap();
            static ref NUSPEC_REGEX: Regex = Regex::new(r##"<None (Include|Update).*?\.nuspec">"##).unwrap();
        }

        TT_REGEX.is_match(&self.file_info.contents) && NUSPEC_REGEX.is_match(&self.file_info.contents)
    }

    fn extract_embedded_debugging(&self) -> bool {
        match self.version {
            // We expect both for it to be correct.
            ProjectVersion::MicrosoftNetSdk | ProjectVersion::MicrosoftNetSdkWeb => self.file_info.contents.contains("<DebugType>embedded</DebugType>") && self.file_info.contents.contains("<EmbedAllSources>true</EmbedAllSources>"),
            ProjectVersion::OldStyle | ProjectVersion::Unknown => false,
        }
    }

    fn extract_linked_solution_info(&self) -> bool {
        lazy_static! {
            static ref SOLUTION_INFO_REGEX: Regex = Regex::new(r##"[ <]Link.*?SolutionInfo\.cs.*?(</|/>)"##).unwrap();
        }

        SOLUTION_INFO_REGEX.is_match(&self.file_info.contents)
    }

    fn extract_auto_generate_binding_redirects(&self) -> bool {
        self.file_info.contents.contains("<AutoGenerateBindingRedirects>true</AutoGenerateBindingRedirects>")
    }

    fn extract_referenced_assemblies(&self) -> Vec<String> {
        // TODO: Necessary to exclude those references that come from NuGet packages?
        // Actually the regex seems good enough, at least for the example files
        // in this project.
        lazy_static! {
            static ref ASM_REF_REGEX: Regex = Regex::new(r##"<Reference Include="(?P<name>.*?)"\s*?/>"##).unwrap();
        }

        let mut result = ASM_REF_REGEX.captures_iter(&self.file_info.contents)
            .map(|cap| cap["name"].to_owned())
            .collect::<Vec<_>>();

        result.sort();
        result.dedup();
        result
    }

    fn extract_target_frameworks(&self) -> Vec<String> {
        lazy_static! {
            static ref OLD_TF_REGEX: Regex = Regex::new(r##"<TargetFrameworkVersion>(?P<tf>.*?)</TargetFrameworkVersion>"##).unwrap();
            static ref SDK_SINGLE_TF_REGEX: Regex = Regex::new(r##"<TargetFramework>(?P<tf>.*?)</TargetFramework>"##).unwrap();
            static ref SDK_MULTI_TF_REGEX: Regex = Regex::new(r##"<TargetFrameworks>(?P<tfs>.*?)</TargetFrameworks>"##).unwrap();
        }

        match self.version {
            ProjectVersion::Unknown => vec![],
            ProjectVersion::OldStyle => OLD_TF_REGEX.captures_iter(&self.file_info.contents)
                .map(|cap| cap["tf"].to_owned())
                .collect(),
            ProjectVersion::MicrosoftNetSdk | ProjectVersion::MicrosoftNetSdkWeb => {
                // One or the other will match.
                let single: Vec<_> = SDK_SINGLE_TF_REGEX.captures_iter(&self.file_info.contents)
                    .map(|cap| cap["tf"].to_owned())
                    .collect();

                if !single.is_empty() {
                    return single;
                }

                let mut result = vec![];

                for cap in SDK_MULTI_TF_REGEX.captures_iter(&self.file_info.contents) {
                    let tfs = cap["tfs"].split(';');
                    for tf in tfs {
                        result.push(tf.to_owned());
                    }
                }

                result
            }
        }
    }

    fn has_file_of_interest(&self, interesting_file: InterestingFile) -> FileStatus {
        // TODO: An optimisation would be to scan for all of these at once rather than separately.
        lazy_static! {
            static ref WEB_CONFIG_RE: Regex = RegexBuilder::new(&format!("\\sInclude=\"{}\"\\s*?/>", InterestingFile::WebConfig.as_str()))
                .case_insensitive(true).build().unwrap();

            static ref APP_CONFIG_RE: Regex = RegexBuilder::new(&format!("\\sInclude=\"{}\"\\s*?/>", InterestingFile::AppConfig.as_str()))
                .case_insensitive(true).build().unwrap();

            static ref APP_SETTINGS_JSON_RE: Regex = RegexBuilder::new(&format!("\\sInclude=\"{}\"\\s*?/>", InterestingFile::AppSettingsJson.as_str()))
                .case_insensitive(true).build().unwrap();

            static ref PACKAGE_JSON_RE: Regex = RegexBuilder::new(&format!("\\sInclude=\"{}\"\\s*?/>", InterestingFile::PackageJson.as_str()))
                .case_insensitive(true).build().unwrap();

            static ref PACKAGES_CONFIG_RE: Regex = RegexBuilder::new(&format!("\\sInclude=\"{}\"\\s*?/>", InterestingFile::PackagesConfig.as_str()))
                .case_insensitive(true).build().unwrap();

            static ref PROJECT_JSON_RE: Regex = RegexBuilder::new(&format!("\\sInclude=\"{}\"\\s*?/>", InterestingFile::ProjectJson.as_str()))
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

        match (re.is_match(&self.file_info.contents), self.get_other_file(interesting_file).is_some()) {
            (true, true) => FileStatus::InProjectFileAndOnDisk,
            (true, false) => FileStatus::InProjectFileOnly,
            (false, true) => FileStatus::OnDiskOnly,
            (false, false) => FileStatus::NotPresent,
        }
    }

    // TODO: Do we still need this?

    /// Checks to see whether a project has another file associated with it
    /// (i.e. that the other file actually exists on disk). This check is based on
    /// the directory of the project and the 'other_files'; we do not use the
    /// XML contents of the project file for this check. We are looking for actual
    /// physical files "in the expected places". This allows us to spot orphaned
    /// files that should have been deleted as part of project migration.
    fn get_other_file(&self, other_file: InterestingFile) -> Option<&PathBuf> {
        for item in &self.other_files {
            if item.filename_as_str().to_lowercase() == other_file.as_str() {
                return Some(item);
            }
        }

        None
    }

    fn extract_packages<L: FileLoader>(&self, file_loader: &L) -> Vec<Package> {
        lazy_static! {
            // It is rather difficult and incomprehensible to do this in a single regex. All these variants have been seen.
            //
            // <PackageReference Include="MoreFluentAssertions" Version="1.2.3" />
            // <PackageReference Include="Microsoft.EntityFrameworkCore">
            //     <Version>2.1.4</Version>
            // </PackageReference>
            // <PackageReference Include="Landmark.Versioning.Bamboo" Version="3.3.19078.47">
            //     <PrivateAssets>all</PrivateAssets>
            //     <IncludeAssets>runtime; build; native; contentfiles; analyzers</IncludeAssets>
            // </PackageReference>
            // <PackageReference Include="FluentAssertions">
            //       <Version>5.6.0</Version>
            // </PackageReference>
            // <PackageReference Include="MoreFluentAssertions" Version="1.2.3" />
            // <PackageReference Include="Landmark.Versioning.Bamboo" Version="3.3.19078.47">
            //     <PrivateAssets>all</PrivateAssets>
            //     <IncludeAssets>runtime; build; native; contentfiles; analyzers</IncludeAssets>
            // </PackageReference>
            // <PackageReference Include="JsonNet.PrivateSettersContractResolvers.Source" Version="0.1.0">
            //     <PrivateAssets>all</PrivateAssets>
            //     <IncludeAssets>runtime; build; native; contentfiles; analyzers</IncludeAssets>
            // </PackageReference>
             //
            // So the idea is to pull out the PackageReference and to its closing tag, getting the package name in the first regex,
            // then to look in the 'rest' to get the version number in a second step.

            static ref SDK_RE: Regex = RegexBuilder::new(r##"<PackageReference\s+Include="(?P<name>[^"]+)"(?P<rest>.+?)(/>|</PackageReference>)"##)
                .case_insensitive(true).dot_matches_new_line(true).build().unwrap();

            static ref SDK_VERSION_RE: Regex = RegexBuilder::new(r##"(Version="(?P<version>[^"]+)"|<Version>(?P<version2>[^<]+)</Version>)"##)
                .case_insensitive(true).build().unwrap();

            static ref PKG_CONFIG_RE: Regex = RegexBuilder::new(r##"<package\s*?id="(?P<name>.*?)"\s*?version="(?P<version>.*?)"(?P<inner>.*?)\s*?/>"##)
                .case_insensitive(true).build().unwrap();


            // This small 3rd party set of matchers essentially allows us to easily recognise a few packages
            // that might otherwise be recognised by the MS or CORP matchers by mistake.
            static ref THIRD_PARTY_PKG_CLASS_RE: Regex = RegexBuilder::new(r##"^System\.IO\.Abstractions.*|^Owin.Metrics"##)
                .case_insensitive(true).build().unwrap();

            static ref OURS_PKG_CLASS_RE: Regex = RegexBuilder::new(r##"^Landmark\..*|^DataMaintenance.*|^ValuationHub\..*|^CaseService\..*|^CaseActivities\..*|^NotificationService\..*|^WorkflowService\..*|^WorkflowRunner\..|^Unity.WF*"##)
                .case_insensitive(true).build().unwrap();

            static ref MS_PKG_CLASS_RE: Regex = RegexBuilder::new(r##"^CommonServiceLocator|^NETStandard\..*|^EntityFramework*|^Microsoft\..*|^MSTest.*|^Owin.*|^System\..*|^EnterpriseLibrary.*"##)
                .case_insensitive(true).build().unwrap();
        }

        let classify = |pkg_name: &str| -> PackageClass {
            if THIRD_PARTY_PKG_CLASS_RE.is_match(pkg_name) {
                PackageClass::ThirdParty
            } else if OURS_PKG_CLASS_RE.is_match(pkg_name) {
                PackageClass::Ours
            } else if MS_PKG_CLASS_RE.is_match(pkg_name) {
                PackageClass::Microsoft
            } else {
                PackageClass::ThirdParty
            }
        };

        let mut packages = match self.version {
            ProjectVersion::MicrosoftNetSdk | ProjectVersion::MicrosoftNetSdkWeb => SDK_RE.captures_iter(&self.file_info.contents)
                .map(|cap| {
                    let pkg_name = &cap["name"];
                    let rest = &cap["rest"];
                    let version_captures = SDK_VERSION_RE.captures(rest).unwrap();
                    let version = version_captures.name("version")
                            .or(version_captures.name("version2"))
                            .unwrap()
                            .as_str();

                    Package::new(
                        pkg_name,
                        version,
                        rest.contains("<PrivateAssets>"),
                        classify(pkg_name),
                    )
                })
                .collect(),
            ProjectVersion::OldStyle => {
                // Grab them from the actual packages.config file contents.
                self.get_other_file(InterestingFile::PackagesConfig)
                    .and_then(|pc_path| file_loader.read_to_string(pc_path).ok())
                    .map(|pc_contents| { PKG_CONFIG_RE.captures_iter(&pc_contents)
                            .map(|cap| {
                                Package::new(
                                    &cap["name"],
                                    &cap["version"],
                                    cap["inner"].contains("developmentDependency=\"true\""),
                                    classify(&cap["name"]),
                                )
                            })
                            .collect()
                    })
                    .unwrap_or_default()
            }
            ProjectVersion::Unknown => vec![],
        };

        packages.sort();
        packages.dedup();
        packages
    }

    fn extract_test_framework(&self) -> TestFramework {
        for pkg in &self.packages {
            let name = pkg.name.to_lowercase();
            if name.starts_with("xunit.") {
                return TestFramework::XUnit;
            } else if name.starts_with("nunit.") {
                return TestFramework::NUnit;
            } else if name.starts_with("mstest.testframework") {
                // I think this is right. There is also MSTest.TestAdapter but
                // that might be for IDE integration, it might not be present.
                return TestFramework::MSTest;
            }
        }

        TestFramework::None
    }

    fn extract_uses_specflow(&self) -> bool {
        self.packages.iter().any(|pkg| pkg.name.to_lowercase().contains("specflow"))
    }
}

 #[cfg(test)]
 mod tests {
    use super::*;
    use crate::file_loader::MemoryFileLoader;
    use crate::project_version::{SDK_PROLOG, OLD_PROLOG, SDK_WEB_PROLOG};

    #[derive(Default)]
    struct ProjectBuilder {
         csproj_contents: String,
         project_version: ProjectVersion,
         packages_config_contents: Option<String>,
         other_files: Vec<PathBuf>
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

        fn with_other_files(mut self, other_files: Vec<PathBuf>) -> Self {
            self.other_files = other_files;
            self
        }

        fn web(mut self) -> Self {
            self.project_version = ProjectVersion::MicrosoftNetSdkWeb;
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
                ProjectVersion::MicrosoftNetSdkWeb => Self::add_web_prolog(&self.csproj_contents),
                ProjectVersion::Unknown => self.csproj_contents
            };

            // Always construct a pta entry for the project itself.
            let mut file_loader = MemoryFileLoader::new();
            let project_path = PathBuf::from("/temp/x.csproj");
            file_loader.files.insert(project_path.clone(), self.csproj_contents);

            // If there is a packages.config, add a pta entry for it and put the contents into the file loader.
            if self.packages_config_contents.is_some() {
                let pc_path = PathBuf::from("/temp/packages.config");
                self.other_files.push(pc_path.clone());
                let pcc = self.packages_config_contents.unwrap();
                file_loader.files.insert(pc_path, pcc);
            }

            Project::new(&project_path, self.other_files, &file_loader)
        }

        fn add_sdk_prolog(contents: &str) -> String {
            format!("{}\n{}", SDK_PROLOG, contents)
        }

        fn add_old_prolog(contents: &str) -> String {
            format!("{}\n{}", OLD_PROLOG, contents)
        }

        fn add_web_prolog(contents: &str) -> String {
            format!("{}\n{}", SDK_WEB_PROLOG, contents)
        }
    }

    #[test]
    pub fn extract_version_works() {
        let project = ProjectBuilder::new(r##""##).build();
        assert_eq!(project.version, ProjectVersion::Unknown);

        let project = ProjectBuilder::new(r##""##).sdk().build();
        assert_eq!(project.version, ProjectVersion::MicrosoftNetSdk);

        let project = ProjectBuilder::new(r##""##).old().build();
        assert_eq!(project.version, ProjectVersion::OldStyle);

        let project = ProjectBuilder::new(r##""##).web().build();
        assert_eq!(project.version, ProjectVersion::MicrosoftNetSdkWeb);
    }

    #[test]
    pub fn extract_output_type_works() {
        let project = ProjectBuilder::new(r##""##).build();
        assert_eq!(project.output_type, OutputType::Library);

        let project = ProjectBuilder::new(r##"<OutputType>Library</OutputType>"##).build();
        assert_eq!(project.output_type, OutputType::Library);

        let project = ProjectBuilder::new(r##"<OutputType>Exe</OutputType>"##).build();
        assert_eq!(project.output_type, OutputType::Exe);

        let project = ProjectBuilder::new(r##"<OutputType>WinExe</OutputType>"##).build();
        assert_eq!(project.output_type, OutputType::WinExe);
    }

    #[test]
    pub fn extract_xml_doc_works() {
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
    pub fn extract_tt_file_works() {
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
    pub fn extract_embedded_debugging_works() {
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
    pub fn extract_linked_solution_info_works() {
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
    pub fn extract_auto_generate_binding_redirects_works() {
        let project = ProjectBuilder::new(r##""##).build();
        assert!(!project.auto_generate_binding_redirects);

        let project = ProjectBuilder::new(r##"blah<AutoGenerateBindingRedirects>true</AutoGenerateBindingRedirects>blah"##).build();
        assert!(project.auto_generate_binding_redirects);

        let project = ProjectBuilder::new(r##"blah<AutoGenerateBindingRedirects>false</AutoGenerateBindingRedirects>blah"##).build();
        assert!(!project.auto_generate_binding_redirects);
    }

    #[test]
    pub fn extract_referenced_assemblies_works() {
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
    pub fn sdk_extract_target_frameworks_works() {
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
    pub fn old_extract_target_frameworks_works() {
        let project = ProjectBuilder::new(r##""##).build();
        assert!(project.target_frameworks.is_empty());

        let project = ProjectBuilder::new(r##"blah<TargetFrameworkVersion>v4.6.2</TargetFrameworkVersion>blah"##).old().build();
        assert_eq!(project.target_frameworks, vec!["v4.6.2"]);

        let project = ProjectBuilder::new(r##"blah<TargetFrameworkVersion>v4.6.2</TargetFrameworkVersion>blah
            <TargetFrameworkVersion>v4.7.2</TargetFrameworkVersion>"##).old().build();
        assert_eq!(project.target_frameworks, vec!["v4.6.2", "v4.7.2"]);
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
    pub fn extract_packages_sdk_one_line() {
        let project = ProjectBuilder::new(r##""##).sdk().build();
        assert!(project.packages.is_empty());

        let project = ProjectBuilder::new(r##"blah<PackageReference Include="Unity" Version="4.0.1" />blah"##).sdk().build();
        assert_eq!(project.packages, vec![Package::new("Unity", "4.0.1", false, PackageClass::ThirdParty)]);
    }

    #[test]
    pub fn extract_packages_sdk_one_line_sorts() {
        let project = ProjectBuilder::new(
            r##"
            blah<PackageReference Include="Unity" Version="4.0.1" />blah
            blah<PackageReference Include="Automapper" Version="3.1.4" />blah
            "##
            ).sdk().build();
        assert_eq!(project.packages, vec![
            Package::new("Automapper", "3.1.4", false, PackageClass::ThirdParty),
            Package::new("Unity", "4.0.1", false, PackageClass::ThirdParty)
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
            Package::new("Automapper", "3.1.4", false, PackageClass::ThirdParty),
            Package::new("Automapper", "3.1.5", false, PackageClass::ThirdParty),
            Package::new("Unity", "4.0.1", false, PackageClass::ThirdParty)
            ]);
    }

    #[test]
    pub fn extract_packages_sdk_one_line_dedups() {
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
            Package::new("Automapper", "3.1.4", false, PackageClass::ThirdParty),
            Package::new("Automapper", "3.1.5", false, PackageClass::ThirdParty),
            Package::new("Unity", "4.0.1", false, PackageClass::ThirdParty)
            ]);
    }

    #[test]
    pub fn extract_packages_sdk_multi_line() {
        let project = ProjectBuilder::new(
            r##"
            blah<PackageReference Include="Unity" Version="4.0.1">
                </PackageReference>
            "##
        ).sdk().build();
        assert_eq!(project.packages, vec![
            Package::new("Unity", "4.0.1", false, PackageClass::ThirdParty)
            ]);
    }

    #[test]
    pub fn extract_packages_sdk_multi_line_private_assets() {
        let project = ProjectBuilder::new(
            r##"
            blah<PackageReference Include="Unity" Version="4.0.1">
                <PrivateAssets>
                </PackageReference>
            "##
        ).sdk().build();
        assert_eq!(project.packages, vec![
            Package::new("Unity", "4.0.1", true, PackageClass::ThirdParty)
            ]);
    }

    #[test]
    pub fn extract_packages_sdk_multi_line_flip_flop() {
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
            Package::new("Automapper", "3.1.4", true, PackageClass::ThirdParty),
            Package::new("EntityFramework", "2.4.6", false, PackageClass::Microsoft),
            Package::new("Unity", "4.0.1", false, PackageClass::ThirdParty),
            Package::new("Versioning.Bamboo", "8.8.9", false, PackageClass::ThirdParty)
            ]);
    }

    #[test]
    pub fn extract_packages_old_including_sort_and_dedup() {
        let project = ProjectBuilder::new(r##" Include="packages.config" />"##).old()
            .with_packages_config(r##"
            <package id="Clarius.TransformOnBuild" version="1.1.12" targetFramework="net462" developmentDependency="true" />
            <package id="Castle.Core" version="4.3.1" targetFramework="net462" />
            <package id="Owin" version="1.0" targetFramework="net462" />
            <package id="Castle.Core" version="4.3.1" targetFramework="net462" />
            "##).build();
        assert_eq!(project.packages, vec![
            Package::new("Castle.Core", "4.3.1", false, PackageClass::ThirdParty),
            Package::new("Clarius.TransformOnBuild", "1.1.12", true, PackageClass::ThirdParty),
            Package::new("Owin", "1.0", false, PackageClass::Microsoft),
        ]);
    }

    #[test]
    pub fn extract_test_framework_mstest() {
        let project = ProjectBuilder::new(r##"<PackageReference Include="MSTest.TestFramework" Version="4.0.1" />"##)
            .sdk().build();
        assert_eq!(project.test_framework, TestFramework::MSTest);
    }

    #[test]
    pub fn extract_test_framework_xunit() {
        let project = ProjectBuilder::new(r##"<PackageReference Include="Xunit.Core" Version="4.0.1" />"##)
            .sdk().build();
        assert_eq!(project.test_framework, TestFramework::XUnit);
    }

    #[test]
    pub fn extract_test_framework_nunit() {
        let project = ProjectBuilder::new(r##"<PackageReference Include="NUnit.Core" Version="4.0.1" />"##)
            .sdk().build();
        assert_eq!(project.test_framework, TestFramework::NUnit);
    }

    #[test]
    pub fn extract_test_framework_none() {
        let project = ProjectBuilder::new(r##"<PackageReference Include="MSTestNotMatched" Version="4.0.1" />"##)
            .sdk().build();
        assert_eq!(project.test_framework, TestFramework::None);
    }

    #[test]
    pub fn extract_uses_specflow_works() {
        let project = ProjectBuilder::new(r##"<PackageReference Include="NUnit.Core" Version="4.0.1" />"##)
            .sdk().build();
        assert!(!project.uses_specflow);

        let project = ProjectBuilder::new(r##"<PackageReference Include="SpecFlow" Version="2.3.2" />"##)
            .sdk().build();
        assert!(project.uses_specflow);
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

        #[test]
        pub fn can_detect_packages() {
            let project = get_sdk_project();
            assert_eq!(project.packages, vec![
                Package::new("Landmark.Versioning.Bamboo", "3.1.44", true, PackageClass::Ours),
                Package::new("Unity", "4.0.1", false, PackageClass::ThirdParty),
            ]);
        }
    }

    /// These tests run against the embedded example old-style project.
    /// They are an extra sanity-check that we really got it right "in the real world".
    mod old_style_tests {
        use super::*;

        fn get_old_project() -> Project {
            ProjectBuilder::new(include_str!("old1.csproj.xml")).old().build()
        }

        fn get_old_project_with_packages(package_config_contents: &str) -> Project {
            ProjectBuilder::new(include_str!("old1.csproj.xml")).old()
                .with_packages_config(package_config_contents)
                .build()
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
            assert_eq!(project.referenced_assemblies, vec![
                "PresentationCore",
                "PresentationFramework",
                "System",
                "System.Activities",
                "System.Core",
                "System.Net.Http",
                "System.Xml",
                "System.configuration",
                "WindowsBase"
            ]);
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

        #[test]
        pub fn can_detect_packages() {
            let project = get_old_project_with_packages(r##"
                <package id="Clarius.TransformOnBuild" version="1.1.12" targetFramework="net462" developmentDependency="true" />
                <package id="MyCorp.Fundamentals" version="1.2.18268.136" targetFramework="net462" />
                <package id="Microsoft.Owin.Hosting" version="4.0.0" targetFramework="net462" />
                <package id="Microsoft.Owin.SelfHost" version="4.0.0" targetFramework="net462" />
                <package id="Moq" version="4.8.3" targetFramework="net462" />
                <package id="Newtonsoft.Json" version="11.0.2" targetFramework="net462" />
                <package id="Npgsql" version="3.2.7" targetFramework="net462" />
                <package id="MyProject.Core" version="1.12.18297.228" targetFramework="net462" />
                <package id="WorkflowService.Client" version="1.12.18297.23" targetFramework="net462" />
            "##);

            assert_eq!(project.packages, vec![
                Package::new("Clarius.TransformOnBuild", "1.1.12", true, PackageClass::ThirdParty),
                Package::new("Microsoft.Owin.Hosting", "4.0.0", false, PackageClass::Microsoft),
                Package::new("Microsoft.Owin.SelfHost", "4.0.0", false, PackageClass::Microsoft),
                Package::new("Moq", "4.8.3", false, PackageClass::ThirdParty),
                Package::new("MyCorp.Fundamentals", "1.2.18268.136", false, PackageClass::ThirdParty),
                Package::new("MyProject.Core", "1.12.18297.228", false, PackageClass::ThirdParty),
                Package::new("Newtonsoft.Json", "11.0.2", false, PackageClass::ThirdParty),
                Package::new("Npgsql", "3.2.7", false, PackageClass::ThirdParty),
                Package::new("WorkflowService.Client", "1.12.18297.23", false, PackageClass::Ours),
            ]);
        }
    }
}
