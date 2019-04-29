use crate::analysis::{Analysis, SolutionDirectory, Solution, Project};
use crate::io::PathExtensions;
use std::collections::HashMap;
use std::fmt;

pub use petgraph::prelude::*;
pub use petgraph::dot::*;
pub use petgraph::algo::*;
pub use petgraph::data::*;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum Node<'a> {
    Analysis(&'a Analysis),
    SolutionDirectory(&'a SolutionDirectory),
    Solution(&'a Solution),
    Project(&'a Project),
}

impl<'a> fmt::Debug for Node<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match *self {
            Node::Analysis(ref an) => write!(f, "{}", an.root_path.display()),
            Node::SolutionDirectory(ref sd) => write!(f, "{}", sd.directory.display()),
            Node::Solution(ref sln) => write!(f, "{}", sln.file_info.path.display()),
            Node::Project(ref proj) => write!(f, "{:?}", proj),
        }
    }
}

impl<'a> fmt::Display for Node<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Node::Analysis(ref an) => write!(f, "{} (analysis)", an.root_path.display()),
            Node::SolutionDirectory(ref sd) => write!(f, "{} (sln dir)", sd.directory.filename_as_str()),
            Node::Solution(ref sln) => write!(f, "{}", sln.file_info.path.filename_as_str()),
            Node::Project(ref proj) => write!(f, "{}", proj.file_info.path.filename_as_str()),
        }
    }
}


/// Construct a graph of the entire analysis results.
/// There are no relationships between the solutions in this graph.
/// It can be used to find redundant project references.
/// TODO: Add packages
/// TODO: Want to do slndir->slndir analysis (more edges)
pub fn make_analysis_graph(analysis: &Analysis) -> Graph<Node, u8>
{
    let mut graph = Graph::default();
    let analysis_node = Node::Analysis(analysis);
    let analysis_node_idx = graph.add_node(analysis_node);

    for sd in &analysis.solution_directories {
        let sd_node = Node::SolutionDirectory(&sd);
        let sd_node_idx = graph.add_node(sd_node);
        graph.add_edge(analysis_node_idx, sd_node_idx, 0);

        for sln in &sd.solutions {
            let sln_node = Node::Solution(&sln);
            let sln_node_idx = graph.add_node(sln_node);
            graph.add_edge(sd_node_idx, sln_node_idx, 0);

            // Get all projects and add them to the graph as nodes.
            // We will work out the edges in a moment.
            let mut proj_node_mapping = HashMap::new();
            for proj in &sln.projects {
                let proj_node = Node::Project(&proj);
                let proj_node_idx = graph.add_node(proj_node);
                proj_node_mapping.insert(proj, proj_node_idx);
            }

            // Now we have to work out all the edges. A project is either (a)
            // referenced by other projects or (b) referenced only by the sln,
            // i.e. it is a top-level deliverable.
            for proj in &sln.projects {
                let parent_projects = proj.get_parent_projects(sln);
                if parent_projects.is_empty() {
                    graph.add_edge(sln_node_idx, proj_node_mapping[proj], 0);
                } else {
                    for parent in parent_projects {
                        graph.add_edge(proj_node_mapping[parent], proj_node_mapping[proj], 0);
                    }
                }
            }
        }
    }

    graph
}
