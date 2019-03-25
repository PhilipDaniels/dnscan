// #[cfg(test)]
// mod tests {
//     use super::*;
//     use dnlib::file_loader::MemoryFileLoader;

//     #[derive(Default)]
//     struct ProjectBuilder {
//         csproj_contents: String,
//         project_version: ProjectVersion,
//         packages_config_contents: Option<String>,
//         paths_to_analyze: PathsToAnalyze
//     }

//     impl ProjectBuilder {
//         fn new(csproj_contents: &str) -> Self {
//             ProjectBuilder {
//                 csproj_contents: csproj_contents.to_owned(),
//                 .. ProjectBuilder::default()
//             }
//         }

//         fn with_packages_config(mut self, packages_config_contents: &str) -> Self {
//             self.packages_config_contents = Some(packages_config_contents.to_owned());
//             self
//         }

//         fn with_paths(mut self, pta: PathsToAnalyze) -> Self {
//             self.paths_to_analyze = pta;
//             self
//         }

//         fn sdk(mut self) -> Self {
//             self.project_version = ProjectVersion::MicrosoftNetSdk;
//             self
//         }

//         fn old(mut self) -> Self {
//             self.project_version = ProjectVersion::OldStyle;
//             self
//         }

//         fn build(mut self) -> Project {
//             self.csproj_contents = match self.project_version {
//                 ProjectVersion::OldStyle => Self::add_old_prolog(&self.csproj_contents),
//                 ProjectVersion::MicrosoftNetSdk => Self::add_sdk_prolog(&self.csproj_contents),
//                 _ => self.csproj_contents
//             };

//             // Always construct a pta entry for the project itself.
//             let mut file_loader = MemoryFileLoader::new();
//             let project_path = PathBuf::from("/temp/x.csproj");
//             file_loader.files.insert(project_path.clone(), self.csproj_contents);

//             // If there is a packages.config, add a pta entry for it and put the contents into the file loader.
//             if self.packages_config_contents.is_some() {
//                 let pc_path = PathBuf::from("/temp/packages.config");
//                 self.paths_to_analyze.other_files.push(pc_path.clone());
//                 let pcc = self.packages_config_contents.unwrap();
//                 file_loader.files.insert(pc_path, pcc);
//             }

//             Project::new(&project_path, &self.paths_to_analyze, &file_loader)
//         }

//         fn add_sdk_prolog(contents: &str) -> String {
//             format!("{}\n{}", SDK_PROLOG, contents)
//         }

//         fn add_old_prolog(contents: &str) -> String {
//             format!("{}\n{}", OLD_PROLOG, contents)
//         }
//     }


//     #[test]
//     pub fn has_xml_doc_works() {
//         let project = ProjectBuilder::new(r##""##).build();
//         assert_eq!(project.xml_doc, XmlDoc::None);

//         let project = ProjectBuilder::new(r##"blah<DocumentationFile>bin\Debug\WorkflowService.Client.xml</DocumentationFile>blah"##).build();
//         assert_eq!(project.xml_doc, XmlDoc::Debug);

//         let project = ProjectBuilder::new(r##"blah<DocumentationFile>bin\Release\WorkflowService.Client.xml</DocumentationFile>blah"##).build();
//         assert_eq!(project.xml_doc, XmlDoc::Release);

//         let project = ProjectBuilder::new(r##"blah<DocumentationFile>bin\Release\WorkflowService.Client.xml</DocumentationFile>
//             <DocumentationFile>bin\Debug\WorkflowService.Client.xml</DocumentationFile>blah"##).build();
//         assert_eq!(project.xml_doc, XmlDoc::Both);
//     }

//     #[test]
//     pub fn has_tt_file_works() {
//         let project = ProjectBuilder::new(r##""##).build();
//         assert!(!project.tt_file);

//         let project = ProjectBuilder::new(r##"blah<None Update="NuSpecTemplate.tt">blah"##).build();
//         assert!(!project.tt_file);

//         let project = ProjectBuilder::new(r##"blah<None Update="NuSpecTemplate.nuspec">blah"##).build();
//         assert!(!project.tt_file);

//         let project = ProjectBuilder::new(r##"blah<None Update="NuSpecTemplate.nuspec">blah
//             <None Update="NuSpecTemplate.tt">blah"##).build();
//         assert!(project.tt_file);

