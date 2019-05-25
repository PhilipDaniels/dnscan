mod csv_output;
mod errors;
mod options;
mod configuration;

use errors::AnalysisResult;
use options::Options;
use dnlib::prelude::*;
use log::{warn};
use std::io::Write;
use chrono::{DateTime, Utc};
use env_logger::Builder;
use dnlib::{timer, stimer};

fn configure_logging() {
    let mut builder = Builder::from_default_env();
    builder.format(|buf, record| {
            let utc: DateTime<Utc> = Utc::now();

            write!(buf,
                "{:?} {} [{}] ",
                //utc.format("%Y-%m-%dT%H:%M:%S.%fZ"),
                utc,                    // same, probably faster?
                record.level(),
                record.target()         // "dnlib::timers" - defaults to same as module_path
            )?;

            match (record.file(), record.line()) {
                (Some(file), Some(line)) => write!(buf, "[{}/{}] ", file, line),
                (Some(file), None) => write!(buf, "[{}] ", file),
                (None, Some(_line)) => write!(buf, " "),
                (None, None) => write!(buf, " "),
            }?;


            writeln!(buf, "{}", record.args())
    });

    builder.init();
}

fn main() {
    configure_logging();
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

    let _tmr = stimer!("Beginning Analysis");
    let dir = options.input_directory.as_ref().unwrap();
    let configuration = Configuration::new(dir);
    let configuration = merge_configuration_and_options(configuration, options);
    run_analysis_and_print_result(&configuration);
}

pub fn run_analysis_and_print_result(configuration: &Configuration) {
    if let Err(e) = run_analysis(configuration) {
            eprintln!("Error occurred {:#?}", e);
            std::process::exit(1);
    }
}

pub fn run_analysis(configuration: &Configuration) -> AnalysisResult<()> {
    let analysis = Analysis::new(&configuration)?;
    if analysis.is_empty() {
        warn!(
            "Did not find any .sln or .csproj files under {}",
            configuration.input_directory.display()
        );
    }

    println!("Loaded {} linked projects and {} orphaned projects",
        analysis.num_linked_projects(),
        analysis.num_orphaned_projects()
        );

    let _tmr = timer!("Calculate project graph and redundant projects");
    let graph_flags = GraphFlags::PROJECTS;
    let mut analysis_graph = make_project_graph(&analysis, graph_flags);
    let removed_edges = analysis_graph.transitive_reduction();
    drop(_tmr);


    let _tmr = timer!("Write output files");
    csv_output::write_solutions(&analysis)?;
    csv_output::write_solutions_to_projects(&analysis)?;
    csv_output::write_projects_to_packages(&analysis)?;

    let redundant_projects = convert_nodes_to_projects(&analysis_graph, &removed_edges);
    csv_output::write_projects_to_child_projects(&analysis, &redundant_projects)?;
    dnlib::graph_output::write_project_dot_file(&analysis_graph, &removed_edges)?;

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
