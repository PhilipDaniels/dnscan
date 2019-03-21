use crate::errors::AnalysisResult;
use crate::solution::Solution;
use crate::project::Project;
use dnlib::path_extensions::PathExtensions;
use csv;

pub fn write_solutions(solutions: &[Solution]) -> AnalysisResult<()> {
    let mut wtr = csv::Writer::from_path("solutions.csv")?;
    wtr.write_record(&["Version", "Directory", "File", "IsValidUTF8", "ProjectCount", "OrphanedProjectCount"])?;

    for sln in solutions {
        wtr.write_record(&[
            sln.version.as_str(),
            &sln.file.parent().unwrap().to_string_lossy(),
            &sln.file.to_string_lossy(),
            &sln.is_valid_utf8.to_string(),
            &sln.linked_projects.len().to_string(),
            &sln.orphaned_projects.len().to_string()
        ])?;
    }

    wtr.flush()?;
    Ok(())
}

pub fn write_projects(projects: &[Project]) -> AnalysisResult<()> {
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
                proj.file.parent_as_str(),
                &proj.file.filename_as_str(),
                &proj.is_valid_utf8.to_string(),
                proj.output_type.as_str(),
                proj.xml_doc.as_str(),
                &proj.tt_file.to_string(),
                &proj.target_frameworks.join(","),
                &proj.embedded_debugging.to_string(),
                &proj.linked_solution_info.to_string(),
                &proj.auto_generate_binding_redirects.to_string(),
                proj.test_framework.as_str(),
                &proj.uses_specflow.to_string(),
                proj.web_config.as_str(),
                proj.app_config.as_str(),
                proj.app_settings_json.as_str(),
                proj.package_json.as_str(),
                proj.packages_config.as_str(),
                proj.project_json.as_str(),
                &pkg.name,
                pkg.class.as_str(),
                &pkg.version,
                &pkg.development.to_string(),
                &pkg.is_preview().to_string(),
                &proj.file.to_string_lossy(),
            ])?;
        }
    }

    Ok(())
}