//         let project = ProjectBuilder::new(r##"blah<None Include="NuSpecTemplate.nuspec">blah
//             <None Include="NuSpecTemplate.tt">blah"##).build();
//         assert!(project.tt_file);
//     }

//     #[test]
//     pub fn has_embedded_debugging_works() {
//         let project = ProjectBuilder::new(r##""##).build();
//         assert!(!project.embedded_debugging);

//         let project = ProjectBuilder::new(r##"blah<DebugType>embedded</DebugType>blah"##).build();
//         assert!(!project.embedded_debugging);

//         let project = ProjectBuilder::new(r##"blah<EmbedAllSources>true</EmbedAllSources>blah"##).build();
//         assert!(!project.embedded_debugging);

//         let project = ProjectBuilder::new(r##"blah<DebugType>embedded</DebugType>blah"
//             <EmbedAllSources>true</EmbedAllSources>blah"##).sdk().build();
//         assert!(project.embedded_debugging);
//     }

//     #[test]
//     pub fn has_linked_solution_info_works() {
//         let project = ProjectBuilder::new(r##""##).build();
//         assert!(!project.linked_solution_info);

//         // SDK style.
//         let project = ProjectBuilder::new(r##"blah<ItemGroup>
//             <Compile Include="..\SolutionInfo.cs" Link="Properties\SolutionInfo.cs" />blah
//             </ItemGroup>blah"##).build();
//         assert!(project.linked_solution_info);

//         // Old style.
//         let project = ProjectBuilder::new(r##"blah<Compile Include="..\SolutionInfo.cs">
//             <Link>Properties\SolutionInfo.cs</Link>blah
//             </Compile>blah"##).build();
//         assert!(project.linked_solution_info);
//     }

//     #[test]
//     pub fn sdk_get_target_frameworks_works() {
//         let project = ProjectBuilder::new(r##""##).build();
//         assert!(project.target_frameworks.is_empty());

//         let project = ProjectBuilder::new(r##"blah<TargetFramework>net462</TargetFramework>blah"##).sdk().build();
//         assert_eq!(project.target_frameworks, vec!["net462"]);

//         // I don't believe this happens, but this is what we get.
//         let project = ProjectBuilder::new(r##"blah<TargetFramework>net462</TargetFramework>blah<TargetFramework>net472</TargetFramework>"##).sdk().build();
//         assert_eq!(project.target_frameworks, vec!["net462", "net472"]);

//         let project = ProjectBuilder::new(r##"blah<TargetFrameworks>net462;net472</TargetFrameworks>blah"##).sdk().build();
//         assert_eq!(project.target_frameworks, vec!["net462", "net472"]);
//     }

//     #[test]
//     pub fn old_get_target_frameworks_works() {
//         let project = ProjectBuilder::new(r##""##).build();
//         assert!(project.target_frameworks.is_empty());

//         let project = ProjectBuilder::new(r##"blah<TargetFrameworkVersion>v4.6.2</TargetFrameworkVersion>blah"##).old().build();
//         assert_eq!(project.target_frameworks, vec!["v4.6.2"]);

//         let project = ProjectBuilder::new(r##"blah<TargetFrameworkVersion>v4.6.2</TargetFrameworkVersion>blah
//             <TargetFrameworkVersion>v4.7.2</TargetFrameworkVersion>"##).old().build();
//         assert_eq!(project.target_frameworks, vec!["v4.6.2", "v4.7.2"]);
//     }

//     #[test]
//     pub fn get_referenced_assemblies_works() {
//         let project = ProjectBuilder::new(r##""##).build();
//         assert!(project.referenced_assemblies.is_empty());

//         let project = ProjectBuilder::new(r##"blah<Reference Include="System.Windows" />blah"##).build();
//         assert_eq!(project.referenced_assemblies, vec!["System.Windows"]);

//         let project = ProjectBuilder::new(r##"blah<Reference Include="System.Windows" />blah
//             blah<Reference Include="System.Windows" />blah"##).build();
//         assert_eq!(project.referenced_assemblies, vec!["System.Windows"]);

//         let project = ProjectBuilder::new(r##"blah<Reference Include="System.Windows" />blah
//             blah<Reference Include="System.Data" />blah"##).build();
//         assert_eq!(project.referenced_assemblies, vec!["System.Data", "System.Windows"]);
//     }

//     #[test]
//     pub fn has_auto_generate_binding_redirects_works() {
//         let project = ProjectBuilder::new(r##""##).build();
//         assert!(!project.auto_generate_binding_redirects);

