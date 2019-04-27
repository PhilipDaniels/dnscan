use csv;
use dnlib::prelude::*;
use crate::errors::AnalysisResult;

fn bool_to_str(b: bool) -> &'static str {
    if b { "true" } else { "false" }
}

pub fn write_files(analysis: &Analysis) -> AnalysisResult<()> {
    write_solutions(analysis)?;
    write_solutions_to_projects(analysis)?;
    write_projects_to_packages(analysis)?;
    write_projects_to_child_projects(analysis)?;
    Ok(())
}

fn write_solutions(analysis: &Analysis) -> AnalysisResult<()> {
    let mut wtr = csv::Writer::from_path("solutions.csv")?;

    wtr.write_record(&[
        "SlnDirectory",
        "GitBranch", "GitSha", "GitSummary", "GitCommitTime", "GitAuthor", "GitAuthorEmail", "GitRemoteName", "GitRemoteUrl",
        "SlnPath", "SlnFile", "SlnIsValidUTF8", "SlnVersion",
        "LinkedProjectsCount", "OrphanedProjectsCount"
    ])?;

    for sd in &analysis.solution_directories {
        for sln in &sd.solutions {
            wtr.write_record(&[
                // sln columns
                sd.directory.as_str(),
                sd.git_info.as_ref().map_or("", |git_info| &git_info.branch),
                sd.git_info.as_ref().map_or("", |git_info| &git_info.sha),
                sd.git_info.as_ref().map_or("", |git_info| &git_info.summary),
                sd.git_info.as_ref().map_or("", |git_info| &git_info.commit_time),
                sd.git_info.as_ref().map_or("", |git_info| &git_info.author),
                sd.git_info.as_ref().map_or("", |git_info| &git_info.author_email),
                sd.git_info.as_ref().map_or("", |git_info| &git_info.remote_name),
                sd.git_info.as_ref().map_or("", |git_info| &git_info.remote_url),
                sln.file_info.path_as_str(),
                sln.file_info.filename_as_str(),
                bool_to_str(sln.file_info.is_valid_utf8),
                sln.version.as_ref(),
                // project columns
                &sln.linked_projects().count().to_string(),
                &sln.orphaned_projects().count().to_string(),
            ])?;
        }
    }

    wtr.flush()?;
    Ok(())
}

fn write_solutions_to_projects(analysis: &Analysis) -> AnalysisResult<()> {
    let mut wtr = csv::Writer::from_path("solutions_to_projects.csv")?;

    wtr.write_record(&[
        "SlnDirectory", "SlnPath", "SlnFile", "SlnIsValidUTF8", "SlnVersion",
        "ProjOwnership", "ProjPath", "ProjFile", "ProjIsValidUTF8", "ProjVersion", "ProjOutputType", "ProjXmlDoc", "ProjTTFile",
        "ProjEmbeddedDebugging", "ProjLinkedSolutionInfo", "ProjAutoGenerateBindingRedirects", "ProjTargetFrameworks",
        "ProjTestFramework", "ProjUsesSpecflow",
        "ProjPackagesCount", "ProjAssembliesCount", "ProjChildCount",
        "ProjWebConfig", "ProjAppConfig", "ProjAppSettingsJson", "ProjPackageJson", "ProjPackagesConfig", "ProjProjectJson"
    ])?;

    for sd in &analysis.solution_directories {
        for sln in &sd.solutions {
            for proj in &sln.projects {
                wtr.write_record(&[
                    // sln columns
                    sd.directory.as_str(),
                    sln.file_info.path_as_str(),
                    sln.file_info.filename_as_str(),
                    &sln.file_info.is_valid_utf8.to_string(),
                    sln.version.as_ref(),
                    // project columns
                    proj.ownership.as_ref(),
                    proj.file_info.path_as_str(),
                    proj.file_info.filename_as_str(),
                    bool_to_str(proj.file_info.is_valid_utf8),
                    proj.version.as_ref(),
                    proj.output_type.as_ref(),
                    proj.xml_doc.as_ref(),
                    bool_to_str(proj.tt_file),
                    bool_to_str(proj.embedded_debugging),
                    bool_to_str(proj.linked_solution_info),
                    bool_to_str(proj.auto_generate_binding_redirects),
                    &proj.target_frameworks.join(","),
                    proj.test_framework.as_ref(),
                    bool_to_str(proj.uses_specflow),
                    &proj.packages.len().to_string(),
                    &proj.referenced_assemblies.len().to_string(),
                    &proj.get_child_projects(sln).len().to_string(),
                    proj.web_config.as_ref(),
                    proj.app_config.as_ref(),
                    proj.app_settings_json.as_ref(),
                    proj.package_json.as_ref(),
                    proj.packages_config.as_ref(),
                    proj.project_json.as_ref(),
                ])?;
            }
        }
    }

    wtr.flush()?;
    Ok(())
}

