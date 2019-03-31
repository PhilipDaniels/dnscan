use csv;
use dnlib::prelude::*;
use crate::errors::AnalysisResult;

pub fn write_files(analysis: &AnalyzedFiles) -> AnalysisResult<()> {
    write_solutions(analysis)?;
    write_solutions_to_projects(analysis)?;
    // write_projects(projects)?;
    Ok(())
}

fn write_solutions(analysis: &AnalyzedFiles) -> AnalysisResult<()> {
    let mut wtr = csv::Writer::from_path("solutions.csv")?;

    wtr.write_record(&["SlnDirectory", "SlnPath", "SlnFile", "SlnIsValidUTF8", "SlnVersion", "LinkedProjectsCount", "OrphanedProjectsCount"])?;

    for sd in &analysis.solution_directories {
        for sln in &sd.solutions {
            wtr.write_record(&[
                // sln columns
                sd.directory.as_str(),
                sln.file_info.path_as_str(),
                sln.file_info.filename_as_str(),
                sln.file_info.is_valid_utf8.as_str(),
                sln.version.as_str(),
                // project columns
                &sln.num_linked_projects().to_string(),
                &sln.num_orphaned_projects().to_string(),
            ])?;
        }
    }

    wtr.flush()?;
    Ok(())
}

fn write_solutions_to_projects(analysis: &AnalyzedFiles) -> AnalysisResult<()> {
    let mut wtr = csv::Writer::from_path("solutions_to_projects.csv")?;

    wtr.write_record(&["SlnDirectory", "SlnPath", "SlnFile", "SlnIsValidUTF8", "SlnVersion",
    "ProjLinkage", "ProjPath", "ProjFile", "ProjIsValidUTF8", "ProjVersion", "ProjOutputType", "ProjXmlDoc", "ProjTTFile",
    "ProjEmbeddedDebugging", "ProjLinkedSolutionInfo", "ProjAutoGenerateBindingRedirects", "ProjTargetFrameworks",
    "ProjTestFramework", "ProjUsesSpecflow",
    "ProjPackagesCount", "ProjAssembliesCount", "ProjReferencedProjectCount",
    "ProjWebConfig", "ProjAppConfig", "ProjAppSettingsJson", "ProjPackageJson", "ProjPackagesConfig", "ProjProjectJson"
    ])?;

    for sd in &analysis.solution_directories {
        for sln in &sd.solutions {
            let all_projects = sln.linked_projects.iter().map(|proj| ("Linked", proj))
                                    .chain(sln.orphaned_projects.iter().map(|proj| ("Orphaned", proj)));

            for (link_type, proj) in all_projects {
                wtr.write_record(&[
                    // sln columns
                    sd.directory.as_str(),
                    sln.file_info.path_as_str(),
                    sln.file_info.filename_as_str(),
                    sln.file_info.is_valid_utf8.as_str(),
                    sln.version.as_str(),
                    // project columns
                    link_type,
                    proj.file_info.path_as_str(),
                    proj.file_info.filename_as_str(),
                    proj.file_info.is_valid_utf8.as_str(),
                    proj.version.as_str(),
                    proj.output_type.as_str(),
                    proj.xml_doc.as_str(),
                    proj.tt_file.as_str(),
                    proj.embedded_debugging.as_str(),
                    proj.linked_solution_info.as_str(),
                    proj.auto_generate_binding_redirects.as_str(),
                    &proj.target_frameworks.join(","),
                    proj.test_framework.as_str(),
                    proj.uses_specflow.as_str(),
                    &proj.packages.len().to_string(),
                    &proj.referenced_assemblies.len().to_string(),
                    &proj.referenced_projects.len().to_string(),
                    proj.web_config.as_str(),
                    proj.app_config.as_str(),
                    proj.app_settings_json.as_str(),
                    proj.package_json.as_str(),
                    proj.packages_config.as_str(),
                    proj.project_json.as_str(),
                ])?;
            }
        }
    }

    wtr.flush()?;
    Ok(())
}

fn write_projects(projects: &[Project]) -> AnalysisResult<()> {
    let mut wtr = csv::Writer::from_path("projects_to_packages.csv")?;
    wtr.write_record(&[
        "Version", "Directory", "File", "IsValidUTF8",
        "OutputType", "XmlDoc", "TTFile",
        "TargetFrameworks", "EmbeddedDebugging", "Linked SolutionInfo.cs",
        "AutoGenerateBindingRedirects", "TestFramework", "UsesSpecflow",
        "web.config",
        "app.config",
        "appsettings.json",
        "package.json",
        "packages.config",
        "project.json",
        "PkgName", "PkgClass", "PkgVersion", "PkgIsDev", "PkgIsPreview",
        "Path"
        ])?;

    for proj in projects {
        for pkg in &proj.packages {
            wtr.write_record(&[
                proj.version.as_str(),
                proj.file_info.directory_as_str(),
                proj.file_info.filename_as_str(),
                proj.file_info.is_valid_utf8.as_str(),
                proj.output_type.as_str(),
                proj.xml_doc.as_str(),
                proj.tt_file.as_str(),
                &proj.target_frameworks.join(","),
                proj.embedded_debugging.as_str(),
                proj.linked_solution_info.as_str(),
                proj.auto_generate_binding_redirects.as_str(),
                proj.test_framework.as_str(),
                proj.uses_specflow.as_str(),
                proj.web_config.as_str(),
                proj.app_config.as_str(),
                proj.app_settings_json.as_str(),
                proj.package_json.as_str(),
                proj.packages_config.as_str(),
                proj.project_json.as_str(),
                &pkg.name,
                pkg.class.as_str(),
                &pkg.version,
                pkg.development.as_str(),
                pkg.is_preview().as_str(),
                proj.file_info.path_as_str(),
            ])?;
        }
    }

    Ok(())
}
