use clap::{App, Arg};
use dnlib::io::{PathExtensions, make_path_under_home_dir};
use rayon::prelude::*;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use termcolor::{Color, ColorChoice, ColorSpec, StandardStreamLock, WriteColor};
use walkdir::{DirEntry, WalkDir};

#[derive(Debug, Default)]
pub struct Options {
    pub clean: bool,
    pub vsclean: bool,
    pub gitclean: bool,
    pub gitdelete: bool,
    pub verbose: bool,
    pub prompt_for_confirmation: bool,
    pub dir: PathBuf,
}

pub fn get_options() -> Options {
    let matches = App::new("dotnet scan")
        .version("0.1")
        .author("Philip Daniels <philip.daniels1971@gmail.com")
        .about("Scans .Net projects and cleans them")
        .arg(Arg::with_name("clean").short("c").help("Cleans the solution by removing bin, obj, packages folder, testresults, and SolutionInfo.cs"))
        .arg(Arg::with_name("vsclean").short("m").help("Cleans the Visual Studio MEF cache, web cache, and JetBrains folders"))
        .arg(Arg::with_name("gitclean").short("x").help("Cleans the folder by running 'git clean -xfd'"))
        .arg(Arg::with_name("gitdelete").short("g").help("Removes the actual .git folders. Use at your peril - removes source control!"))
        .arg(Arg::with_name("verbose").short("v").help("Be verbose (prints messages about what is being done)"))
        .arg(Arg::with_name("prompt").short("p").help("Prompt for confirmation before deleting things (irrelevant for analyze)"))
        .arg(Arg::with_name("DIR").help("Specifies the directory to start scanning from. Defaults to the current directory").required(true))
        .get_matches();

    Options {
        clean: matches.is_present("clean"),
        vsclean: matches.is_present("vsclean"),
        gitclean: matches.is_present("gitclean"),
        gitdelete: matches.is_present("gitdelete"),
        verbose: matches.is_present("verbose"),
        prompt_for_confirmation: matches.is_present("prompt"),
        dir: PathBuf::from(matches.value_of("DIR").unwrap()),
    }
}

fn main() {
    let options = get_options();

    if !options.dir.exists() {
        eprintln!("The directory {:?} does not exist.", options.dir);
        std::process::exit(1);
    }

    run_clean(options);
}

#[derive(Debug, Default)]
pub struct PathsToClean {
    pub git_dirs: Vec<PathBuf>,
    pub sln_dirs_to_delete: Vec<PathBuf>,
    pub other_dirs_to_delete: Vec<PathBuf>,
    pub files_to_delete: Vec<PathBuf>,
}

impl PathsToClean {
    pub fn sort(&mut self) {
        self.git_dirs.sort();
        self.sln_dirs_to_delete.sort();
        self.other_dirs_to_delete.sort();
        self.files_to_delete.sort();
    }

    pub fn is_empty(&self) -> bool {
        self.git_dirs.is_empty()
            && self.sln_dirs_to_delete.is_empty()
            && self.other_dirs_to_delete.is_empty()
            && self.files_to_delete.is_empty()
    }
}

pub fn run_clean(options: Options) {
    let paths = get_paths_of_interest(&options);
    if paths.is_empty() {
        return;
    };

    let mut do_delete = true;
    if options.prompt_for_confirmation {
        print_deletion_candidates(&options, &paths);
        do_delete = get_confirmation();
    }

    if do_delete {
        println!("Deleting...");
        delete_candidates(paths, options.verbose);
    }
}

#[derive(Debug)]
enum DeletionType {
    File,
    Directory,
    DirectoryContents,
}

