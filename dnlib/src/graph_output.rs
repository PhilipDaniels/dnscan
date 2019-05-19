use crate::errors::DnLibResult;
use crate::graph::Node;
use std::collections::HashSet;
use std::fs::File;
use std::io::{BufWriter, Write};

use petgraph::prelude::*;
use petgraph::visit::{IntoNodeReferences, IntoEdgeReferences};

// This is how you get a node given an index. You have to use &...
// which is about as clear as mud from the 'documentation'.
// let source_node = &graph[edge.source()];
// let target_node = &graph[edge.target()];

pub fn write_project_dot_file(
    graph: &StableGraph<Node, u8>,
    removed_edges: &HashSet<(usize, usize)>,
) -> DnLibResult<()> {
    let file = File::create("analysis.dot")?;
    let mut writer = BufWriter::new(file);
    write_project_dot(&mut writer, graph, removed_edges)
}

pub fn write_project_dot<W>(
    writer: &mut W,
    graph: &StableGraph<Node, u8>,
    removed_edges: &HashSet<(usize, usize)>,
) -> DnLibResult<()>
where
    W: Write,
{
    writeln!(writer, "digraph {{")?;

    for (node_idx, node_ref) in graph.node_references() {
        writeln!(writer, "    {} [label=\"{}\",{}]",
            node_idx.index(), node_ref, node_ref.dot_attributes())?;
    }

    println!("Removed edges = {:?}", removed_edges);

    for edge in graph.edge_references() {
        let source_node_idx = edge.source().index();
        let target_node_idx = edge.target().index();
        writeln!(writer, "    {} -> {}", source_node_idx, target_node_idx)?;
    }

    for edge in removed_edges {
        writeln!(
            writer,
            "    {} -> {} [color=red,style=dotted,penwidth=2]",
            edge.0, edge.1
        )?;
    }

    writeln!(writer, "}}")?;

    Ok(())
}

