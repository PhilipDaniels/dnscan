use std::path::{Path, PathBuf};
use std::sync::Arc;
use lazy_static::lazy_static;
use regex::{Regex, RegexBuilder};
use crate::project_version::ProjectVersion;
use crate::output_type::OutputType;
use crate::file_info::FileInfo;
use crate::file_loader::FileLoader;
use crate::xml_doc::XmlDoc;
use crate::test_framework::TestFramework;
use crate::file_status::FileStatus;
use crate::package::Package;
use crate::package_class::PackageClass;
use crate::interesting_file::InterestingFile;
use crate::as_str::AsStr;
use crate::path_extensions::PathExtensions;

/// The results of analyzing a project file.
#[derive(Debug, Default)]
pub struct Project {
    pub file_info: FileInfo,
    pub other_files: Vec<PathBuf>,
    pub version: ProjectVersion,
    pub output_type: OutputType,
    pub xml_doc: XmlDoc,
    pub tt_file: bool,
    pub target_frameworks: Vec<String>,
    pub embedded_debugging: bool,
    pub linked_solution_info: bool,
    pub auto_generate_binding_redirects: bool,
    pub test_framework: TestFramework,
    pub uses_specflow: bool,
    pub web_config: FileStatus,
    pub app_config: FileStatus,
    pub app_settings_json: FileStatus,
    pub package_json: FileStatus,
    pub packages_config: FileStatus,
    pub project_json: FileStatus,
    pub referenced_assemblies: Vec<String>,
    pub packages: Vec<Package>,
    pub referenced_projects: Vec<Arc<Project>>,
    // packages_require_consolidation
    // redundant_packages_count
    // redundant_projects_count
}

impl Project {
    pub fn new<P>(path: P, other_files: Vec<&PathBuf>, file_loader: &FileLoader) -> Self
        where P: AsRef<Path>
    {
        let mut proj = Project::default();
        for other in other_files {
            proj.other_files.push(other.to_owned());
        }

        proj.file_info = FileInfo::new(path, file_loader);
        if !proj.file_info.is_valid_utf8 {
            return proj;
        }

        proj.version = ProjectVersion::extract(&proj.file_info.contents).unwrap_or_default();
        proj.output_type = OutputType::extract(&proj.file_info.contents);
        proj.xml_doc = XmlDoc::extract(&proj.file_info.contents);
        proj.tt_file = proj.has_tt_file();
        proj.embedded_debugging = proj.has_embedded_debugging();
        proj.linked_solution_info = proj.has_linked_solution_info();
        proj.auto_generate_binding_redirects = proj.has_auto_generate_binding_redirects();
        proj.referenced_assemblies = proj.get_referenced_assemblies();
        proj.target_frameworks = proj.get_target_frameworks();
        proj.web_config = proj.has_file_of_interest(InterestingFile::WebConfig);
        proj.app_config = proj.has_file_of_interest(InterestingFile::AppConfig);
        proj.app_settings_json = proj.has_file_of_interest(InterestingFile::AppSettingsJson);
        proj.package_json = proj.has_file_of_interest(InterestingFile::PackageJson);
        proj.packages_config = proj.has_file_of_interest(InterestingFile::PackagesConfig);
        proj.project_json = proj.has_file_of_interest(InterestingFile::ProjectJson);

        // The things after here are dependent on having first determined the packages
        // that the project uses.
        proj.packages = proj.get_packages(file_loader);
        proj.test_framework = proj.get_test_framework();
        proj.uses_specflow = proj.uses_specflow();

        proj
    }

