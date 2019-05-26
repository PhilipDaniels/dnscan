use crate::errors::DnLibResult;
use crate::graph::DnGraph;
use std::collections::HashSet;
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::{Path};

use petgraph::prelude::*;
use petgraph::visit::{IntoNodeReferences, IntoEdgeReferences};
use log::info;

pub fn write_project_dot_file(
    graph: &DnGraph,
    removed_edges: &HashSet<(NodeIndex, NodeIndex)>,
) -> DnLibResult<()> {
    let file = File::create("analysis.dot")?;
    let mut writer = BufWriter::new(file);
    write_project_dot(&mut writer, graph, removed_edges)
}

pub fn write_project_dot_file2<P: AsRef<Path>>(
    dir: P,
    filename: &Path,
    graph: &DnGraph,
    removed_edges: &HashSet<(NodeIndex, NodeIndex)>,
) -> DnLibResult<()> {

    let mut path = dir.as_ref().to_path_buf();
    fs::create_dir_all(&path)?;
    path.push(filename);
    path.set_extension("dot");

    let file = File::create(&path)?;
    let mut writer = BufWriter::new(file);
    write_project_dot(&mut writer, graph, removed_edges)?;
    info!("Wrote {:?}", path);
    Ok(())
}




pub fn write_project_dot<W>(
    writer: &mut W,
    graph: &DnGraph,
    removed_edges: &HashSet<(NodeIndex, NodeIndex)>,
) -> DnLibResult<()>
where
    W: Write,
{
    writeln!(writer, "digraph {{")?;

    for (node_idx, node_ref) in graph.node_references() {
        writeln!(writer, "    {} [label=\"{}\",{}]",
            node_idx.index(), node_ref, node_ref.dot_attributes())?;
    }

    for edge in graph.edge_references() {
        let source_node_idx = edge.source().index();
        let target_node_idx = edge.target().index();
        writeln!(writer, "    {} -> {}", source_node_idx, target_node_idx)?;
    }

    for edge in removed_edges {
        writeln!(
            writer,
            "    {} -> {} [color=red,style=dotted,penwidth=2]",
            edge.0.index(), edge.1.index()
        )?;
    }

    writeln!(writer, "}}")?;

    Ok(())
}
