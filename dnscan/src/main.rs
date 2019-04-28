mod csv_output;
mod errors;
mod options;

use errors::AnalysisResult;
use options::Options;
use dnlib::prelude::*;
//use dnlib::graph::*;
use std::fs;
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
    csv_output::write_files(&analysis)?;
    if options.verbose {
        println!("CSV files written in {:?}", start.elapsed());
    }


    let start = std::time::Instant::now();
    let analysis_graph = make_analysis_graph(&analysis);
    let analysis_dot = Dot::with_config(&analysis_graph, &[Config::EdgeNoLabel]);
    fs::write("analysis.dot", analysis_dot.to_string())?;
    if options.verbose {
        println!("analysis.dot written in {:?}", start.elapsed());
    }

    let mst = min_spanning_tree(&analysis_graph);
    let mut mst_edges = HashSet::new();
    for elem in mst {
        match elem {
            Element::Edge { source, target, .. } => { mst_edges.insert((source, target)); },
            _ => {},
        }
    }

    for edge in analysis_graph.edge_references() {
        //println!("Checking edge {:?}", edge);
        let e = (edge.source().index(), edge.target().index());
        if !mst_edges.contains(&e) {
            println!("  Redundant edge = {:?}", e);
        }
    }

    Ok(())
}

