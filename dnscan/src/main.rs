mod csv_output;
mod errors;
mod options;

use elapsed::{measure_time};
use errors::AnalysisResult;
use options::Options;
use rayon::prelude::*;
use dnlib::prelude::*;

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
    // let (elapsed, paths) = measure_time(|| {
    //     find_files::get_paths_of_interest(&options)
    // });

    // if paths.is_empty() {
    //     println!(
    //         "Did not find any .sln or .csproj files under {}",
    //         options.dir.display()
    //     );
    // }

    // if options.verbose {
    //     println!("paths = {:#?}", paths);
    // }

    // if options.verbose {
    //     println!(
    //         "Found {} solutions and {} projects to analyze in {}.",
    //         paths.sln_files.len(),
    //         paths.csproj_files.len(),
    //         elapsed
    //     );
    // }

    // let file_loader = DiskFileLoader::new();

    // let (elapsed, solutions) = measure_time(|| {
    //     paths.sln_files.par_iter().map(|path| {
    //         Solution::new(path, &file_loader)
    //     }).collect::<Vec<_>>()
    // });

    // if options.verbose {
    //     println!("{} Solutions loaded and analyzed in {}", solutions.len(), elapsed);
    // }

    // let (elapsed, projects) = measure_time(|| {
    //     paths.csproj_files.par_iter().map(|path| {
    //         Project::new(path, &paths, &file_loader)
    //     }).collect::<Vec<_>>()
    // });

    // if options.verbose {
    //     println!("{} Projects loaded and analyzed in {}", projects.len(), elapsed);
    // }

    // let (elapsed, result) = measure_time(|| {
    //     csv_output::write_files(&solutions, &projects)
    // });
    // result?;

    // if options.verbose {
    //     println!("CSV files written in {}", elapsed);
    // }

    Ok(())
}
