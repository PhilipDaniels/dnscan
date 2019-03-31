mod csv_output;
mod errors;
mod options;

use elapsed::{measure_time};
use errors::AnalysisResult;
use options::Options;
use dnlib::prelude::*;

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
    let (elapsed, analysis) = measure_time(|| {
        AnalyzedFiles::new(&options.dir)
     });

    let analysis = analysis?;
    if analysis.is_empty() {
        println!(
            "Did not find any .sln or .csproj files under {}",
            options.dir.display()
        );
    }

    if options.verbose {
        println!("Found {} solutions, {} linked projects and {} orphaned projects in {}.",
            analysis.num_solutions(),
            analysis.num_linked_projects(),
            analysis.num_orphaned_projects(),
            elapsed);
    }

    let (elapsed, result) = measure_time(|| {
        csv_output::write_files(&analysis)
    });
    result?;

    if options.verbose {
        println!("CSV files written in {}", elapsed);
    }

    Ok(())
}

