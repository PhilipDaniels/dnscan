use crate::errors::AnalysisResult;
use crate::solution::Solution;
use crate::project::Project;
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
        "PkgName", "PkgClass", "PkgVersion", "PkgIsDev", "PkgIsPreview"
        ])?;

    for proj in projects {
        for pkg in &proj.packages {
            wtr.write_record(&[
                proj.version.as_str(),
                &proj.file.parent().unwrap().to_string_lossy(),
                &proj.file.to_string_lossy(),
                &proj.is_valid_utf8.to_string(),
                proj.output_type.as_str(),
                proj.xml_doc.as_str(),
                &proj.tt_file.to_string(),
                &pkg.name,
                pkg.class.as_str(),
                &pkg.version,
                &pkg.development.to_string(),
                &pkg.is_preview().to_string(),
            ])?;
        }
    }

    Ok(())
}
