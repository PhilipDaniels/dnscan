# dnscan

A small Rust library with two binaries

- dnclean - cleans bin, obj and other folders from a dotnet solution folder
- dnscan - scan dotnet projects and make fancy graphs and statistics


## To run

```
cargo run --bin dnclean [-cmxgvp]
cargo run --bin dnscan [-v]
```

## TODO
- Redo the config file system to use `~/.dnscan/config` and `~/.dnscan/packages/newtonsoft_1.3.13`
  - Remove LM-specific package classifications from config defaults
  - Add abbreviations for the graph
- Add logging.
- Find packages that are redundant
    - First level is to find redundant installs within a solution (caused by project references brining them in)
    - Second level is to analyze the NuGet package itself, find redundancies within a project and then transitively
- Find what-uses-what
    - We already have projects_to_packages
    - Really care about what is the compilation order of our ecosystem.
- Tests for mentioned projects are completely inadequate.
- Build a REST API ('serve mode') for getting at the data
  - Consider some sort of 'reporting data structure'
  - Allow the ability to filter the graph down to a single solution even if you have
    analyzed the entire directory
- A web site built on the REST API
