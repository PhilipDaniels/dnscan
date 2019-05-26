use crate::errors::DnLibResult;
use crate::git_info::GitInfo;
use crate::enums::*;
use crate::io::{PathExtensions, PathsToAnalyze, DiskFileLoader, find_files, FileLoader};
use crate::configuration::Configuration;
use crate::{timer, finish};

use lazy_static::lazy_static;
use regex::{Regex, RegexBuilder};
use rayon::prelude::*;
use log::warn;
use std::path::{Path, PathBuf};
use std::ffi::OsStr;
use std::cmp::Ordering;
use std::hash::{Hash, Hasher};
use std::fmt;

/// The set of all files found during analysis.
#[derive(Debug, Default)]
pub struct Analysis {
    pub root_path: PathBuf,
    pub paths_analyzed: PathsToAnalyze,
    pub solution_directories: Vec<SolutionDirectory>,
}

impl PartialEq for Analysis {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.root_path == other.root_path
    }
}

impl Hash for Analysis {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.root_path.hash(state)
    }
}

impl Eq for Analysis { }

impl Analysis {
    pub fn new(configuration: &Configuration) -> DnLibResult<Self>
    {
        let pta = find_files(&configuration.input_directory)?;

        let mut af = Self {
            root_path: configuration.input_directory.clone(),
            paths_analyzed: pta,
            ..Default::default()
        };

        let fs_loader = DiskFileLoader::default();
        af.analyze(configuration, fs_loader)?;

        Ok(af)
    }

    pub fn sort(&mut self) {
        self.solution_directories.sort();
        for sd in &mut self.solution_directories {
            sd.sort();
        }
    }

    pub fn is_empty(&self) -> bool {
        self.solution_directories.is_empty()
    }

    pub fn num_solutions(&self) -> usize {
        self.solution_directories.iter()
            .map(|sln_dir| sln_dir.num_solutions())
            .sum()
    }

    pub fn num_linked_projects(&self) -> usize {
        self.solution_directories.iter()
            .map(|sln_dir| sln_dir.num_linked_projects())
            .sum()
    }

    pub fn num_orphaned_projects(&self) -> usize {
        self.solution_directories.iter()
            .map(|sln_dir| sln_dir.num_orphaned_projects())
            .sum()
    }

    /// The actual guts of `new`, using a file loader so we can test it.
    fn analyze<L>(&mut self, configuration: &Configuration, file_loader: L) -> DnLibResult<()>
    where L: FileLoader + std::marker::Sync
    {
        // Load and analyze each solution and place them into folders.
        let tmr = timer!("Load And Analyze Solution files");
        let solutions = self.paths_analyzed.sln_files.par_iter()
            .map(|sln_path| {
                Solution::new(sln_path, &file_loader.clone())
            }).collect::<Vec<_>>();

        for sln in solutions {
            self.add_solution(sln);
        }
        drop(tmr);


        // For each project, grab all the 'other' files in the same directory.
        // (This is very hacky. Assumes they are all in the project directory! Can fix by replacing
        // the '==' with a closure). Then analyze the project itself.
        let tmr = timer!("Load And Analyze Project files");
        let projects = self.paths_analyzed.csproj_files.par_iter()
            .map(|proj_path| {
                let other_paths = self.paths_analyzed.other_files.iter()
                    .filter(|&other_path| other_path.is_same_dir(proj_path))
                    .cloned()
                    .collect::<Vec<_>>();

                Project::new(proj_path, other_paths, &file_loader.clone(), configuration)
            })
            .collect::<Vec<_>>();

        for proj in projects {
            self.add_project(proj);
        }

        finish!(tmr, "Found {} linked projects and {} orphaned projects",
            self.num_linked_projects(),
            self.num_orphaned_projects()
            );

        self.sort();
        Ok(())
    }

    fn add_solution(&mut self, sln: Solution)
    {
        let sln_dir = sln.file_info.path.parent().unwrap();

        for item in &mut self.solution_directories {
            if item.directory == sln_dir {
                item.solutions.push(sln);
                return;
            }
        }

        let mut sd = SolutionDirectory::new(sln_dir);
        sd.get_git_info(&self.root_path);
        sd.solutions.push(sln);
        self.solution_directories.push(sd);
    }

    fn add_project(&mut self, mut project: Project) {
        if let Some((sln, ownership)) = self.get_solution_that_owns_project(&project.file_info.path) {
            project.ownership = ownership;
            sln.projects.push(project);
        } else {
            warn!("Could not associate project {:?} with a solution, ignoring.", &project.file_info.path);
        }
    }

    fn get_solution_that_owns_project<P>(&mut self, project_path: P) -> Option<(&mut Solution, ProjectOwnership)>
    where
        P: AsRef<Path>,
    {
        let project_path = project_path.as_ref();
        let parent_dir = project_path.parent().expect("Should always be able to get the parent dir of a project.");

        let mut handles = None;

        'outer: for ownership_type in &[ProjectOwnership::Linked, ProjectOwnership::Orphaned] {
            for (dir_idx, sln_dir) in self.solution_directories.iter_mut().enumerate() {
                for (sln_idx, sln) in sln_dir.solutions.iter_mut().enumerate() {

                    match ownership_type {
                        ProjectOwnership::Linked => if sln.refers_to_project(project_path) {
                            handles = Some((dir_idx, sln_idx, ownership_type));
                            break 'outer;
                        },
                        ProjectOwnership::Orphaned => if sln.file_info.path.is_same_dir(project_path) ||
                                                        sln.file_info.path.is_same_dir(parent_dir)
                        {
                            handles = Some((dir_idx, sln_idx, ownership_type));
                            break 'outer;
                        },
                        ProjectOwnership::Unknown => unreachable!("There are only 2 ownership types to check.")
                    }
                }
            }
        };

