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

- Project things
    - pub referenced_projects: Vec<Arc<Project>>,
    - packages_require_consolidation,
    - redundant_packages_count
    - redundant_projects_count
- Analyze_files: merge find_linked_sln and find_orphaned_slns into one fn
- Analyze_files: make integration tests due to use of Path.is_dir()
    - Get rid of tp
    - Same types of tests for Windows and Linux
    - Use tempfile/tempdir?
- Git info extraction
- Tests for mentioned projects are completely inadequate.
- Better settings for rustfmt
    - Longer lines!
    - Preserve vertical space, do not wrap everything
    - Maybe look in ripgrep repo
- Build a REST API ('serve mode') for getting at the data
  - Consider some sort of 'reporting data structure'
- A web site built on the REST API
- Implement Fix mode
  - Remove redundant project references
  - Remove redundant NuGet package references
  - Scan source for redundant NuGet packages
  - Remove redundant Assembly references
