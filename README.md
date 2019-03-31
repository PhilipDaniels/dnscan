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

- Should really get rid of tp and have separate sets of tests for Windows and Linux
  - Actually we now have a major problem running on Linux due to paths not matching
- Config file for package classification regexes
- Figure out how to run Rayon with 1 thread for debugging.
- Tests for mentioned projects are completely inadequate.
- Convert to use Unicase where possible. See if it makes a speed difference.
- Better settings for rustfmt
- Git info extraction
- Deal with

```
<PackageReference Include="Microsoft.EntityFrameworkCore">
    <Version>2.1.4</Version>
</PackageReference>
```
