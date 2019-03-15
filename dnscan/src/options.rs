use clap::{App, Arg};
use std::path::PathBuf;

#[derive(Debug, Default)]
pub struct Options {
    pub verbose: bool,
    pub dir: PathBuf,
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
            Arg::with_name("DIR")
                .help("Specifies the directory to start scanning from")
                .required(true),
        )
        .get_matches();

    Options {
        verbose: matches.is_present("verbose"),
        dir: PathBuf::from(matches.value_of("DIR").unwrap()),
    }
}