//         let project = ProjectBuilder::new(r##"blah<AutoGenerateBindingRedirects>true</AutoGenerateBindingRedirects>blah"##).build();
//         assert!(project.auto_generate_binding_redirects);

//         let project = ProjectBuilder::new(r##"blah<AutoGenerateBindingRedirects>false</AutoGenerateBindingRedirects>blah"##).build();
//         assert!(!project.auto_generate_binding_redirects);
//     }

//     #[test]
//     pub fn has_packages_config_not_present() {
//         let project = ProjectBuilder::new(r##""##).build();
//         assert_eq!(project.packages_config, FileStatus::NotPresent);
//     }

//     #[test]
//     pub fn has_packages_config_on_disk() {
//         let project = ProjectBuilder::new(r##""##).with_packages_config("contents").build();
//         assert_eq!(project.packages_config, FileStatus::OnDiskOnly);
//     }

//     #[test]
//     pub fn has_packages_config_in_project_file_only() {
//         let project = ProjectBuilder::new(r##" Include="packages.config" />"##).build();
//         assert_eq!(project.packages_config, FileStatus::InProjectFileOnly);
//     }

//     #[test]
//     pub fn has_packages_config_in_project_file_and_on_disk() {
//         let project = ProjectBuilder::new(r##" Include="packages.config" />"##).with_packages_config("contents").build();
//         assert_eq!(project.packages_config, FileStatus::InProjectFileAndOnDisk);
//     }

//     #[test]
//     pub fn get_packages_sdk_one_line() {
//         let project = ProjectBuilder::new(r##""##).sdk().build();
//         assert!(project.packages.is_empty());

//         let project = ProjectBuilder::new(r##"blah<PackageReference Include="Unity" Version="4.0.1" />blah"##).sdk().build();
//         assert_eq!(project.packages, vec![Package::new("Unity", "4.0.1", false, PackageClass::ThirdParty)]);
//     }

//     #[test]
//     pub fn get_packages_sdk_one_line_sorts() {
//         let project = ProjectBuilder::new(
//             r##"
//             blah<PackageReference Include="Unity" Version="4.0.1" />blah
//             blah<PackageReference Include="Automapper" Version="3.1.4" />blah
//             "##
//             ).sdk().build();
//         assert_eq!(project.packages, vec![
//             Package::new("Automapper", "3.1.4", false, PackageClass::ThirdParty),
//             Package::new("Unity", "4.0.1", false, PackageClass::ThirdParty)
//             ]);

//         // Dedup & sort by secondary key (version).
//         let project = ProjectBuilder::new(
//             r##"
//             blah<PackageReference Include="Automapper" Version="3.1.5" />blah
//             blah<PackageReference Include="Unity" Version="4.0.1" />blah
//             blah<PackageReference Include="Automapper" Version="3.1.4" />blah
//             blah<PackageReference Include="Unity" Version="4.0.1" />blah
//             "##
//             ).sdk().build();
//         assert_eq!(project.packages, vec![
//             Package::new("Automapper", "3.1.4", false, PackageClass::ThirdParty),
//             Package::new("Automapper", "3.1.5", false, PackageClass::ThirdParty),
//             Package::new("Unity", "4.0.1", false, PackageClass::ThirdParty)
//             ]);
//     }

//     #[test]
//     pub fn get_packages_sdk_one_line_dedups() {
//         // Dedup & sort by secondary key (i.e. the version).
//         let project = ProjectBuilder::new(
//             r##"
//             blah<PackageReference Include="Automapper" Version="3.1.5" />blah
//             blah<PackageReference Include="Unity" Version="4.0.1" />blah
//             blah<PackageReference Include="Automapper" Version="3.1.4" />blah
//             blah<PackageReference Include="Unity" Version="4.0.1" />blah
//             "##
//             ).sdk().build();
//         assert_eq!(project.packages, vec![
//             Package::new("Automapper", "3.1.4", false, PackageClass::ThirdParty),
//             Package::new("Automapper", "3.1.5", false, PackageClass::ThirdParty),
//             Package::new("Unity", "4.0.1", false, PackageClass::ThirdParty)
//             ]);
//     }

