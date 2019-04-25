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

- Analyze_files: merge find_linked_sln and find_orphaned_slns into one fn
- Hide internal errors, convert to strings.
- Find projects that are redundant
- Find packages that are redundant
    - First level is to find redundant installs within a solution (caused by project references brining them in)
    - Second level is to analyze the NuGet package itself, find redundancies within a project and then transitively
- Find what-uses-what
    - We already have projects_to_packages
    - Really care about what is the compilation order of our ecosystem.

- Tests for mentioned projects are completely inadequate.
- Better settings for rustfmt
    - Longer lines!
    - Preserve vertical space, do not wrap everything
    - Maybe look in ripgrep repo
- Build a REST API ('serve mode') for getting at the data
  - Consider some sort of 'reporting data structure'
- A web site built on the REST API
