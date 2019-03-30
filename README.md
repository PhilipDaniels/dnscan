# dnscan

A small Rust library with two binaries

- dnclean - cleans bin, obj and other folders from a dotnet solution folder
- dnanalyze - scan dotnet projects and make fancy graphs and statistics
Dotnet scan -

## To run

```
cargo run --bin dnclean [-cmxgvp]
cargo run --bin dnscan [-v]
```

## TODO

- Should really get rid of tp and have separate sets of tests for Windows and Linux
- Restore all project tests
- Config file for package classification regexes
- Convert to use Unicase where possible. See if it makes a speed difference.
- Deal with

```
<PackageReference Include="Microsoft.EntityFrameworkCore">
    <Version>2.1.4</Version>
</PackageReference>
```
