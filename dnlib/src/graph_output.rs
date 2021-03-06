use crate::configuration::Configuration;
use crate::errors::DnLibResult;
use crate::graph::DnGraph;
use std::collections::HashSet;
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::Path;

use log::info;
use petgraph::prelude::*;
use petgraph::visit::{IntoEdgeReferences, IntoNodeReferences};

pub fn write_project_dot_file<P: AsRef<Path>>(
    configuration: &Configuration,
    filename: P,
    graph: &DnGraph,
    removed_edges: &HashSet<(NodeIndex, NodeIndex)>,
) -> DnLibResult<()>
{
    let mut path = configuration.output_directory.clone();
    fs::create_dir_all(&path)?;
    path.push(filename);
    path.set_extension("dot");

    let file = File::create(&path)?;
    let mut writer = BufWriter::new(file);
    write_project_dot(&mut writer, configuration, graph, removed_edges)?;
    info!("Wrote {:?}", path);
    Ok(())
}

fn write_project_dot<W>(
    writer: &mut W,
    configuration: &Configuration,
    graph: &DnGraph,
    removed_edges: &HashSet<(NodeIndex, NodeIndex)>,
) -> DnLibResult<()>
where
    W: Write,
{
    writeln!(writer, "digraph {{")?;

    for (node_idx, node_ref) in graph.node_references() {
        writeln!(
            writer,
            "    {} [label=\"{}\",{}]",
            node_idx.index(),
            apply_abbreviations(node_ref.to_string(), configuration),
            node_ref.dot_attributes()
        )?;
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
            edge.0.index(),
            edge.1.index()
        )?;
    }

    writeln!(writer, "}}")?;

    Ok(())
}

fn apply_abbreviations(mut s: String, configuration: &Configuration) -> String {
    if !configuration.abbreviate_on_graphs {
        return s;
    }

    for (replacement, search_terms) in &configuration.abbreviations {
        for term in search_terms {
            s = s.replace(term , replacement);
        }
    }

    s
}