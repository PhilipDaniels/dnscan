mod csv_output;
mod errors;
mod options;
mod graph_output;

use errors::AnalysisResult;
use options::Options;
use dnlib::prelude::*;
use std::collections::HashSet;

fn main() {
    let options = options::get_options();

    if options.dump_config {
        Configuration::dump_defaults();
        std::process::exit(0);
    }

    match options.dir.as_ref() {
        Some(d) => if !d.exists() || !d.is_dir() {
            eprintln!("The directory {:?} does not exist or is a file.", d);
            std::process::exit(1);
        },
        None => {
            eprintln!("Please specify a DIR to scan");
            std::process::exit(1);
        }
    }

    let configuration = Configuration::new(options.dir.as_ref().unwrap());

    let start = std::time::Instant::now();
    run_analysis_and_print_result(&options, &configuration);
    if options.verbose {
        println!("Total Time = {:?}", start.elapsed());
    }
}

pub fn run_analysis_and_print_result(options: &Options, configuration: &Configuration) {
    match run_analysis(options, configuration) {
        Ok(_) => if options.verbose { println!("Analysis completed without errors") },
        Err(e) => {
            eprintln!("Error occurred {:#?}", e);
            std::process::exit(1);
        }
    }
}

pub fn run_analysis(options: &Options, configuration: &Configuration) -> AnalysisResult<()> {
    let dir = options.dir.as_ref().unwrap();
    let analysis = Analysis::new(&dir, configuration)?;
    if analysis.is_empty() {
        println!(
            "Did not find any .sln or .csproj files under {}",
            dir.display()
        );
    }

    if options.verbose {
        println!("Discovered files in {:?}", analysis.disk_walk_duration);
        println!("Loaded {} solutions in {:?}", analysis.num_solutions(), analysis.solution_load_duration);
        println!("Loaded {} linked projects and {} orphaned projects in {:?}",
            analysis.num_linked_projects(),
            analysis.num_orphaned_projects(),
            analysis.project_load_duration);
    }

    let start = std::time::Instant::now();
    let mut analysis_graph = make_analysis_graph(&analysis);
    let removed_edges = transitive_reduction_stable(&mut analysis_graph);
    if options.verbose {
        println!("Project graph and redundant projects found in {:?}", start.elapsed());
    }

    let start = std::time::Instant::now();
    csv_output::write_solutions(&analysis)?;
    csv_output::write_solutions_to_projects(&analysis)?;
    csv_output::write_projects_to_packages(&analysis)?;

    let redundant_project_relationships = convert_removed_edges_to_node_references(&analysis_graph, &removed_edges);
    csv_output::write_projects_to_child_projects(&analysis, &redundant_project_relationships)?;
    graph_output::write_project_dot_file(&analysis_graph, &removed_edges)?;

    if options.verbose {
        println!("Output files written in {:?}", start.elapsed());
    }

    Ok(())
}
