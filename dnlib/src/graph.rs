use crate::analysis::{Analysis, SolutionDirectory, Solution, Project};
use crate::io::PathExtensions;
use std::collections::HashMap;
use std::fmt;

pub use petgraph::prelude::*;
pub use petgraph::dot::*;
pub use petgraph::algo::*;
pub use petgraph::data::*;
// TODO: Only doing this 'pub use' so we can implement tred in dnscan.
// Should not re-export like this.
pub use petgraph::EdgeType;
pub use petgraph::graph::{IndexType};
pub use petgraph::visit::*;
pub use petgraph::visit::GetAdjacencyMatrix;

pub use fixedbitset::FixedBitSet;

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

pub trait FixedBitSetExtensions {
    fn index(x: usize, y: usize) -> usize;
}

impl FixedBitSetExtensions for FixedBitSet {
    #[inline]
    fn index(x: usize, y: usize) -> usize {
        0
    }
}

/// Convert the adjacency matrix (also known as an edge matrix) to a path matrix.
/// The adjacency matrix has a 1 if there is an edge from a to b; the path matrix
/// has a 1 if there is a path (by any route) from a to b.
/// The path matrix therefore represents the transitive closure of the graph.
#[cfg(test)]
fn calculate_path_matrix<N, E, Ty, Ix>(graph: &Graph<N, E, Ty, Ix>) -> FixedBitSet
where
    Ty: EdgeType,
    Ix: IndexType
{
    let nc = graph.node_count();
    let mut matrix = graph.adjacency_matrix();
    assert_eq!(matrix.len(), nc * nc);

    // The adjacency matrix is square with nc * nc elements.
    // An edge (a,c) is represented with 'a' on the rows and 'c' on the columns.
    //
    //        c b a
    //      c 0 0 0
    //      b 1 0 0        (b,c)
    //      a 1 0 0        (a,c)
    //
    // In this matrix, bits 2 and 5 are set, corresponding to edges (a,c) and (b,c).
    // The bits are counted from the bottom right corner (bit 0) moving leftwards and then
    // up to the end of the previous row, until we reach the top left corner (c,c).
    //
    // The nodex indexes are:
    //      a.index() == 0
    //      b.index() == 1
    //      c.index() == 2
    //
    // For edge (x,y), the element in the bitset is at x.index() * nc + y.index(),
    // Therefore, for (a,c) we have:   0 * 3 + 2 = 2
    // Therefore, for (b,c) we have:   1 * 3 + 2 = 5


    // Now convert to a path matrix.
    let calc_index = |x,y| x * nc + y;

    for i in 0..nc {
        for j in 0..nc {
            // Ignore the diagonals
            if i == j { continue };
            let ji_idx = calc_index(j, i);
            if matrix[ji_idx] {
                for k in 0..nc {
                    let jk_idx = calc_index(j, k);
                    if !matrix[jk_idx] {
                        let ik_idx = calc_index(i, k);
                        matrix.set(jk_idx, matrix.contains(ik_idx));
                    }
                }
            }
        }
    }

    matrix
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_bitset(graph: &Graph<&str, ()>, bits: usize) -> FixedBitSet {
        let nc = graph.node_count();
        let mut bitset = FixedBitSet::with_capacity(nc * nc);

        for n in 0..bitset.len() {
            let bit = (bits >> n)  & 1;
            if bit == 1 {
                bitset.set(n, true);
            }
        }

        bitset
    }

    #[test]
    pub fn cpm_graph_a() {
        let mut graph = Graph::<&str, ()>::new();
        graph.add_node("a");
        let pm = calculate_path_matrix(&graph);

        let expected = make_bitset(&graph, 0);
        assert_eq!(pm, expected, "A graph with 1 node and no edge should have a blank matrix");
    }

    #[test]
    pub fn cpm_graph_ab() {
        let mut graph = Graph::<&str, ()>::new();
        graph.add_node("a");
        graph.add_node("b");
        let pm = calculate_path_matrix(&graph);

        let expected = make_bitset(&graph, 0);
        assert_eq!(pm, expected);
    }

    #[test]
    pub fn cpm_graph_ab_axb() {
        let mut graph = Graph::<&str, ()>::new();
        let a = graph.add_node("a");
        let b = graph.add_node("b");
        graph.add_edge(a, b, ());
        let pm = calculate_path_matrix(&graph);

        let expected = make_bitset(&graph, 0b10);
        assert_eq!(pm, expected);
    }

    #[test]
    pub fn cpm_graph_abc_axc() {
        let mut graph = Graph::<&str, ()>::new();
        let a = graph.add_node("a");
        graph.add_node("b");
        let c = graph.add_node("c");
        graph.add_edge(a, c, ());
        let pm = calculate_path_matrix(&graph);

        let expected = make_bitset(&graph, 0b100);
        assert_eq!(pm, expected);
    }

    #[test]
    pub fn cpm_graph_abc_axc_bxc() {
        let mut graph = Graph::<&str, ()>::new();
        let a = graph.add_node("a");
        let b = graph.add_node("b");
        let c = graph.add_node("c");
        graph.add_edge(a, c, ());
        graph.add_edge(b, c, ());
        let pm = calculate_path_matrix(&graph);

        let expected = make_bitset(&graph, 0b100100);
        assert_eq!(pm, expected);
    }

    #[test]
    pub fn cpm_graph_abc_axc_bxc_cxa() {
        let mut graph = Graph::<&str, ()>::new();
        let a = graph.add_node("a");
        let b = graph.add_node("b");
        let c = graph.add_node("c");
        graph.add_edge(a, c, ());
        graph.add_edge(b, c, ());
        graph.add_edge(c, a, ());
        let pm = calculate_path_matrix(&graph);

        // For edge matrix:
        //        c b a
        //      c 0 0 1        (c,a)
        //      b 1 0 0        (b,c)
        //      a 1 0 0        (a,c)

        // For path matrix:
        //        c b a
        //      c 1 0 1        (c,a)  (c,c)
        //      b 1 0 1        (b,c)  (b,a)
        //      a 1 0 1        (a,c)  (a,a)
        //
        // The cycle between a and c adds these extra edges.
        let expected = make_bitset(&graph, 0b101101101);
        assert_eq!(pm, expected);
    }

    #[test]
    pub fn cpm_graph_abc_axb_bxc() {
        let mut graph = Graph::<&str, ()>::new();
        let a = graph.add_node("a");
        let b = graph.add_node("b");
        let c = graph.add_node("c");
        graph.add_edge(a, b, ());
        graph.add_edge(b, c, ());
        let pm = calculate_path_matrix(&graph);

        let expected = make_bitset(&graph, 0b100110);
        assert_eq!(pm, expected, "The pm should add edge (a,c)");
    }


    #[test]
    pub fn cpm_graph_abcdef() {
        let mut graph = Graph::<&str, ()>::new();
        let a = graph.add_node("a");
        let b = graph.add_node("b");
        let c = graph.add_node("c");
        let d = graph.add_node("d");
        let e = graph.add_node("e");
        let f = graph.add_node("f");
        graph.add_edge(a, b, ());
        graph.add_edge(b, c, ());
        graph.add_edge(c, d, ());
        graph.add_edge(c, e, ());
        graph.add_edge(b, f, ());
        let pm = calculate_path_matrix(&graph);

        // Edge matrix:
        //        f e d c b a
        //      f 0 0 0 0 0 0
        //      e 0 0 0 0 0 0
        //      d 0 0 0 0 0 0
        //      c 0 1 1 0 0 0
        //      b 1 0 0 1 0 0
        //      a 0 0 0 0 1 0

        // Path matrix:
        //        f e d c b a
        //      f 0 0 0 0 0 0
        //      e 0 0 0 0 0 0
        //      d 0 0 0 0 0 0
        //      c 0 1 1 0 0 0
        //      b 1 1 1 1 0 0
        //      a 1 1 1 1 1 0

        let expected = make_bitset(&graph, 0b011000_111100_111110);
        assert_eq!(pm, expected);
    }
}