fn delete_candidates(paths: PathsToClean, verbose: bool) {
    let f_iterator = paths
        .files_to_delete
        .into_iter()
        .map(|p| (DeletionType::File, p));

    let git_iterator = paths
        .git_dirs
        .into_iter()
        .map(|p| (DeletionType::Directory, p));

    let sd_iterator = paths
        .sln_dirs_to_delete
        .into_iter()
        .map(|p| (DeletionType::Directory, p));

    let od_iterator = paths
        .other_dirs_to_delete
        .into_iter()
        .map(|p| (DeletionType::DirectoryContents, p));

    // Rayon will not work with a chained iterator, so we have to collect
    // everything into a Vec, unfortunately.
    let all_deletions: Vec<_> = f_iterator
        .chain(git_iterator)
        .chain(sd_iterator)
        .chain(od_iterator)
        .collect();

    all_deletions
        .par_iter()
        .for_each(|(del_type, path)| match del_type {
            DeletionType::File => match delete_file(path, verbose) {
                Err(e) => eprintln!("Could not delete file {:?}, err = {:?}", path, e),
                _ => {}
            },
            DeletionType::Directory => match delete_directory(path, verbose) {
                Err(e) => eprintln!("Could not delete directory {:?}, err = {:?}", path, e),
                _ => {}
            },
            DeletionType::DirectoryContents => match delete_directory_contents(path, verbose) {
                Err(e) => eprintln!(
                    "Could not delete contents of directory {:?}, err = {:?}",
                    path, e
                ),
                _ => {}
            },
        });
}

fn get_paths_of_interest(options: &Options) -> PathsToClean {
    let mut paths = PathsToClean::default();

    // Get paths from the solution (i.e. the -c, -x and -g options).
    let walker = WalkDir::new(&options.dir);
    for _entry in walker
        .into_iter()
        .filter_entry(|e| continue_walking_sln(e, &options, &mut paths))
    {}

    // Now the vsclean options (-m).
    if options.vsclean {
        // This is the MEF component cache. VS will rebuild it on restart.
        let path = make_path_under_home_dir("AppData/Local/Microsoft/VisualStudio");
        let walker = WalkDir::new(path);
        for _entry in walker
            .into_iter()
            .filter_entry(|e| continue_walking_mef(e, &mut paths))
        {}

        // JetBrains caches.
        let path = make_path_under_home_dir("AppData/Local/JetBrains");
        let walker = WalkDir::new(path);
        for _entry in walker
            .into_iter()
            .filter_entry(|e| continue_walking_jetbrains(e, &mut paths))
        {}

        let path = make_path_under_home_dir("AppData/Microsoft/WebsiteCache");
        if path.exists() {
            paths.other_dirs_to_delete.push(path);
        }
    }

    paths.sort();
    paths
}

fn continue_walking_sln(entry: &DirEntry, options: &Options, paths: &mut PathsToClean) -> bool {
    let path = entry.path();

    // Taking 'paths' as a parameter allows us to accumulate these directories without recursing into them.

    if path.is_git_dir() {
        if options.gitdelete {
            paths.git_dirs.push(path.to_owned());
        }
        return false;
    }

    // These are the standard directories we want to clean.
    if path.is_bin_or_obj_dir() || path.is_packages_dir() || path.is_test_results_dir() {
        if options.clean {
            paths.sln_dirs_to_delete.push(path.to_owned());
        }
        return false;
    }

    // Remaining directories we don't want to walk into.
    if path.is_hidden_dir() || path.is_node_modules_dir() {
        return false;
    }

    // Various files we typically want to remove.
    if path.is_solution_info_file()
        || path.is_version_out_file()
        || path.is_suo_file()
        || path.is_upgrade_log_file()
    {
        paths.files_to_delete.push(path.to_owned());
    }

    true
}

fn continue_walking_mef(entry: &DirEntry, paths: &mut PathsToClean) -> bool {
    let path = entry.path();

    if path.is_mef_cache_dir() {
        paths.other_dirs_to_delete.push(path.to_owned());
        return false;
    }

    true
}

fn continue_walking_jetbrains(entry: &DirEntry, paths: &mut PathsToClean) -> bool {
    let path = entry.path();

    if path.is_mef_cache_dir() {
        paths.other_dirs_to_delete.push(path.to_owned());
        return false;
    }

    true
}

