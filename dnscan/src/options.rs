use clap::{App, Arg};
use std::path::PathBuf;

#[derive(Debug, Default)]
/// The command line options.
pub struct Options {
    pub verbose: bool,
    pub dump_config: bool,
    pub dir: Option<PathBuf>,
    pub output_directory: Option<PathBuf>,
}

pub fn get_options() -> Options {
    let matches = App::new("dotnet scan")
        .version("0.1")
        .author("Philip Daniels <philip.daniels1971@gmail.com")
        .about("Scans .Net projects and cleans or analyzes them")
        .arg(
            Arg::with_name("verbose")
                .short("v")
                .help("Be verbose (prints messages about what is being done)"),
        )
        .arg(
            Arg::with_name("dump-example-config")
                .short("x")
                .help("Prints the default configuration to stdout (for use as the basis of a custom configuration file")
                .conflicts_with_all(&["DIR", "verbose"]),
        )
        .arg(
            Arg::with_name("output-directory")
                .short("o")
                .help("Specifies the output directory where CSV and graphs will be written. Can be relative or absolute.")
        )
        .arg(
            Arg::with_name("DIR")
                .help("Specifies the directory to start scanning from")
        )
        .get_matches();

    Options {
        verbose: matches.is_present("verbose"),
        dump_config: matches.is_present("dump-example-config"),
        dir: matches.value_of("DIR").map(|d| Some(PathBuf::from(d))).unwrap_or_default(),
        output_directory: matches.value_of("output-directory").map(|d| Some(PathBuf::from(d))).unwrap_or_default(),
    }
}