        if let Some((dir_idx, sln_idx, ownership_type)) = handles {
            Some((&mut self.solution_directories[dir_idx].solutions[sln_idx], *ownership_type))
        } else {
            None
        }
    }
}


#[derive(Debug, Default, Eq)]
/// Represents a directory that contains 1 or more solution files.
pub struct SolutionDirectory {
    /// The directory path, e.g. `C:\temp\my_solution`.
    pub directory: PathBuf,

    /// The sln files in this directory.
    pub solutions: Vec<Solution>,

    /// Info about the Git repo, if any.
    pub git_info: Option<GitInfo>,
}

impl PartialEq for SolutionDirectory {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.directory == other.directory
    }
}

impl Hash for SolutionDirectory {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.directory.hash(state)
    }
}

impl PartialOrd for SolutionDirectory {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.directory.cmp(&other.directory))
    }
}

impl Ord for SolutionDirectory {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        self.directory.cmp(&other.directory)
    }
}



impl SolutionDirectory {
    fn new<P: Into<PathBuf>>(sln_directory: P) -> Self {
        SolutionDirectory {
            directory: sln_directory.into(),
            ..Default::default()
        }
    }

    pub fn sort(&mut self) {
        self.solutions.sort();
        for sf in &mut self.solutions {
            sf.sort();
        }
    }

    pub fn num_solutions(&self) -> usize {
        self.solutions.len()
    }

    pub fn num_linked_projects(&self) -> usize {
        self.solutions.iter()
            .map(|sln| sln.linked_projects().count())
            .sum()
    }

    pub fn num_orphaned_projects(&self) -> usize {
        self.solutions.iter()
            .map(|sln| sln.orphaned_projects().count())
            .sum()
    }

    fn get_git_info<C>(&mut self, ceiling_dir: C)
    where C: AsRef<OsStr>
    {
        self.git_info = GitInfo::new(&self.directory, ceiling_dir).ok();
    }
}

#[derive(Debug, Default)]
/// Represents a sln file and any projects that are associated with it.
pub struct Solution {
    pub file_info: FileInfo,
    pub version: VisualStudioVersion,
    pub git_info: GitInfo,

    // The set of projects that we found during the disk walk and have loaded and
    // associated with this solution (either by explicit linkage because they are
    // mentioned in the .sln file, or by assumed-orphanship because they are in
    // the same directory, but no longer in the solution).
    pub projects: Vec<Project>,

    /// The set of projects that is mentioned inside the sln file.
    /// This is populated by reading the solution file and normalizing
    /// the extracted paths.
    mentioned_projects: Vec<PathBuf>
}

impl PartialEq for Solution {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.file_info == other.file_info
    }
}

impl Eq for Solution {}

impl Hash for Solution {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.file_info.hash(state)
    }
}

impl PartialOrd for Solution {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.file_info.cmp(&other.file_info))
    }
}

impl Ord for Solution {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        self.file_info.cmp(&other.file_info)
    }
}


/// Convert this extracted path to a form that matches what is in use on
/// the operating system the program is running on. Mentioned paths are
/// always of the form "Dir\Foo.csproj" (in other words, even on Linux
/// they use Windows-style slashes)
#[cfg(windows)]
fn norm_mentioned_path(mp: &str) -> String {
    mp.to_owned()
}

#[cfg(not(windows))]
fn norm_mentioned_path(mp: &str) -> String {
    mp.replace('\\', "/").to_owned()
}

// From https://github.com/rust-lang/cargo/blob/2e4cfc2b7d43328b207879228a2ca7d427d188bb/src/cargo/util/paths.rs#L65-L90
fn normalize_path(path: &Path) -> PathBuf {
    use std::path::Component;

    let mut components = path.components().peekable();
    let mut ret = if let Some(c @ Component::Prefix(..)) = components.peek().cloned() {
        components.next();
        PathBuf::from(c.as_os_str())
    } else {
        PathBuf::new()
    };

    for component in components {
        match component {
            Component::Prefix(..) => unreachable!(),
            Component::RootDir => {
                ret.push(component.as_os_str());
            }
            Component::CurDir => {}
            Component::ParentDir => {
                ret.pop();
            }
            Component::Normal(c) => {
                ret.push(c);
            }
        }
    }
    ret
}

impl Solution {
    pub fn new<P, L>(path: P, file_loader: &L) -> Self
    where
        P: AsRef<Path>,
        L: FileLoader,
    {
        let fi = FileInfo::new(path.as_ref(), file_loader);
        let ver = VisualStudioVersion::extract(&fi.contents).unwrap_or_default();
        let sln_dir = fi.path.parent().unwrap().to_owned();
        let mp = Self::extract_mentioned_projects(sln_dir, &fi.contents);

        Solution {
            file_info: fi,
            version: ver,
            mentioned_projects: mp,
            ..Default::default()
        }
    }

    fn sort(&mut self) {
        self.projects.sort();
    }

    pub fn linked_projects(&self) -> impl Iterator<Item = &Project> {
        self.projects.iter().filter(|p| p.ownership == ProjectOwnership::Linked)
    }

    pub fn orphaned_projects(&self) -> impl Iterator<Item = &Project> {
        self.projects.iter().filter(|p| p.ownership == ProjectOwnership::Orphaned)
    }

