use std::collections::HashSet;
use std::fs::File;
use std::io::{Write, BufWriter};
use dnlib::prelude::*;
use crate::errors::AnalysisResult;

// This is how you get a node given an index. You have to use &...
// which is about as clear as mud from the 'documentation'.
//let source_node = &graph[edge.source()];
//let target_node = &graph[edge.target()];


pub fn write_project_dot_file(
    graph: &StableGraph<Node, u8>,
    removed_edges: &HashSet<(usize, usize)>) -> AnalysisResult<()>
{

    let file = File::create("analysis.dot")?;
    let mut writer = BufWriter::new(file);

    // TODO: Consider highlighting test projects, exes etc.
    writeln!(writer, "digraph {{")?;

    for (node_idx, node_ref) in graph.node_references() {
        writeln!(writer, "    {} [label=\"{}\"]", node_idx.index(), node_ref)?;
    }

    println!("Removed edges = {:?}", removed_edges);

    for edge in graph.edge_references() {
        let source_node_idx = edge.source().index();
        let target_node_idx = edge.target().index();
        writeln!(writer, "    {} -> {}", source_node_idx, target_node_idx)?;
    }

    for edge in removed_edges {
        writeln!(writer, "    {} -> {} [color=red;style=dotted]", edge.0, edge.1)?;
    }

    writeln!(writer, "}}")?;

    let analysis_dot = Dot::with_config(&graph, &[Config::EdgeNoLabel]);
    std::fs::write("default.dot", analysis_dot.to_string())?;

    Ok(())
}



// TODO: Implement a 'name' method for a node, e.g. to trim the .csproj.
// TODO: Use different shapes for different node types, including orphaned projects.
// TODO: Write redundant projects to csv also.
// TODO: Write to a writer, not a file.
