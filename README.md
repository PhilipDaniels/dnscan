# dnlib, dnscan and dnclean

A small Rust library with two binaries for analyzing .Net solutions.

- dnlib - commmon code library for the two binaries
- dnscan - scan dotnet projects and make fancy graphs and statistics
- dnclean - cleans bin, obj and other folders from a dotnet solution folder

## TODO
- Find packages that are redundant
    - First level is to find redundant installs within a solution (caused by project references brining them in)
    - Second level is to analyze the NuGet package itself, find redundancies within a project and then transitively
    - Want to keep a cache of NuGet package metadata in ~/.dnscan
- Find what-uses-what
    - We already have projects_to_packages
    - Really care about what is the compilation order of our ecosystem.
- Tests for mentioned projects are completely inadequate.
- Build a REST API ('serve mode') for getting at the data
  - Consider some sort of 'reporting data structure'
  - Allow the ability to filter the graph down to a single solution even if you have
    analyzed the entire directory
- A web site built on the REST API
- Handle VB and F#.


## Links for the NuGet stuff

https://docs.microsoft.com/en-us/nuget/api/overview
https://github.com/NuGet/Home/issues/6393
https://joelverhagen.github.io/NuGetUndocs/?http#endpoint-get-a-single-package
https://stackoverflow.com/questions/34958908/where-can-i-find-documentation-for-the-nuget-v3-api