fn write_in_color(stream: &mut StandardStreamLock, msg: &str, color: Color) {
    stream
        .set_color(ColorSpec::new().set_fg(Some(color)))
        .unwrap();
    writeln!(stream, "{}", msg).unwrap();
    stream.reset().unwrap();
}

fn write_heading(stream: &mut StandardStreamLock, msg: &str) {
    write_in_color(stream, msg, Color::Cyan);
}

fn write_risky_heading(stream: &mut StandardStreamLock, msg: &str) {
    write_in_color(stream, msg, Color::Red);
}

fn print_deletion_candidates(options: &Options, paths: &PathsToClean) {
    let stdout = termcolor::StandardStream::stdout(ColorChoice::Always);
    let mut stdoutlock = stdout.lock();

    if !paths.other_dirs_to_delete.is_empty() {
        write_heading(&mut stdoutlock, "Delete these miscellaneous directories?");
        for p in &paths.other_dirs_to_delete {
            writeln!(stdoutlock, "    {}", p.display()).unwrap();
        }
    }

    if !paths.sln_dirs_to_delete.is_empty() {
        write_heading(&mut stdoutlock, "Delete these solution directories?");
        for p in &paths.sln_dirs_to_delete {
            writeln!(stdoutlock, "    {}", p.display()).unwrap();
        }
    }

    if !paths.files_to_delete.is_empty() {
        write_heading(&mut stdoutlock, "Delete these files?");
        for p in &paths.files_to_delete {
            writeln!(stdoutlock, "    {}", p.display()).unwrap();
        }
    }

    if !paths.git_dirs.is_empty() {
        let msg = match (options.gitclean, options.gitdelete) {
            (true, true) => "Run 'git clean -xfd' on these directories AND THEN DELETE THEM?",
            (true, false) => "Run 'git clean -xfd' on these directories?",
            (false, true) => "DELETE THESE GIT DIRECTORIES?",
            (false, false) => unreachable!(),
        };

        write_risky_heading(&mut stdoutlock, msg);
        for p in &paths.git_dirs {
            writeln!(stdoutlock, "    {}", p.display()).unwrap();
        }
    }

    stdoutlock.flush().unwrap();
}

fn get_confirmation() -> bool {
    let stdout = termcolor::StandardStream::stdout(ColorChoice::Always);
    let mut stdoutlock = stdout.lock();
    write_in_color(
        &mut stdoutlock,
        "Press Y to confirm deletion, any other key cancels: ",
        Color::Cyan,
    );
    stdoutlock.flush().unwrap();

    let gc = getch::Getch::new();
    let do_delete = match gc.getch() {
        Ok(key) => key == b'Y' || key == b'y',
        Err(_) => false,
    };

    //    println!();
    do_delete
}

fn delete_file(path: &Path, verbose: bool) -> io::Result<()> {
    if path.is_file() {
        make_deletable(path)?;
        fs::remove_file(path)?;
        if verbose {
            println!("Deleted file {}", path.display());
        }
    }
    Ok(())
}

fn delete_directory(path: &Path, verbose: bool) -> io::Result<()> {
    if path.is_dir() {
        delete_directory_contents(path, false)?;
        make_deletable(path)?;
        fs::remove_dir(path)?;
        if verbose {
            println!("Deleted directory {}", path.display());
        }
    }
    Ok(())
}

fn delete_directory_contents(path: &Path, verbose: bool) -> io::Result<()> {
    if path.is_dir() {
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                delete_directory(&path, false)?;
            } else {
                delete_file(&path, false)?;
            }
        }

        if verbose {
            println!("Deleted contents of {}", path.display());
        }
    }
    Ok(())
}

fn make_deletable(path: &Path) -> io::Result<()> {
    let mut perms = fs::metadata(path)?.permissions();
    perms.set_readonly(false);
    fs::set_permissions(path, perms)
}