//     #[test]
//     pub fn get_packages_sdk_multi_line() {
//         let project = ProjectBuilder::new(
//             r##"
//             blah<PackageReference Include="Unity" Version="4.0.1">
//                 </PackageReference>
//             "##
//         ).sdk().build();
//         assert_eq!(project.packages, vec![
//             Package::new("Unity", "4.0.1", false, PackageClass::ThirdParty)
//             ]);
//     }

//     #[test]
//     pub fn get_packages_sdk_multi_line_private_assets() {
//         let project = ProjectBuilder::new(
//             r##"
//             blah<PackageReference Include="Unity" Version="4.0.1">
//                 <PrivateAssets>
//                 </PackageReference>
//             "##
//         ).sdk().build();
//         assert_eq!(project.packages, vec![
//             Package::new("Unity", "4.0.1", true, PackageClass::ThirdParty)
//             ]);
//     }

//     #[test]
//     pub fn get_packages_sdk_multi_line_flip_flop() {
//         // This flip-flop of styles discovered problems in the regex when it
//         // was not terminating early enough.
//         let project = ProjectBuilder::new(
//             r##"
//             blah<PackageReference Include="Unity" Version="4.0.1">
//                 </PackageReference>

//                 <PackageReference Include="EntityFramework" Version="2.4.6" />

//                 <PackageReference Include="Automapper" Version="3.1.4">
//                     <PrivateAssets>
//                 </PackageReference>

//                 <PackageReference Include="Versioning.Bamboo" Version="8.8.9" />
//             "##
//         ).sdk().build();
//         assert_eq!(project.packages, vec![
//             Package::new("Automapper", "3.1.4", true, PackageClass::ThirdParty),
//             Package::new("EntityFramework", "2.4.6", false, PackageClass::Microsoft),
//             Package::new("Unity", "4.0.1", false, PackageClass::ThirdParty),
//             Package::new("Versioning.Bamboo", "8.8.9", false, PackageClass::ThirdParty)
//             ]);
//     }

//     #[test]
//     pub fn get_packages_old_including_sort_and_dedup() {
//         let project = ProjectBuilder::new(r##" Include="packages.config" />"##).old()
//             .with_packages_config(r##"
//             <package id="Clarius.TransformOnBuild" version="1.1.12" targetFramework="net462" developmentDependency="true" />
//             <package id="Castle.Core" version="4.3.1" targetFramework="net462" />
//             <package id="Owin" version="1.0" targetFramework="net462" />
//             <package id="Castle.Core" version="4.3.1" targetFramework="net462" />
//             "##).build();
//         assert_eq!(project.packages, vec![
//             Package::new("Castle.Core", "4.3.1", false, PackageClass::ThirdParty),
//             Package::new("Clarius.TransformOnBuild", "1.1.12", true, PackageClass::ThirdParty),
//             Package::new("Owin", "1.0", false, PackageClass::Microsoft),
//         ]);
//     }

//     #[test]
//     pub fn get_test_framework_mstest() {
//         let project = ProjectBuilder::new(r##"<PackageReference Include="MSTest.TestFramework" Version="4.0.1" />"##)
//             .sdk().build();
//         assert_eq!(project.test_framework, TestFramework::MSTest);
//     }

//     #[test]
//     pub fn get_test_framework_xunit() {
//         let project = ProjectBuilder::new(r##"<PackageReference Include="Xunit.Core" Version="4.0.1" />"##)
//             .sdk().build();
//         assert_eq!(project.test_framework, TestFramework::XUnit);
//     }

//     #[test]
//     pub fn get_test_framework_nunit() {
//         let project = ProjectBuilder::new(r##"<PackageReference Include="NUnit.Core" Version="4.0.1" />"##)
//             .sdk().build();
//         assert_eq!(project.test_framework, TestFramework::NUnit);
//     }

//     #[test]
//     pub fn get_test_framework_none() {
//         let project = ProjectBuilder::new(r##"<PackageReference Include="MSTestNotMatched" Version="4.0.1" />"##)
//             .sdk().build();
//         assert_eq!(project.test_framework, TestFramework::None);
//     }

//     #[test]
//     pub fn uses_specflow_works() {
//         let project = ProjectBuilder::new(r##"<PackageReference Include="NUnit.Core" Version="4.0.1" />"##)
//             .sdk().build();
//         assert!(!project.uses_specflow);

//         let project = ProjectBuilder::new(r##"<PackageReference Include="SpecFlow" Version="2.3.2" />"##)
//             .sdk().build();
//         assert!(project.uses_specflow);
//     }