fn write_projects_to_packages(analysis: &Analysis) -> AnalysisResult<()> {
    let mut wtr = csv::Writer::from_path("projects_to_packages.csv")?;

    wtr.write_record(&[
        "SlnDirectory", "SlnPath", "SlnFile", "SlnIsValidUTF8", "SlnVersion",
        "ProjOwnership", "ProjPath", "ProjFile", "ProjIsValidUTF8", "ProjVersion", "ProjOutputType", "ProjTargetFrameworks",
        "PkgName", "PkgClass", "PkgVersion", "PkgIsDevelopment", "PkgIsPreview"
    ])?;

    for sd in &analysis.solution_directories {
        for sln in &sd.solutions {
            for proj in &sln.projects {
                for pkg in &proj.packages {
                    wtr.write_record(&[
                        // sln columns
                        sd.directory.as_str(),
                        sln.file_info.path_as_str(),
                        sln.file_info.filename_as_str(),
                        bool_to_str(sln.file_info.is_valid_utf8),
                        sln.version.as_ref(),
                        // project columns
                        proj.ownership.as_ref(),
                        proj.file_info.path_as_str(),
                        proj.file_info.filename_as_str(),
                        bool_to_str(proj.file_info.is_valid_utf8),
                        proj.version.as_ref(),
                        proj.output_type.as_ref(),
                        &proj.target_frameworks.join(","),
                        // package columns
                        &pkg.name,
                        &pkg.class,
                        &pkg.version,
                        bool_to_str(pkg.development),
                        bool_to_str(pkg.is_preview()),
                    ])?;
                }
            }
        }
    }

    wtr.flush()?;
    Ok(())
}

fn write_projects_to_child_projects(analysis: &Analysis) -> AnalysisResult<()> {
    let mut wtr = csv::Writer::from_path("projects_to_child_projects.csv")?;

    wtr.write_record(&[
        "SlnDirectory", "SlnPath", "SlnFile",
        "ProjPath", "ProjFile", "ProjIsValidUTF8", "ProjVersion", "ProjOutputType",
        "ChildProjPath", "ChildProjFile", "ChildProjIsValidUTF8", "ChildProjVersion", "ChildProjOutputType",
    ])?;


    for sd in &analysis.solution_directories {
        for sln in &sd.solutions {
            for owning_proj in &sln.projects {
                for child_proj in &owning_proj.get_child_projects(sln) {
                    wtr.write_record(&[
                        // sln columns
                        sd.directory.as_str(),
                        sln.file_info.path_as_str(),
                        sln.file_info.filename_as_str(),
                        // project columns
                        owning_proj.file_info.path_as_str(),
                        owning_proj.file_info.filename_as_str(),
                        bool_to_str(owning_proj.file_info.is_valid_utf8),
                        owning_proj.version.as_ref(),
                        owning_proj.output_type.as_ref(),
                        // referenced project columns
                        child_proj.file_info.path_as_str(),
                        child_proj.file_info.filename_as_str(),
                        bool_to_str(child_proj.file_info.is_valid_utf8),
                        child_proj.version.as_ref(),
                        child_proj.output_type.as_ref(),
                    ])?;
                }
            }
        }
    }

    wtr.flush()?;
    Ok(())
}