    fn has_tt_file(&self) -> bool {
        lazy_static! {
            static ref TT_REGEX: Regex = Regex::new(r##"<None (Include|Update).*?\.tt">"##).unwrap();
            static ref NUSPEC_REGEX: Regex = Regex::new(r##"<None (Include|Update).*?\.nuspec">"##).unwrap();
        }

        TT_REGEX.is_match(&self.file_info.contents) && NUSPEC_REGEX.is_match(&self.file_info.contents)
    }

    fn has_embedded_debugging(&self) -> bool {
        lazy_static! {
            // We expect both for it to be correct.
            static ref DEBUG_TYPE_REGEX: Regex = Regex::new(r##"<DebugType>embedded</DebugType>"##).unwrap();
            static ref EMBED_ALL_REGEX: Regex = Regex::new(r##"<EmbedAllSources>true</EmbedAllSources>"##).unwrap();
        }

        match self.version {
            ProjectVersion::MicrosoftNetSdk | ProjectVersion::MicrosoftNetSdkWeb => DEBUG_TYPE_REGEX.is_match(&self.file_info.contents) && EMBED_ALL_REGEX.is_match(&self.file_info.contents),
            ProjectVersion::OldStyle | ProjectVersion::Unknown => false
        }
    }

    fn has_linked_solution_info(&self) -> bool {
        lazy_static! {
            static ref SOLUTION_INFO_REGEX: Regex = Regex::new(r##"[ <]Link.*?SolutionInfo\.cs.*?(</|/>)"##).unwrap();
        }

        SOLUTION_INFO_REGEX.is_match(&self.file_info.contents)
    }

    fn has_auto_generate_binding_redirects(&self) -> bool {
        self.file_info.contents.contains("<AutoGenerateBindingRedirects>true</AutoGenerateBindingRedirects>")
    }

    fn get_referenced_assemblies(&self) -> Vec<String> {
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

    fn get_target_frameworks(&self) -> Vec<String> {
        lazy_static! {
            static ref OLD_TF_REGEX: Regex = Regex::new(r##"<TargetFrameworkVersion>(?P<tf>.*?)</TargetFrameworkVersion>"##).unwrap();
            static ref SDK_SINGLE_TF_REGEX: Regex = Regex::new(r##"<TargetFramework>(?P<tf>.*?)</TargetFramework>"##).unwrap();
            static ref SDK_MULTI_TF_REGEX: Regex = Regex::new(r##"<TargetFrameworks>(?P<tfs>.*?)</TargetFrameworks>"##).unwrap();
        }

        match self.version {
            ProjectVersion::Unknown => vec![],
            ProjectVersion::OldStyle => OLD_TF_REGEX.captures_iter(&self.file_info.contents).map(|cap| cap["tf"].to_owned()).collect(),
            ProjectVersion::MicrosoftNetSdk | ProjectVersion::MicrosoftNetSdkWeb => {
                // One or the other will match.
                let single: Vec<_> = SDK_SINGLE_TF_REGEX.captures_iter(&self.file_info.contents).map(|cap| cap["tf"].to_owned()).collect();
                if !single.is_empty() {
                    return single;
                }

                let mut result = vec![];

                for cap in SDK_MULTI_TF_REGEX.captures_iter(&self.file_info.contents) {
                    let tfs = cap["tfs"].split(";");
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

        match (re.is_match(&self.file_info.contents), self.get_other_file(interesting_file).is_some()) {
            (true, true) => FileStatus::InProjectFileAndOnDisk,
            (true, false) => FileStatus::InProjectFileOnly,
            (false, true) => FileStatus::OnDiskOnly,
            (false, false) => FileStatus::NotPresent
        }
    }

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

    fn get_packages(&self, file_loader: &FileLoader) -> Vec<Package> {
        lazy_static! {
            static ref SDK_RE: Regex = RegexBuilder::new(r##"<PackageReference\s*?Include="(?P<name>.*?)"\s*?Version="(?P<version>.*?)"(?P<inner>.*?)(/>|</PackageReference>)"##)
                                        .case_insensitive(true).dot_matches_new_line(true).build().unwrap();
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
            ProjectVersion::MicrosoftNetSdk | ProjectVersion::MicrosoftNetSdkWeb => {
                SDK_RE.captures_iter(&self.file_info.contents)
                    .map(|cap| Package::new(
                        &cap["name"],
                        &cap["version"],
                        cap["inner"].contains("<PrivateAssets>"),
                        classify(&cap["name"])
                        ))
                    .collect()
            },
            ProjectVersion::OldStyle => {
                // Grab them from the actual packages.config file contents.
                self.get_other_file(InterestingFile::PackagesConfig)
                    .and_then(|pc_path| file_loader.read_to_string(pc_path).ok())
                    .map(|pc_contents|
                        PKG_CONFIG_RE.captures_iter(&pc_contents)
                            .map(|cap| Package::new(
                                &cap["name"],
                                &cap["version"],
                                cap["inner"].contains("developmentDependency=\"true\""),
                                classify(&cap["name"])
                                ))
                            .collect()
                    ).unwrap_or_default()
            },
            ProjectVersion::Unknown => vec![]
        };

        packages.sort();
        packages.dedup();
        packages
    }

    fn get_test_framework(&self) -> TestFramework {
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

    fn uses_specflow(&self) -> bool {
        self.packages.iter()
            .any(|pkg| pkg.name.to_lowercase().contains("specflow"))
    }
}