//     /// These tests run against the embedded example SDK-style project.
//     /// They are an extra sanity-check that we really got it right "in the real world".
//     mod sdk_tests {
//         use super::*;

//         fn get_sdk_project() -> Project {
//             ProjectBuilder::new(include_str!("sdk1.csproj.xml")).sdk().build()
//         }

//         #[test]
//         pub fn can_detect_version() {
//             let project = get_sdk_project();
//             assert_eq!(project.version, ProjectVersion::MicrosoftNetSdk);
//         }

//         #[test]
//         pub fn can_detect_xml_doc() {
//             let project = get_sdk_project();
//             assert_eq!(project.xml_doc, XmlDoc::Both);
//         }

//         #[test]
//         pub fn can_detect_tt_file() {
//             let project = get_sdk_project();
//             assert!(project.tt_file);
//         }

//         #[test]
//         pub fn can_detect_embedded_debugging() {
//             let project = get_sdk_project();
//             assert!(project.embedded_debugging);
//         }

//         #[test]
//         pub fn can_detect_linked_solution_info() {
//             let project = get_sdk_project();
//             assert!(project.linked_solution_info);
//         }

//         #[test]
//         pub fn can_detect_target_framework() {
//             let project = get_sdk_project();
//             assert_eq!(project.target_frameworks, vec!["net462"]);
//         }

//         #[test]
//         pub fn can_detect_referenced_assemblies() {
//             let project = get_sdk_project();
//             assert_eq!(project.referenced_assemblies, vec!["System.Configuration", "System.Windows"]);
//         }

//         #[test]
//         pub fn can_detect_has_auto_generate_binding_redirects() {
//             let project = get_sdk_project();
//             assert!(project.auto_generate_binding_redirects);
//         }

//         #[test]
//         pub fn can_detect_web_config() {
//             let project = get_sdk_project();
//             assert_eq!(project.web_config, FileStatus::NotPresent);
//         }

//         #[test]
//         pub fn can_detect_app_config() {
//             let project = get_sdk_project();
//             assert_eq!(project.app_config, FileStatus::NotPresent);
//         }

//         #[test]
//         pub fn can_detect_app_settings_json() {
//             let project = get_sdk_project();
//             assert_eq!(project.app_settings_json, FileStatus::NotPresent);
//         }

//         #[test]
//         pub fn can_detect_package_json() {
//             let project = get_sdk_project();
//             assert_eq!(project.package_json, FileStatus::NotPresent);
//         }

//         #[test]
//         pub fn can_detect_packages_config() {
//             let project = get_sdk_project();
//             assert_eq!(project.packages_config, FileStatus::NotPresent);
//         }

//         #[test]
//         pub fn can_detect_project_json() {
//             let project = get_sdk_project();
//             assert_eq!(project.project_json, FileStatus::NotPresent);
//         }

//         #[test]
//         pub fn can_detect_output_type() {
//             let project = get_sdk_project();
//             assert_eq!(project.output_type, OutputType::Library);
//         }

//         #[test]
//         pub fn can_detect_packages() {
//             let project = get_sdk_project();
//             assert_eq!(project.packages, vec![
//                 Package::new("Landmark.Versioning.Bamboo", "3.1.44", true, PackageClass::Ours),
//                 Package::new("Unity", "4.0.1", false, PackageClass::ThirdParty),
//             ]);
//         }
//     }



//     /// These tests run against the embedded example old-style project.
//     /// They are an extra sanity-check that we really got it right "in the real world".
//     mod old_style_tests {
//         use super::*;

//         fn get_old_project() -> Project {
//             ProjectBuilder::new(include_str!("old1.csproj.xml")).old().build()
//         }

//         fn get_old_project_with_packages(package_config_contents: &str) -> Project {
//             ProjectBuilder::new(include_str!("old1.csproj.xml")).old()
//                 .with_packages_config(package_config_contents)
//                 .build()
//         }

//         #[test]
//         pub fn can_detect_version() {
//             let project = get_old_project();
//             assert_eq!(project.version, ProjectVersion::OldStyle);
//         }

//         #[test]
//         pub fn can_detect_xml_doc() {
//             let project = get_old_project();
//             assert_eq!(project.xml_doc, XmlDoc::Both);
//         }

//         #[test]
//         pub fn can_detect_tt_file() {
//             let project = get_old_project();
//             assert!(project.tt_file);
//         }

