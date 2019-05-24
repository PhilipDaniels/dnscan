mod csv_output;
mod errors;
mod options;
mod configuration;

use errors::AnalysisResult;
use options::Options;
use dnlib::prelude::*;

fn main() {
    env_logger::init();
    let options = options::get_options();

    if options.dump_example_config {
        Configuration::dump_defaults();
        std::process::exit(0);
    }

    match options.input_directory.as_ref() {
        Some(d) => if !d.exists() || !d.is_dir() {
            eprintln!("The directory {:?} does not exist or is a file.", d);
            std::process::exit(1);
        },
        None => {
            eprintln!("Please specify a DIR to scan");
            std::process::exit(1);
        }
    }

    let configuration = Configuration::new(options.input_directory.as_ref().unwrap());
    let configuration = merge_configuration_and_options(configuration, options);

    let start = std::time::Instant::now();
    run_analysis_and_print_result(&configuration);
    println!("Total Time = {:?}", start.elapsed());
}

pub fn run_analysis_and_print_result(configuration: &Configuration) {
    match run_analysis(configuration) {
        Ok(_) => println!("Analysis completed without errors"),
        Err(e) => {
            eprintln!("Error occurred {:#?}", e);
            std::process::exit(1);
        }
    }
}

pub fn run_analysis(configuration: &Configuration) -> AnalysisResult<()> {
    let analysis = Analysis::new(&configuration)?;
    if analysis.is_empty() {
        println!(
            "Did not find any .sln or .csproj files under {}",
            configuration.input_directory.display()
        );
    }

    println!("Discovered files in {:?}", analysis.disk_walk_duration);
    println!("Loaded {} solutions in {:?}", analysis.num_solutions(), analysis.solution_load_duration);
    println!("Loaded {} linked projects and {} orphaned projects in {:?}",
        analysis.num_linked_projects(),
        analysis.num_orphaned_projects(),
        analysis.project_load_duration);

    let start = std::time::Instant::now();
    let graph_flags = GraphFlags::PROJECTS;
    let mut analysis_graph = make_project_graph(&analysis, graph_flags);
    let removed_edges = analysis_graph.transitive_reduction();
    println!("Project graph and redundant projects found in {:?}", start.elapsed());

    let start = std::time::Instant::now();
    csv_output::write_solutions(&analysis)?;
    csv_output::write_solutions_to_projects(&analysis)?;
    csv_output::write_projects_to_packages(&analysis)?;

    let redundant_projects = convert_nodes_to_projects(&analysis_graph, &removed_edges);
    csv_output::write_projects_to_child_projects(&analysis, &redundant_projects)?;
    dnlib::graph_output::write_project_dot_file(&analysis_graph, &removed_edges)?;

    println!("Output files written in {:?}", start.elapsed());

    Ok(())
}

fn merge_configuration_and_options(mut config: Configuration, options: Options) -> Configuration {
    if let Some(dir) = options.output_directory {
        config.output_directory = dir;
    }

    if let Some(dir) = options.input_directory {
        config.input_directory = dir;
    }

    config
}
