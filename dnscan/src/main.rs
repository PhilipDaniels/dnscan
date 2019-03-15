mod errors;
mod find_files;
mod options;
mod project;
mod solution;

use elapsed::{measure_time};
use errors::AnalysisResult;
use options::Options;
use rayon::prelude::*;
use solution::Solution;
use project::Project;

// TODO: Write our own wrapper around println that captures the options.verbose flag.

fn main() {
    let options = options::get_options();

    if !options.dir.exists() {
        eprintln!("The directory {:?} does not exist.", options.dir);
        std::process::exit(1);
    }

    let (elapsed, _) = measure_time(|| run_analysis_and_print_result(&options));
    if options.verbose {
        println!("Total Time = {}", elapsed);
    }
}

pub fn run_analysis_and_print_result(options: &Options) {
    match run_analysis(options) {
        Ok(_) => if options.verbose { println!("Analysis completed without errors") },
        Err(e) => {
            eprintln!("Error occurred {:#?}", e);
            std::process::exit(1);
        }
    }
}

pub fn run_analysis(options: &Options) -> AnalysisResult<()> {
    let (elapsed, paths) = measure_time(|| {
        find_files::get_paths_of_interest(&options)
    });

    if paths.is_empty() {
        println!(
            "Did not find any .sln or .csproj files under {}",
            options.dir.display()
        );
    }

    println!("paths = {:#?}", paths);
 
    if options.verbose {
        println!(
            "Found {} solutions and {} projects to analyze in {}.",
            paths.sln_files.len(),
            paths.csproj_files.len(),
            elapsed
        );
    }

    let (elapsed, solutions) = measure_time(|| {
        paths.sln_files.par_iter().map(|path| {
            Solution::new(path)
        }).collect::<Vec<_>>()
    });

    if options.verbose {
        println!("{} Solutions loaded and analyzed in {}", solutions.len(), elapsed);
    }

    let (elapsed, projects) = measure_time(|| {
        paths.csproj_files.par_iter().map(|path| {
            Project::new(path)
        }).collect::<Vec<_>>()
    });

    if options.verbose {
        println!("{} Projects loaded and analyzed in {}", projects.len(), elapsed);
    }

    // Perform all single-file analysis that can be done
    //   Projects: Everything except referenced_projects
    // For each solution:
    //   Find all projects under its folder
    //   Add them to the linked or orphaned projects collections
    // For each linked project
    //   Determine the list of referenced_projects (they will all be in the same sln)

    Ok(())
}