//         #[test]
//         pub fn embedded_debugging_is_always_false() {
//             let project = get_old_project();
//             assert!(!project.embedded_debugging);
//         }

//         #[test]
//         pub fn can_detect_linked_solution_info() {
//             let project = get_old_project();
//             assert!(project.linked_solution_info);
//         }

//         #[test]
//         pub fn can_detect_target_framework() {
//             let project = get_old_project();
//             assert_eq!(project.target_frameworks, vec!["v4.6.2"]);
//         }

//         #[test]
//         pub fn can_detect_referenced_assemblies() {
//             let project = get_old_project();
//             assert_eq!(project.referenced_assemblies, vec![
//                 "PresentationCore",
//                 "PresentationFramework",
//                 "System",
//                 "System.Activities",
//                 "System.Core",
//                 "System.Net.Http",
//                 "System.Xml",
//                 "System.configuration",
//                 "WindowsBase"
//             ]);
//         }

//         #[test]
//         pub fn can_detect_has_auto_generate_binding_redirects() {
//             let project = get_old_project();
//             assert!(!project.auto_generate_binding_redirects);
//         }

//         #[test]
//         pub fn can_detect_web_config() {
//             let project = get_old_project();
//             assert_eq!(project.web_config, FileStatus::NotPresent);
//         }

//         #[test]
//         pub fn can_detect_app_config() {
//             let project = get_old_project();
//             assert_eq!(project.app_config, FileStatus::InProjectFileOnly);
//         }

//         #[test]
//         pub fn can_detect_app_settings_json() {
//             let project = get_old_project();
//             assert_eq!(project.app_settings_json, FileStatus::NotPresent);
//         }

//         #[test]
//         pub fn can_detect_package_json() {
//             let project = get_old_project();
//             assert_eq!(project.package_json, FileStatus::NotPresent);
//         }

//         #[test]
//         pub fn can_detect_packages_config() {
//             let project = get_old_project();
//             assert_eq!(project.packages_config, FileStatus::InProjectFileOnly);
//         }

//         #[test]
//         pub fn can_detect_project_json() {
//             let project = get_old_project();
//             assert_eq!(project.project_json, FileStatus::NotPresent);
//         }

//         #[test]
//         pub fn can_detect_output_type() {
//             let project = get_old_project();
//             assert_eq!(project.output_type, OutputType::Library);
//         }

//         #[test]
//         pub fn can_detect_packages() {
//             let project = get_old_project_with_packages(r##"
//                 <package id="Clarius.TransformOnBuild" version="1.1.12" targetFramework="net462" developmentDependency="true" />
//                 <package id="MyCorp.Fundamentals" version="1.2.18268.136" targetFramework="net462" />
//                 <package id="Microsoft.Owin.Hosting" version="4.0.0" targetFramework="net462" />
//                 <package id="Microsoft.Owin.SelfHost" version="4.0.0" targetFramework="net462" />
//                 <package id="Moq" version="4.8.3" targetFramework="net462" />
//                 <package id="Newtonsoft.Json" version="11.0.2" targetFramework="net462" />
//                 <package id="Npgsql" version="3.2.7" targetFramework="net462" />
//                 <package id="MyProject.Core" version="1.12.18297.228" targetFramework="net462" />
//                 <package id="WorkflowService.Client" version="1.12.18297.23" targetFramework="net462" />
//             "##);

//             assert_eq!(project.packages, vec![
//                 Package::new("Clarius.TransformOnBuild", "1.1.12", true, PackageClass::ThirdParty),
//                 Package::new("Microsoft.Owin.Hosting", "4.0.0", false, PackageClass::Microsoft),
//                 Package::new("Microsoft.Owin.SelfHost", "4.0.0", false, PackageClass::Microsoft),
//                 Package::new("Moq", "4.8.3", false, PackageClass::ThirdParty),
//                 Package::new("MyCorp.Fundamentals", "1.2.18268.136", false, PackageClass::ThirdParty),
//                 Package::new("MyProject.Core", "1.12.18297.228", false, PackageClass::ThirdParty),
//                 Package::new("Newtonsoft.Json", "11.0.2", false, PackageClass::ThirdParty),
//                 Package::new("Npgsql", "3.2.7", false, PackageClass::ThirdParty),
//                 Package::new("WorkflowService.Client", "1.12.18297.23", false, PackageClass::Ours),
//             ]);
//         }
//     }
// }
