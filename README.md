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

- Should really get rid of tp and have separate sets of tests for Windows and Linux?
- Config file for package classification regexes
- Tests for pkg version variations
- Make project analysis run in parallel again
- Figure out how to run Rayon with 1 thread for debugging.
- Tests for mentioned projects are completely inadequate.
- Convert to use Unicase where possible.
- Better settings for rustfmt
- Git info extraction
- Build a REST API ('serve mode') for getting at the data
  - Consider some sort of 'reporting data structure'
- A web site built on the REST API
- Implement Fix mode
  - Remove redundant project references
  - Remove redundant NuGet package references
  - Scan source for redundant NuGet packages
  - Remove redundant Assembly references
