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
use dnlib::{timer, stimer, finish};

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

    let _tmr = stimer!("Directory Analysis");
    let dir = options.input_directory.as_ref().unwrap();
    let configuration = Configuration::new(dir);
    let configuration = merge_configuration_and_options(configuration, options);

    //println!("Effective config={:#?}", configuration);

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

    let tmr = timer!("Calculate project graph and redundant projects");
    let mut individual_graphs = make_project_graphs(&analysis);
    let individual_graphs = individual_graphs
        .iter_mut()
        .map(|(sln, graph)| {
            let removed_edges = graph.transitive_reduction();
            (sln, graph, removed_edges)
        })
        .collect::<Vec<_>>();

    let mut overall_graph = make_project_graph(&analysis, GraphFlags::PROJECTS);
    let removed_edges = overall_graph.transitive_reduction();
    let redundant_projects = convert_nodes_to_projects(&overall_graph, &removed_edges);
    finish!(tmr, "Found {} redundant project relationships", removed_edges.len());


    let _tmr = timer!("Write output files");
    csv_output::write_solutions(&configuration.output_directory, &analysis)?;
    csv_output::write_solutions_to_projects(&configuration.output_directory, &analysis)?;
    csv_output::write_projects_to_packages(&configuration.output_directory, &analysis)?;
    // We could probably figure out the overall set of redundant projects from the individual graphs,
    // but this is the way I did it originally, and for now it's good enough.
    csv_output::write_projects_to_child_projects(&configuration.output_directory, &analysis, &redundant_projects)?;

    dnlib::graph_output::write_project_dot_file2(
        &configuration.output_directory,
        &std::path::PathBuf::from("dnscan.dot"),
        &overall_graph,
        &removed_edges)?;

    for (sln, graph, removed_edges) in individual_graphs {
        dnlib::graph_output::write_project_dot_file2(
            &configuration.output_directory,
            &std::path::PathBuf::from(sln.file_info.path.file_name().unwrap()),
            &graph,
            &removed_edges)?;
    }

    Ok(())
}

fn merge_configuration_and_options(mut config: Configuration, options: Options) -> Configuration {
    if let Some(dir) = options.output_directory {
        config.output_directory = dir;
    }

    if let Some(dir) = options.input_directory {
        config.input_directory = dir;
    }

    if config.output_directory.is_relative() {
        let tmp = config.output_directory;
        config.output_directory = config.input_directory.clone();
        config.output_directory.push(tmp);
    }

    config
}
