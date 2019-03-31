use csv;
use dnlib::prelude::*;
use crate::errors::AnalysisResult;

pub fn write_files(analysis: &AnalyzedFiles) -> AnalysisResult<()> {
    // write_solutions(solutions)?;
    // write_projects(projects)?;
    Ok(())
}

fn write_solutions(solutions: &[Solution]) -> AnalysisResult<()> {
    let mut wtr = csv::Writer::from_path("solutions.csv")?;
    wtr.write_record(&["Version", "Directory", "File", "IsValidUTF8", "ProjectCount", "OrphanedProjectCount"])?;

    for sln in solutions {
        wtr.write_record(&[
            sln.version.as_str(),
            sln.file_info.path.directory_as_str(),
            sln.file_info.path_as_str(),
            sln.file_info.is_valid_utf8.as_str(),
            "0", //&sln.linked_projects.len().to_string(),
            "0", //&sln.orphaned_projects.len().to_string()
        ])?;
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