    /// Extracts the projects from the contents of the solution file. Note that there is
    /// a potential problem here, in that the paths constructed will be in the format
    /// of the system that the solution was created on (e.g. Windows) and not the
    /// format of the system the program is running on (e.g. Linux).
    /// See also `refers_to_project` where this surfaces.
    fn extract_mentioned_projects(sln_dir: PathBuf, contents: &str) -> Vec<PathBuf> {
        lazy_static! {
            static ref PROJECT_RE: Regex = RegexBuilder::new(r#""(?P<projpath>[^"]+csproj)"#)
                .case_insensitive(true).build().unwrap();
        }

        let mut project_paths = PROJECT_RE.captures_iter(contents)
            .map(|cap| {
                let mut path = sln_dir.clone();
                let x = norm_mentioned_path(&cap["projpath"]);
                path.push(x);
                path
            })
            .collect::<Vec<_>>();

        project_paths.sort();
        project_paths.dedup();
        project_paths
    }

    fn refers_to_project<P: AsRef<Path>>(&self, project_path: P) -> bool {
        let project_path = project_path.as_ref();
        self.mentioned_projects.iter().any(|mp| mp.eq_ignoring_case(project_path))
    }
}

#[derive(Debug, Default, Clone, Eq)]
/// Represents information about a .sln or .csproj file.
pub struct FileInfo {
    pub path: PathBuf,
    pub contents: String,
    pub is_valid_utf8: bool,
}

impl FileInfo {
    pub fn new<P, L>(path: P, file_loader: &L) -> Self
        where P: Into<PathBuf>,
              L: FileLoader
    {
        let mut fi = FileInfo::default();
        fi.path = path.into();
        let file_contents_result = file_loader.read_to_string(&fi.path);
        fi.is_valid_utf8 = file_contents_result.is_ok();
        fi.contents = file_contents_result.unwrap_or_default();
        fi
    }

    /// Returns the whole path as a str, or "" if it cannot be converted.
    pub fn path_as_str(&self) -> &str {
        self.path.as_str()
    }

    /// Returns the final filename component as a str, or "" if it cannot be converted.
    pub fn filename_as_str(&self) -> &str {
        self.path.filename_as_str()
    }

    /// Returns the directory component as a str, or "" if it cannot be converted.
    pub fn directory_as_str(&self) -> &str {
        self.path.directory_as_str()
    }
}

impl PartialEq for FileInfo {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.path == other.path
    }
}

impl Hash for FileInfo {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.path.hash(state)
    }
}

impl PartialOrd for FileInfo {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for FileInfo {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        self.path.cmp(&other.path)
    }
}




/// The results of analyzing a project file.
#[derive(Default)]
pub struct Project {
    pub file_info: FileInfo,
    pub ownership: ProjectOwnership,
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

    // This is a collection of the normalized 'foo.csproj' paths as extracted from this csproj file.
    // We call these 'child projects'.
    child_project_paths: Vec<PathBuf>,
}


impl fmt::Debug for Project {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "{}", self.file_info.path.filename_as_str())
    }
}

impl PartialEq for Project {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.file_info == other.file_info
    }
}

impl Eq for Project { }

impl Hash for Project {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.file_info.hash(state)
    }
}

impl PartialOrd for Project {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.file_info.cmp(&other.file_info))
    }
}

impl Ord for Project {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        self.file_info.cmp(&other.file_info)
    }
}


impl Project {
    pub fn new<P, L>(path: P, other_files: Vec<PathBuf>, file_loader: &L, configuration: &Configuration) -> Self
    where
        P: AsRef<Path>,
        L: FileLoader,
    {
        let mut proj = Project::default();
        proj.other_files = other_files;
        proj.file_info = FileInfo::new(path.as_ref(), file_loader);
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
        proj.child_project_paths = proj.extract_project_paths();

        // The things after here are dependent on having first determined the packages
        // that the project uses.
        proj.packages = proj.extract_packages(file_loader, configuration);
        proj.test_framework = proj.extract_test_framework();
        proj.uses_specflow = proj.extract_uses_specflow();

        proj
    }

