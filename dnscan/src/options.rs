use clap::{App, Arg};
use std::path::PathBuf;

#[derive(Debug, Default)]
/// The command line options.
pub struct Options {
    pub dump_example_config: bool,
    pub input_directory: Option<PathBuf>,
    pub output_directory: Option<PathBuf>,
}

pub fn get_options() -> Options {
    let matches = App::new("dnscan")
        .version("0.1")
        .author("Philip Daniels <philip.daniels1971@gmail.com")
        .about("Scans .Net projects and analyzes them")
        .arg(
            Arg::with_name("dump-example-config")
                .short("x")
                .help("Prints the default configuration to stdout (for use as the basis of a custom configuration file)")
                .conflicts_with_all(&["DIR", "verbose"]),
        )
        .arg(
            Arg::with_name("output-directory")
                .short("o")
                .long("output-directory")
                .takes_value(true)
                .help("Specifies the output directory where CSV and graphs will be written. Can be relative or absolute.")
        )
        .arg(
            Arg::with_name("input-directory")
                .help("Specifies the directory to start scanning from")
        )
        .get_matches();

    Options {
        dump_example_config: matches.is_present("dump-example-config"),
        input_directory: matches
            .value_of("input-directory")
            .map(|d| Some(PathBuf::from(d)))
            .unwrap_or_default(),
        output_directory: matches
            .value_of("output-directory")
            .map(|d| Some(PathBuf::from(d)))
            .unwrap_or_default(),
    }
}