    /// Finds all the projects in the solution that this project references.
    /// I.e. finds all the 'children' of this project.
    pub fn get_child_projects<'s>(&self, sln: &'s Solution) -> Vec<&'s Project> {
        sln.projects
            .iter()
            .filter(|potential_child| self.refers_to(potential_child))
            .collect()
    }

    /// Finds all the projects in the solution that refer to this project.
    /// I.e. finds all the 'parents' of this project.
    pub fn get_parent_projects<'s>(&self, sln: &'s Solution) -> Vec<&'s Project> {
        sln.projects
            .iter()
            .filter(|potential_parent| potential_parent.refers_to(self))
            .collect()
    }

    fn refers_to(&self, other: &Self) -> bool {
        self.child_project_paths
            .iter()
            .find(|our_child_path| **our_child_path == other.file_info.path).is_some()
    }

    fn extract_tt_file(&self) -> bool {
        lazy_static! {
            static ref TT_REGEX: Regex = Regex::new(r#"<None (Include|Update).*?\.tt">"#).unwrap();
            static ref NUSPEC_REGEX: Regex = Regex::new(r#"<None (Include|Update).*?\.nuspec">"#).unwrap();
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
            static ref SOLUTION_INFO_REGEX: Regex = Regex::new(r#"[ <]Link.*?SolutionInfo\.cs.*?(</|/>)"#).unwrap();
        }

        SOLUTION_INFO_REGEX.is_match(&self.file_info.contents)
    }

    fn extract_auto_generate_binding_redirects(&self) -> bool {
        self.file_info.contents.contains("<AutoGenerateBindingRedirects>true</AutoGenerateBindingRedirects>")
    }

    fn extract_referenced_assemblies(&self) -> Vec<String> {
        // Necessary to exclude those references that come from NuGet packages?
        // Actually the regex seems good enough, at least for the example files
        // in this project.
        lazy_static! {
            static ref ASM_REF_REGEX: Regex = Regex::new(r#"<Reference Include="(?P<name>.*?)"\s*?/>"#).unwrap();
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
            static ref OLD_TF_REGEX: Regex = Regex::new(r#"<TargetFrameworkVersion>(?P<tf>.*?)</TargetFrameworkVersion>"#).unwrap();
            static ref SDK_SINGLE_TF_REGEX: Regex = Regex::new(r#"<TargetFramework>(?P<tf>.*?)</TargetFramework>"#).unwrap();
            static ref SDK_MULTI_TF_REGEX: Regex = Regex::new(r#"<TargetFrameworks>(?P<tfs>.*?)</TargetFrameworks>"#).unwrap();
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
            static ref WEB_CONFIG_RE: Regex = RegexBuilder::new(&format!("\\sInclude=\"{}\"\\s*?/>", InterestingFile::WebConfig))
                .case_insensitive(true).build().unwrap();

            static ref APP_CONFIG_RE: Regex = RegexBuilder::new(&format!("\\sInclude=\"{}\"\\s*?/>", InterestingFile::AppConfig))
                .case_insensitive(true).build().unwrap();

            static ref APP_SETTINGS_JSON_RE: Regex = RegexBuilder::new(&format!("\\sInclude=\"{}\"\\s*?/>", InterestingFile::AppSettingsJson))
                .case_insensitive(true).build().unwrap();

            static ref PACKAGE_JSON_RE: Regex = RegexBuilder::new(&format!("\\sInclude=\"{}\"\\s*?/>", InterestingFile::PackageJson))
                .case_insensitive(true).build().unwrap();

            static ref PACKAGES_CONFIG_RE: Regex = RegexBuilder::new(&format!("\\sInclude=\"{}\"\\s*?/>", InterestingFile::PackagesConfig))
                .case_insensitive(true).build().unwrap();

            static ref PROJECT_JSON_RE: Regex = RegexBuilder::new(&format!("\\sInclude=\"{}\"\\s*?/>", InterestingFile::ProjectJson))
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

        match (re.is_match(&self.file_info.contents), self.find_other_file(interesting_file).is_some()) {
            (true, true) => FileStatus::InProjectFileAndOnDisk,
            (true, false) => FileStatus::InProjectFileOnly,
            (false, true) => FileStatus::OnDiskOnly,
            (false, false) => FileStatus::NotPresent,
        }
    }

    /// Checks to see whether a project has another file associated with it
    /// (i.e. that the other file actually exists on disk). This check is based on
    /// the directory of the project and the 'other_files'; we do not use the
    /// XML contents of the project file for this check. We are looking for actual
    /// physical files "in the expected places". This allows us to spot orphaned
    /// files that should have been deleted as part of project migration.
    fn find_other_file(&self, other_file: InterestingFile) -> Option<&PathBuf> {
        self.other_files.iter()
            .find(|item| unicase::eq(item.filename_as_str(), other_file.as_ref()))
    }

    fn extract_project_paths(&self) -> Vec<PathBuf> {
        lazy_static! {
            static ref PROJECT_REF_REGEX: Regex = RegexBuilder::new(r#"<ProjectReference\s+Include="(?P<name>[^"]+)"(?P<rest>.+?)(/>|</ProjectReference>)"#)
                .case_insensitive(true).dot_matches_new_line(true).build().unwrap();
        }

        let mut paths: Vec<PathBuf> = PROJECT_REF_REGEX.captures_iter(&self.file_info.contents)
            .map(|cap| {
                let mut path = self.file_info.path.parent().unwrap().to_owned();
                // This will be something like "..\Foo\Foo.csproj"
                let relative_csproj_path = norm_mentioned_path(&cap["name"]);
                path.push(relative_csproj_path);
                let path = normalize_path(&path);
                path
            })
            .collect();

        paths.sort();
        paths.dedup();
        paths
    }


    fn extract_packages<L: FileLoader>(&self, file_loader: &L, configuration: &Configuration) -> Vec<Package> {
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

            static ref SDK_RE: Regex = RegexBuilder::new(r#"<PackageReference\s+Include="(?P<name>[^"]+)"(?P<rest>.+?)(/>|</PackageReference>)"#)
                .case_insensitive(true).dot_matches_new_line(true).build().unwrap();

            static ref SDK_VERSION_RE: Regex = RegexBuilder::new(r#"(Version="(?P<version>[^"]+)"|<Version>(?P<version2>[^<]+)</Version>)"#)
                .case_insensitive(true).build().unwrap();

            static ref PKG_CONFIG_RE: Regex = RegexBuilder::new(r#"<package\s*?id="(?P<name>.*?)"\s*?version="(?P<version>.*?)"(?P<inner>.*?)\s*?/>"#)
                .case_insensitive(true).build().unwrap();
        }

        let classify = |pkg_name: &str| -> String {
            for pkg_group in &configuration.package_groups {
                if pkg_group.regex.is_match(pkg_name) {
                    return pkg_group.name.clone();
                }
            }

            "Unclassified".to_owned()
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
                self.find_other_file(InterestingFile::PackagesConfig)
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
mod analysis_tests {
    use super::*;
    use tempfile;
    use std::io::{self, Write};
    use std::fs::{self, File};
    use crate::io::PathExtensions;

    fn make_temporary_directory() -> io::Result<tempfile::TempDir> {
        let root = tempfile::Builder::new()
            .prefix("dnlib-temp-")
            .rand_bytes(5)
            .tempdir()?;

        let file_path = root.path().join("car.sln");
        let mut file = File::create(&file_path)?;

        // Slns always use Windows-style paths, even when using 'dotnet' on Linux.
        writeln!(file, r#"
                        "ford.csproj"
                        "sub\toyota.csproj"
                        "#)?;

        let file_path = root.path().join("ford.csproj");
        File::create(&file_path)?;
        let file_path = root.path().join("bmw.csproj");
        File::create(&file_path)?;

        let sub_dir = root.path().join("sub");
        fs::create_dir_all(&sub_dir)?;
        let file_path = sub_dir.join("toyota.csproj");
        File::create(file_path)?;

        // Trucks.
        let truck_dir = root.path().join("trucks");
        fs::create_dir_all(&truck_dir)?;

        let file_path = truck_dir.join("truck.sln");
        let mut file = File::create(&file_path)?;
        writeln!(file, r#"  "volvo.csproj"  "#)?;

        let file_path = truck_dir.join("volvo.csproj");
        File::create(&file_path)?;

        let file_path = truck_dir.join("mercedes.csproj");
        File::create(&file_path)?;

        let file_path = truck_dir.join("renault.csproj");
        File::create(&file_path)?;

        Ok(root)
    }

    #[test]
    pub fn test_disk_scanning_and_project_association() {
        let temp_files = make_temporary_directory().unwrap();
        let root_dir = temp_files.path();
        let config = Configuration::default();
        let analyzed_files = Analysis::new(&config).unwrap();

        assert_eq!(analyzed_files.solution_directories.len(), 2);

        let car_sln_dir = &analyzed_files.solution_directories[0];
        assert_eq!(car_sln_dir.directory, root_dir);
        assert_eq!(car_sln_dir.num_solutions(), 1);
        assert_eq!(car_sln_dir.num_linked_projects(), 2);
        assert_eq!(car_sln_dir.num_orphaned_projects(), 1);
        let car_sln = &car_sln_dir.solutions[0];
        assert_eq!(car_sln.file_info.filename_as_str(), "car.sln");
        assert_eq!(car_sln.linked_projects().nth(0).unwrap().file_info.path.filename_as_str(), "ford.csproj");
        assert_eq!(car_sln.linked_projects().nth(1).unwrap().file_info.path.filename_as_str(), "toyota.csproj");
        // BMW is orphaned because not actually mentioned in the sln file.
        assert_eq!(car_sln.orphaned_projects().nth(0).unwrap().file_info.path.filename_as_str(), "bmw.csproj");


        let truck_sln_dir = &analyzed_files.solution_directories[1];
        let expected_truck_dir = root_dir.join("trucks");
        assert_eq!(truck_sln_dir.directory, expected_truck_dir);
        assert_eq!(truck_sln_dir.num_solutions(), 1);
        assert_eq!(truck_sln_dir.num_linked_projects(), 1);
        assert_eq!(truck_sln_dir.num_orphaned_projects(), 2);
        let truck_sln = &truck_sln_dir.solutions[0];
        assert_eq!(truck_sln.file_info.filename_as_str(), "truck.sln");
        assert_eq!(truck_sln.linked_projects().nth(0).unwrap().file_info.path.filename_as_str(), "volvo.csproj");
        assert_eq!(truck_sln.orphaned_projects().nth(0).unwrap().file_info.path.filename_as_str(), "mercedes.csproj");
        assert_eq!(truck_sln.orphaned_projects().nth(1).unwrap().file_info.path.filename_as_str(), "renault.csproj");
    }
}

#[cfg(test)]
 mod project_tests {
    use super::*;
    use crate::io::MemoryFileLoader;

    #[derive(Default)]
    struct ProjectBuilder {
         csproj_contents: String,
         project_version: ProjectVersion,
         packages_config_contents: Option<String>,
         other_files: Vec<PathBuf>
     }

    impl ProjectBuilder {
        fn new<S>(csproj_contents: S) -> Self
        where S: Into<String>
        {
            ProjectBuilder {
                csproj_contents: csproj_contents.into(),
                .. ProjectBuilder::default()
            }
        }

        fn with_packages_config(mut self, packages_config_contents: &str) -> Self {
            self.packages_config_contents = Some(packages_config_contents.to_owned());
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

            Project::new(&project_path, self.other_files, &file_loader, &Configuration::default())
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
        let project = ProjectBuilder::new(r#""#).build();
        assert_eq!(project.version, ProjectVersion::Unknown);

        let project = ProjectBuilder::new(r#""#).sdk().build();
        assert_eq!(project.version, ProjectVersion::MicrosoftNetSdk);

        let project = ProjectBuilder::new(r#""#).old().build();
        assert_eq!(project.version, ProjectVersion::OldStyle);

        let project = ProjectBuilder::new(r#""#).web().build();
        assert_eq!(project.version, ProjectVersion::MicrosoftNetSdkWeb);
    }

    #[test]
    pub fn extract_output_type_works() {
        let project = ProjectBuilder::new(r#""#).build();
        assert_eq!(project.output_type, OutputType::Library);

        let project = ProjectBuilder::new(r#"<OutputType>Library</OutputType>"#).build();
        assert_eq!(project.output_type, OutputType::Library);

        let project = ProjectBuilder::new(r#"<OutputType>Exe</OutputType>"#).build();
        assert_eq!(project.output_type, OutputType::Exe);

        let project = ProjectBuilder::new(r#"<OutputType>WinExe</OutputType>"#).build();
        assert_eq!(project.output_type, OutputType::WinExe);
    }

    #[test]
    pub fn extract_xml_doc_works() {
        let project = ProjectBuilder::new(r#""#).build();
        assert_eq!(project.xml_doc, XmlDoc::None);

        let project = ProjectBuilder::new(r#"blah<DocumentationFile>bin\Debug\WorkflowService.Client.xml</DocumentationFile>blah"#).build();
        assert_eq!(project.xml_doc, XmlDoc::Debug);

        let project = ProjectBuilder::new(r#"blah<DocumentationFile>bin\Release\WorkflowService.Client.xml</DocumentationFile>blah"#).build();
        assert_eq!(project.xml_doc, XmlDoc::Release);

        let project = ProjectBuilder::new(r#"blah<DocumentationFile>bin\Release\WorkflowService.Client.xml</DocumentationFile>
            <DocumentationFile>bin\Debug\WorkflowService.Client.xml</DocumentationFile>blah"#).build();
        assert_eq!(project.xml_doc, XmlDoc::Both);
    }

    #[test]
    pub fn extract_tt_file_works() {
        let project = ProjectBuilder::new(r#""#).build();
        assert!(!project.tt_file);

        let project = ProjectBuilder::new(r#"blah<None Update="NuSpecTemplate.tt">blah"#).build();
        assert!(!project.tt_file);

        let project = ProjectBuilder::new(r#"blah<None Update="NuSpecTemplate.nuspec">blah"#).build();
        assert!(!project.tt_file);

        let project = ProjectBuilder::new(r#"blah<None Update="NuSpecTemplate.nuspec">blah
            <None Update="NuSpecTemplate.tt">blah"#).build();
        assert!(project.tt_file);

        let project = ProjectBuilder::new(r#"blah<None Include="NuSpecTemplate.nuspec">blah
            <None Include="NuSpecTemplate.tt">blah"#).build();
        assert!(project.tt_file);
    }

    #[test]
    pub fn extract_embedded_debugging_works() {
        let project = ProjectBuilder::new(r#""#).build();
        assert!(!project.embedded_debugging);

        let project = ProjectBuilder::new(r#"blah<DebugType>embedded</DebugType>blah"#).build();
        assert!(!project.embedded_debugging);

        let project = ProjectBuilder::new(r#"blah<EmbedAllSources>true</EmbedAllSources>blah"#).build();
        assert!(!project.embedded_debugging);

        let project = ProjectBuilder::new(r#"blah<DebugType>embedded</DebugType>blah"
            <EmbedAllSources>true</EmbedAllSources>blah"#).sdk().build();
        assert!(project.embedded_debugging);
    }

    #[test]
    pub fn extract_linked_solution_info_works() {
        let project = ProjectBuilder::new(r#""#).build();
        assert!(!project.linked_solution_info);

        // SDK style.
        let project = ProjectBuilder::new(r#"blah<ItemGroup>
            <Compile Include="..\SolutionInfo.cs" Link="Properties\SolutionInfo.cs" />blah
            </ItemGroup>blah"#).build();
        assert!(project.linked_solution_info);

        // Old style.
        let project = ProjectBuilder::new(r#"blah<Compile Include="..\SolutionInfo.cs">
            <Link>Properties\SolutionInfo.cs</Link>blah
            </Compile>blah"#).build();
        assert!(project.linked_solution_info);
    }

    #[test]
    pub fn extract_auto_generate_binding_redirects_works() {
        let project = ProjectBuilder::new(r#""#).build();
        assert!(!project.auto_generate_binding_redirects);

        let project = ProjectBuilder::new(r#"blah<AutoGenerateBindingRedirects>true</AutoGenerateBindingRedirects>blah"#).build();
        assert!(project.auto_generate_binding_redirects);

        let project = ProjectBuilder::new(r#"blah<AutoGenerateBindingRedirects>false</AutoGenerateBindingRedirects>blah"#).build();
        assert!(!project.auto_generate_binding_redirects);
    }

    #[test]
    pub fn extract_referenced_assemblies_works() {
        let project = ProjectBuilder::new(r#""#).build();
        assert!(project.referenced_assemblies.is_empty());

        let project = ProjectBuilder::new(r#"blah<Reference Include="System.Windows" />blah"#).build();
        assert_eq!(project.referenced_assemblies, vec!["System.Windows"]);

        let project = ProjectBuilder::new(r#"blah<Reference Include="System.Windows" />blah
            blah<Reference Include="System.Windows" />blah"#).build();
        assert_eq!(project.referenced_assemblies, vec!["System.Windows"]);

        let project = ProjectBuilder::new(r#"blah<Reference Include="System.Windows" />blah
            blah<Reference Include="System.Data" />blah"#).build();
        assert_eq!(project.referenced_assemblies, vec!["System.Data", "System.Windows"]);
    }

    #[test]
    pub fn sdk_extract_target_frameworks_works() {
        let project = ProjectBuilder::new(r#""#).build();
        assert!(project.target_frameworks.is_empty());

        let project = ProjectBuilder::new(r#"blah<TargetFramework>net462</TargetFramework>blah"#).sdk().build();
        assert_eq!(project.target_frameworks, vec!["net462"]);

        // I don't believe this happens, but this is what we get.
        let project = ProjectBuilder::new(r#"blah<TargetFramework>net462</TargetFramework>blah<TargetFramework>net472</TargetFramework>"#).sdk().build();
        assert_eq!(project.target_frameworks, vec!["net462", "net472"]);

        let project = ProjectBuilder::new(r#"blah<TargetFrameworks>net462;net472</TargetFrameworks>blah"#).sdk().build();
        assert_eq!(project.target_frameworks, vec!["net462", "net472"]);
    }

    #[test]
    pub fn old_extract_target_frameworks_works() {
        let project = ProjectBuilder::new(r#""#).build();
        assert!(project.target_frameworks.is_empty());

        let project = ProjectBuilder::new(r#"blah<TargetFrameworkVersion>v4.6.2</TargetFrameworkVersion>blah"#).old().build();
        assert_eq!(project.target_frameworks, vec!["v4.6.2"]);

        let project = ProjectBuilder::new(r#"blah<TargetFrameworkVersion>v4.6.2</TargetFrameworkVersion>blah
            <TargetFrameworkVersion>v4.7.2</TargetFrameworkVersion>"#).old().build();
        assert_eq!(project.target_frameworks, vec!["v4.6.2", "v4.7.2"]);
    }

    #[test]
    pub fn has_packages_config_not_present() {
        let project = ProjectBuilder::new(r#""#).build();
        assert_eq!(project.packages_config, FileStatus::NotPresent);
    }

    #[test]
    pub fn has_packages_config_on_disk() {
        let project = ProjectBuilder::new(r#""#).with_packages_config("contents").build();
        assert_eq!(project.packages_config, FileStatus::OnDiskOnly);
    }

    #[test]
    pub fn has_packages_config_in_project_file_only() {
        let project = ProjectBuilder::new(r#" Include="packages.config" />"#).build();
        assert_eq!(project.packages_config, FileStatus::InProjectFileOnly);
    }

    #[test]
    pub fn has_packages_config_in_project_file_and_on_disk() {
        let project = ProjectBuilder::new(r#" Include="packages.config" />"#).with_packages_config("contents").build();
        assert_eq!(project.packages_config, FileStatus::InProjectFileAndOnDisk);
    }

    #[test]
    pub fn extract_packages_sdk_one_line() {
        let project = ProjectBuilder::new(r#""#).sdk().build();
        assert!(project.packages.is_empty());

        let project = ProjectBuilder::new(r#"blah<PackageReference Include="Unity" Version="4.0.1" />blah"#).sdk().build();
        assert_eq!(project.packages, vec![Package::new("Unity", "4.0.1", false, "Third Party")]);
    }

    #[test]
    pub fn extract_packages_sdk_one_line_sorts() {
        let project = ProjectBuilder::new(
            r#"
            blah<PackageReference Include="Unity" Version="4.0.1" />blah
            blah<PackageReference Include="Automapper" Version="3.1.4" />blah
            "#
            ).sdk().build();

        assert_eq!(project.packages, vec![
            Package::new("Automapper", "3.1.4", false, "Third Party"),
            Package::new("Unity", "4.0.1", false, "Third Party")
            ]);

        // Dedup & sort by secondary key (version).
        let project = ProjectBuilder::new(
            r#"
            blah<PackageReference Include="Automapper" Version="3.1.5" />blah
            blah<PackageReference Include="Unity" Version="4.0.1" />blah
            blah<PackageReference Include="Automapper" Version="3.1.4" />blah
            blah<PackageReference Include="Unity" Version="4.0.1" />blah
            "#
            ).sdk().build();

        assert_eq!(project.packages, vec![
            Package::new("Automapper", "3.1.4", false, "Third Party"),
            Package::new("Automapper", "3.1.5", false, "Third Party"),
            Package::new("Unity", "4.0.1", false, "Third Party")
            ]);
    }

    #[test]
    pub fn extract_packages_sdk_one_line_dedups() {
        // Dedup & sort by secondary key (i.e. the version).
        let project = ProjectBuilder::new(
            r#"
            blah<PackageReference Include="Automapper" Version="3.1.5" />blah
            blah<PackageReference Include="Unity" Version="4.0.1" />blah
            blah<PackageReference Include="Automapper" Version="3.1.4" />blah
            blah<PackageReference Include="Unity" Version="4.0.1" />blah
            "#
            ).sdk().build();

        assert_eq!(project.packages, vec![
            Package::new("Automapper", "3.1.4", false, "Third Party"),
            Package::new("Automapper", "3.1.5", false, "Third Party"),
            Package::new("Unity", "4.0.1", false, "Third Party")
            ]);
    }

    #[test]
    pub fn extract_packages_sdk_multi_line() {
        let project = ProjectBuilder::new(
            r#"
            blah<PackageReference Include="Unity" Version="4.0.1">
                </PackageReference>
            "#
        ).sdk().build();

        assert_eq!(project.packages, vec![
            Package::new("Unity", "4.0.1", false, "Third Party")
            ]);
    }

    #[test]
    pub fn extract_packages_sdk_multi_line_private_assets() {
        let project = ProjectBuilder::new(
            r#"
            blah<PackageReference Include="Unity" Version="4.0.1">
                <PrivateAssets>
                </PackageReference>
            "#
        ).sdk().build();

        assert_eq!(project.packages, vec![
            Package::new("Unity", "4.0.1", true, "Third Party")
            ]);
    }

    #[test]
    pub fn extract_packages_sdk_multi_line_flip_flop() {
        // This flip-flop of styles discovered problems in the regex when it
        // was not terminating early enough.
        let project = ProjectBuilder::new(
            r#"
            blah<PackageReference Include="Unity" Version="4.0.1">
                </PackageReference>

                <PackageReference Include="EntityFramework" Version="2.4.6" />

                <PackageReference Include="Automapper" Version="3.1.4">
                    <PrivateAssets>
                </PackageReference>

                <PackageReference Include="Versioning.Bamboo" Version="8.8.9" />
            "#
        ).sdk().build();

        assert_eq!(project.packages, vec![
            Package::new("Automapper", "3.1.4", true, "Third Party"),
            Package::new("EntityFramework", "2.4.6", false, "Microsoft"),
            Package::new("Unity", "4.0.1", false, "Third Party"),
            Package::new("Versioning.Bamboo", "8.8.9", false, "Third Party")
            ]);
    }

    #[test]
    pub fn extract_packages_worst_case_seen_in_real_life() {
        // This flip-flop of styles discovered problems in the regex when it
        // was not terminating early enough.
        let project = ProjectBuilder::new(
            r#"
            <PackageReference Include="MoreFluentAssertions" Version="1.2.3" />
            <PackageReference Include="Microsoft.EntityFrameworkCore">
                <Version>2.1.4</Version>
            </PackageReference>
            <PackageReference Include="Landmark.Versioning.Bamboo" Version="3.3.19078.47">
                <PrivateAssets>all</PrivateAssets>
                <IncludeAssets>runtime; build; native; contentfiles; analyzers</IncludeAssets>
            </PackageReference>
            <PackageReference Include="FluentAssertions">
                  <Version>5.6.0</Version>
            </PackageReference>
            <PackageReference Include="MoreFluentAssertions" Version="1.2.3" />
            <PackageReference Include="Landmark.Versioning.Bamboo" Version="3.3.19078.47">
                <PrivateAssets>all</PrivateAssets>
                <IncludeAssets>runtime; build; native; contentfiles; analyzers</IncludeAssets>
            </PackageReference>
            <PackageReference Include="JsonNet.PrivateSettersContractResolvers.Source" Version="0.1.0">
                <PrivateAssets>all</PrivateAssets>
                <IncludeAssets>runtime; build; native; contentfiles; analyzers</IncludeAssets>
            </PackageReference>
            "#
        ).sdk().build();

        assert_eq!(project.packages, vec![
            Package::new("FluentAssertions", "5.6.0", false, "Third Party"),
            Package::new("JsonNet.PrivateSettersContractResolvers.Source", "0.1.0", true, "Third Party"),
            Package::new("Landmark.Versioning.Bamboo", "3.3.19078.47", true, "ValHub"),
            Package::new("Microsoft.EntityFrameworkCore", "2.1.4", false, "Microsoft"),
            Package::new("MoreFluentAssertions", "1.2.3", false, "Third Party"),
            ]);
    }

    #[test]
    pub fn extract_packages_old_including_sort_and_dedup() {
        let project = ProjectBuilder::new(r#" Include="packages.config" />"#).old()
            .with_packages_config(r#"
            <package id="Clarius.TransformOnBuild" version="1.1.12" targetFramework="net462" developmentDependency="true" />
            <package id="Castle.Core" version="4.3.1" targetFramework="net462" />
            <package id="Owin" version="1.0" targetFramework="net462" />
            <package id="Castle.Core" version="4.3.1" targetFramework="net462" />
            "#).build();
        assert_eq!(project.packages, vec![
            Package::new("Castle.Core", "4.3.1", false, "Third Party"),
            Package::new("Clarius.TransformOnBuild", "1.1.12", true, "Third Party"),
            Package::new("Owin", "1.0", false, "Microsoft"),
        ]);
    }

    #[test]
    pub fn extract_test_framework_mstest() {
        let project = ProjectBuilder::new(r#"<PackageReference Include="MSTest.TestFramework" Version="4.0.1" />"#)
            .sdk().build();
        assert_eq!(project.test_framework, TestFramework::MSTest);
    }

    #[test]
    pub fn extract_test_framework_xunit() {
        let project = ProjectBuilder::new(r#"<PackageReference Include="Xunit.Core" Version="4.0.1" />"#)
            .sdk().build();
        assert_eq!(project.test_framework, TestFramework::XUnit);
    }

    #[test]
    pub fn extract_test_framework_nunit() {
        let project = ProjectBuilder::new(r#"<PackageReference Include="NUnit.Core" Version="4.0.1" />"#)
            .sdk().build();
        assert_eq!(project.test_framework, TestFramework::NUnit);
    }

    #[test]
    pub fn extract_test_framework_none() {
        let project = ProjectBuilder::new(r#"<PackageReference Include="MSTestNotMatched" Version="4.0.1" />"#)
            .sdk().build();
        assert_eq!(project.test_framework, TestFramework::None);
    }

    #[test]
    pub fn extract_uses_specflow_works() {
        let project = ProjectBuilder::new(r#"<PackageReference Include="NUnit.Core" Version="4.0.1" />"#)
            .sdk().build();
        assert!(!project.uses_specflow);

        let project = ProjectBuilder::new(r#"<PackageReference Include="SpecFlow" Version="2.3.2" />"#)
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
                Package::new("Landmark.Versioning.Bamboo", "3.1.44", true, "ValHub"),
                Package::new("Unity", "4.0.1", false, "Third Party"),
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
            let project = get_old_project_with_packages(r#"
                <package id="Clarius.TransformOnBuild" version="1.1.12" targetFramework="net462" developmentDependency="true" />
                <package id="MyCorp.Fundamentals" version="1.2.18268.136" targetFramework="net462" />
                <package id="Microsoft.Owin.Hosting" version="4.0.0" targetFramework="net462" />
                <package id="Microsoft.Owin.SelfHost" version="4.0.0" targetFramework="net462" />
                <package id="Moq" version="4.8.3" targetFramework="net462" />
                <package id="Newtonsoft.Json" version="11.0.2" targetFramework="net462" />
                <package id="Npgsql" version="3.2.7" targetFramework="net462" />
                <package id="MyProject.Core" version="1.12.18297.228" targetFramework="net462" />
                <package id="WorkflowService.Client" version="1.12.18297.23" targetFramework="net462" />
            "#);

            assert_eq!(project.packages, vec![
                Package::new("Clarius.TransformOnBuild", "1.1.12", true, "Third Party"),
                Package::new("Microsoft.Owin.Hosting", "4.0.0", false, "Microsoft"),
                Package::new("Microsoft.Owin.SelfHost", "4.0.0", false, "Microsoft"),
                Package::new("Moq", "4.8.3", false, "Third Party"),
                Package::new("MyCorp.Fundamentals", "1.2.18268.136", false, "Third Party"),
                Package::new("MyProject.Core", "1.12.18297.228", false, "Third Party"),
                Package::new("Newtonsoft.Json", "11.0.2", false, "Third Party"),
                Package::new("Npgsql", "3.2.7", false, "Third Party"),
                Package::new("WorkflowService.Client", "1.12.18297.23", false, "VRM"),
            ]);
        }
    }
}
