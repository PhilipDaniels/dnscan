use crate::analysis::{Analysis, SolutionDirectory, Solution, Project};
use crate::io::PathExtensions;
use std::collections::{HashMap, HashSet};
use std::fmt;
use bitflags::bitflags;

use petgraph::prelude::*;
use petgraph::EdgeType;
use petgraph::graph::{IndexType};
use petgraph::visit::GetAdjacencyMatrix;
use fixedbitset::FixedBitSet;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum Node<'a> {
    Analysis(&'a Analysis),
    SolutionDirectory(&'a SolutionDirectory),
    Solution(&'a Solution),
    Project(&'a Project),
}

/// This library generates directed graphs of `Node` with indexes that are stable
/// across removals and unweighted edges.
pub type DnGraph<'a> = StableGraph<Node<'a>, (), Directed, u32>;

bitflags! {
    pub struct GraphFlags: u32 {
        const ANALYSIS_ROOT = 0b00000001;
        const SOLUTION_DIRECTORY = 0b00000010;
        const PROJECTS = 0b00000100;
        const PACKAGES = 0b00001000;
        const ALL = Self::ANALYSIS_ROOT.bits |
                    Self::SOLUTION_DIRECTORY.bits |
                    Self::PROJECTS.bits |
                    Self::PACKAGES.bits;
    }
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
            Node::Analysis(ref an) => write!(f, "{} (root dir)", an.root_path.display()),
            Node::SolutionDirectory(ref sd) => write!(f, "{} (sln dir)", sd.directory.file_stem_as_str()),
            Node::Solution(ref sln) => write!(f, "{}", sln.file_info.path.file_stem_as_str()),
            Node::Project(ref proj) => write!(f, "{}", proj.file_info.path.file_stem_as_str()),
        }
    }
}

impl<'a> Node<'a> {
    pub fn dot_attributes(&self) -> &'static str {
        use crate::enums::ProjectOwnership;

        // [color=blue,,fontcolor=red]"

        // We are using X11 colors.
        // https://graphviz.gitlab.io/_pages/doc/info/shapes.html#d:style
        // https://graphviz.gitlab.io/_pages/doc/info/attrs.html#k:color

        match *self {
            Node::Analysis(_) => "shape=invhouse,style=filled,fillcolor=gold,penwidth=3",
            Node::SolutionDirectory(_) => "shape=octagon,style=filled,fillcolor=turquoise,penwidth=3",
            Node::Solution(_) => "shape=ellipse,style=filled,fillcolor=grey,penwidth=3",
            Node::Project(ref p) if p.ownership == ProjectOwnership::Orphaned => "shape=rectangle,style=\"filled,rounded\",fillcolor=firebrick1",
            Node::Project(_) => "shape=rectangle,style=rounded",
        }
    }
}

/// Construct a graph of the entire analysis results.
/// There are no relationships between the solutions in this graph.
/// It can be used to find redundant project references.
pub fn make_project_graph(
    analysis: &Analysis,
    graph_flags: GraphFlags
    )
-> DnGraph
{
    let mut graph = DnGraph::default();

    let analysis_node_idx = if graph_flags.contains(GraphFlags::ANALYSIS_ROOT) {
        Some(graph.add_node(Node::Analysis(analysis)))
    } else {
        None
    };

    for sd in &analysis.solution_directories {
        let sd_node_idx = if graph_flags.contains(GraphFlags::SOLUTION_DIRECTORY) {
            Some(graph.add_node(Node::SolutionDirectory(&sd)))
        } else {
            None
        };

        if let Some(analysis_node_idx) = analysis_node_idx {
            if let Some(sd_node_idx) = sd_node_idx {
                graph.add_edge(analysis_node_idx, sd_node_idx, ());
            }
        }

        for sln in &sd.solutions {
            let sln_node_idx = graph.add_node(Node::Solution(&sln));
            if let Some(sd_node_idx) = sd_node_idx {
                graph.add_edge(sd_node_idx, sln_node_idx, ());
            }

            // Get all projects and add them to the graph as nodes.
            // We will work out the edges in a moment.
            let mut proj_node_mapping = HashMap::new();
            for proj in &sln.projects {
                let proj_node_idx = graph.add_node(Node::Project(&proj));
                proj_node_mapping.insert(proj, proj_node_idx);
            }

            // Now we have to work out all the edges. A project is either (a)
            // referenced by other projects or (b) referenced only by the sln,
            // i.e. it is a top-level deliverable.
            for proj in &sln.projects {
                let parent_projects = proj.get_parent_projects(sln);
                if parent_projects.is_empty() {
                    graph.add_edge(sln_node_idx, proj_node_mapping[proj], ());
                } else {
                    for parent in parent_projects {
                        graph.add_edge(proj_node_mapping[parent], proj_node_mapping[proj], ());
                    }
                }
            }
        }
    }

    graph
}

/// Construct a set of graphs, one graph for each solution in the analysis results.
pub fn make_project_graphs(analysis: &Analysis) -> HashMap<&Solution, DnGraph> {
    let mut results = HashMap::default();

    for sd in &analysis.solution_directories {
        for sln in &sd.solutions {
            let mut graph = DnGraph::default();
            let sln_node_idx = graph.add_node(Node::Solution(&sln));
            //add_proj(&mut graph, &sln, sln_node_idx);

            // COMMON
            // Get all projects and add them to the graph as nodes.
            // We will work out the edges in a moment.
            let mut proj_node_mapping = HashMap::new();
            for proj in &sln.projects {
                let proj_node_idx = graph.add_node(Node::Project(&proj));
                proj_node_mapping.insert(proj, proj_node_idx);
            }

            // Now we have to work out all the edges. A project is either (a)
            // referenced by other projects or (b) referenced only by the sln,
            // i.e. it is a top-level deliverable.
            for proj in &sln.projects {
                let parent_projects = proj.get_parent_projects(sln);
                if parent_projects.is_empty() {
                    graph.add_edge(sln_node_idx, proj_node_mapping[proj], ());
                } else {
                    for parent in parent_projects {
                        graph.add_edge(proj_node_mapping[parent], proj_node_mapping[proj], ());
                    }
                }
            }
            // COMMON

            results.insert(sln, graph);
        }
    }

    results
}

// fn add_proj<'a>(graph: &'a mut DnGraph<'a>, sln: &'a Solution, sln_node_idx: NodeIndex<u32>)
// {
//     // Get all projects and add them to the graph as nodes.
//     // We will work out the edges in a moment.
//     let mut proj_node_mapping = HashMap::new();
//     for proj in &sln.projects {
//         let proj_node_idx = graph.add_node(Node::Project(&proj));
//         proj_node_mapping.insert(proj, proj_node_idx);
//     }

//     // Now we have to work out all the edges. A project is either (a)
//     // referenced by other projects or (b) referenced only by the sln,
//     // i.e. it is a top-level deliverable.
//     for proj in &sln.projects {
//         let parent_projects = proj.get_parent_projects(sln);
//         if parent_projects.is_empty() {
//             graph.add_edge(sln_node_idx, proj_node_mapping[proj], ());
//         } else {
//             for parent in parent_projects {
//                 graph.add_edge(proj_node_mapping[parent], proj_node_mapping[proj], ());
//             }
//         }
//     }
// }


// TODO: Only the method needs to be generic? But that causes a shadowing when we impl it.
pub trait TredExtensions<Ix> {
    fn get_path_matrix(&self) -> GraphMatrix;
    fn transitive_reduction(&mut self) -> HashSet<(NodeIndex<Ix>, NodeIndex<Ix>)>;
}

impl<N, E, Ty, Ix> TredExtensions<Ix> for StableGraph<N, E, Ty, Ix>
where
    Ty: EdgeType,
    Ix: IndexType,
{
    /// Returns the path matrix for a graph. This has a 1 in any cell where there
    /// is a path, of any length, between 2 nodes.
    fn get_path_matrix(&self) -> GraphMatrix {
        let mut matrix = GraphMatrix::new(self.adjacency_matrix(), self.node_count());
        matrix.calculate_path_matrix();
        matrix
    }

    fn transitive_reduction(&mut self) -> HashSet<(NodeIndex<Ix>, NodeIndex<Ix>)> {
        let mut matrix = self.get_path_matrix();
        matrix.calculate_transitive_reduction_of_path_matrix();

        // Now remove edges if they are not in the transitive reduction.
        let edge_indices: Vec<_> = self.edge_indices().collect();

        let mut removed_edges = HashSet::new();
        for e in edge_indices {
            if let Some((i, j)) = self.edge_endpoints(e) {
                if !matrix.contains(i.index(), j.index()) {
                    self.remove_edge(e);
                    removed_edges.insert((i, j));
                }
            }
        }

        removed_edges
    }
}

/// Helper type because the API of the FixedBitSet is appalling
/// for this use-case.
#[derive(Debug, PartialEq, Eq)]
pub struct GraphMatrix {
    bitset: FixedBitSet,
    num_columns: usize
}

impl GraphMatrix {
    fn new(bitset: FixedBitSet, num_columns: usize) -> Self {
        Self { bitset, num_columns }
    }

    #[inline]
    fn idx(&self, x: usize, y: usize) -> usize {
        x * self.num_columns + y
    }

    #[inline]
    fn contains(&self, x: usize, y: usize) -> bool {
        let idx = self.idx(x, y);
        self.bitset.contains(idx)
    }

    fn set(&mut self, x: usize, y: usize, enabled: bool) {
        let idx = self.idx(x, y);
        self.bitset.set(idx, enabled)
    }

    /// Convert the adjacency matrix (also known as an edge matrix) to a path matrix.
    /// The adjacency matrix has a 1 if there is an edge from a to b; the path matrix
    /// has a 1 if there is a path (by any route) from a to b.
    /// The path matrix therefore represents the transitive closure of the graph.
    fn calculate_path_matrix(&mut self) {
        // The edge matrix (aka adjacency matrix) is square with nc * nc elements.
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
        for i in 0..self.num_columns {
            for j in 0..self.num_columns {
                // Ignore the diagonals.
                if i == j { continue };

                if self.contains(j, i) {
                    for k in 0..self.num_columns {
                        if !self.contains(j, k) {
                            let flag = self.contains(i, k);
                            self.set(j, k, flag);
                        }
                    }
                }
            }
        }
    }

    fn calculate_transitive_reduction_of_path_matrix(&mut self) {
        // From https://stackoverflow.com/questions/1690953/transitive-reduction-algorithm-pseudocode
        // See Harry Hsu. "An algorithm for finding a minimal equivalent graph of a digraph.", Journal
        // of the ACM, 22(1):11-16, January 1975. The simple cubic algorithm below (using an N x N path
        // matrix) suffices for DAGs, but Hsu generalizes it to cyclic graphs.
        for j in 0..self.num_columns {
            for i in 0..self.num_columns {
                if self.contains(i, j) {
                    for k in 0..self.num_columns {
                        if self.contains(j, k) {
                            self.set(i, k, false);
                        }
                    }
                }
            }
        }
    }
}

pub fn get_node_project<'a>(graph: &'a DnGraph, node_index: NodeIndex) -> &'a Project {
    let node = &graph[node_index];

    match node {
        Node::Project(project) => return project,
        _ => panic!("Asked for a project on a non-project node")
    }
}

pub fn convert_nodes_to_projects<'a>(graph: &'a DnGraph, node_pairs: &HashSet<(NodeIndex, NodeIndex)>)
-> HashSet<(&'a Project, &'a Project)>
{
    node_pairs
    .iter()
    .map(|(source, target)| (get_node_project(graph, *source), get_node_project(graph, *target)))
    .collect()
}


#[cfg(test)]
mod tests {
    use super::*;

    fn make_bitset(nc: usize, bits: usize) -> FixedBitSet {
        let mut bitset = FixedBitSet::with_capacity(nc * nc);

        for n in 0..bitset.len() {
            let bit = (bits >> n)  & 1;
            if bit == 1 {
                bitset.set(n, true);
            }
        }

        bitset
    }


    mod tred_tests {
        use super::*;

        // For the contains_edge expressions, we are assuming that the nodes are
        // added in the order a,b,c...It makes the code a lot simpler.
        #[test]
        pub fn tred_graph_a() {
            let mut graph = graph_a();
            graph.transitive_reduction();
            assert_eq!(graph.edge_count(), 0);
        }

        #[test]
        pub fn tred_graph_ab() {
            let mut graph = graph_ab();
            graph.transitive_reduction();
            assert_eq!(graph.edge_count(), 0);
        }

        #[test]
        pub fn tred_graph_ab_edges_ab() {
            let mut graph = graph_ab_edges_ab();
            graph.transitive_reduction();
            assert_eq!(graph.edge_count(), 1);
            assert!(graph.find_edge(0.into(), 1.into()).is_some());
        }

        #[test]
        pub fn tred_graph_abc_edges_ac() {
            let mut graph = graph_abc_edges_ac();
            graph.transitive_reduction();
            assert_eq!(graph.edge_count(), 1);
            assert!(graph.find_edge(0.into(), 2.into()).is_some());
        }

        #[test]
        pub fn tred_graph_abc_edges_ac_bc() {
            let mut graph = graph_abc_edges_ac_bc();
            graph.transitive_reduction();
            assert_eq!(graph.edge_count(), 2);
            assert!(graph.find_edge(0.into(), 2.into()).is_some());
            assert!(graph.find_edge(1.into(), 2.into()).is_some());
        }

        // #[test]
        // pub fn tred_graph_abc_edges_ac_bc_ca() {
        //     // This graph has a cycle a <-> c, and tred is not well defined.
        //     // We should return a Cycle error in this case.
        //     let mut graph = graph_abc_edges_ac_bc_ca();
        //     graph.transitive_reduction();
        //     assert_eq!(graph.edge_count(), 3);
        //     assert!(graph.find_edge(0.into(), 2.into()).is_some());
        //     assert!(graph.find_edge(1.into(), 2.into()).is_some());
        //     assert!(graph.find_edge(2.into(), 0.into()).is_some());
        // }

        #[test]
        pub fn tred_graph_abc_edges_ab_bc() {
            let mut graph = graph_abc_edges_ab_bc();
            graph.transitive_reduction();
            assert_eq!(graph.edge_count(), 2);
            assert!(graph.find_edge(0.into(), 1.into()).is_some());
            assert!(graph.find_edge(1.into(), 2.into()).is_some());
        }

        #[test]
        pub fn tred_graph_abcdef_edges_ab_bc_cd_ce_bf() {
            let mut graph = graph_abcdef_edges_ab_bc_cd_ce_bf();
            graph.transitive_reduction();
            assert_eq!(graph.edge_count(), 5);
            assert!(graph.find_edge(0.into(), 1.into()).is_some());
            assert!(graph.find_edge(1.into(), 2.into()).is_some());
            assert!(graph.find_edge(2.into(), 3.into()).is_some());
            assert!(graph.find_edge(2.into(), 4.into()).is_some());
            assert!(graph.find_edge(1.into(), 5.into()).is_some());
        }

        // None of the above actually remove any edges.

        #[test]
        pub fn tred_graph_abc_edges_ab_bc_ac() {
            let mut graph = graph_abc_edges_ab_bc_ac();
            graph.transitive_reduction();
            assert_eq!(graph.edge_count(), 2);
            assert!(graph.find_edge(0.into(), 1.into()).is_some());
            assert!(graph.find_edge(1.into(), 2.into()).is_some());
        }

        #[test]
        pub fn tred_graph_wikipedia() {
            let mut graph = graph_wikipedia();
            graph.transitive_reduction();
            assert_eq!(graph.edge_count(), 5);
            assert!(graph.find_edge(0.into(), 1.into()).is_some());
            assert!(graph.find_edge(0.into(), 2.into()).is_some());
            assert!(graph.find_edge(1.into(), 3.into()).is_some());
            assert!(graph.find_edge(2.into(), 3.into()).is_some());
            assert!(graph.find_edge(3.into(), 4.into()).is_some());
        }

        #[test]
        pub fn tred_graph_abcd_edges_ab_ac_bd_cd() {
            let mut graph = graph_abcd_edges_ab_ac_bd_cd();
            graph.transitive_reduction();
            assert_eq!(graph.edge_count(), 4);
            assert!(graph.find_edge(0.into(), 1.into()).is_some());
            assert!(graph.find_edge(0.into(), 2.into()).is_some());
            assert!(graph.find_edge(1.into(), 3.into()).is_some());
            assert!(graph.find_edge(2.into(), 3.into()).is_some());
        }

    }

    fn graph_a() -> StableGraph<&'static str, ()> {
        let mut graph = StableGraph::<&str, ()>::new();
        graph.add_node("a");
        graph
    }

    fn graph_ab() -> StableGraph<&'static str, ()> {
        let mut graph = StableGraph::<&str, ()>::new();
        graph.add_node("a");
        graph.add_node("b");
        graph
    }

    fn graph_ab_edges_ab() -> StableGraph<&'static str, ()> {
        let mut graph = StableGraph::<&str, ()>::new();
        let a = graph.add_node("a");
        let b = graph.add_node("b");
        graph.add_edge(a, b, ());
        graph
    }

    fn graph_abc_edges_ac() -> StableGraph<&'static str, ()> {
        let mut graph = StableGraph::<&str, ()>::new();
        let a = graph.add_node("a");
        graph.add_node("b");
        let c = graph.add_node("c");
        graph.add_edge(a, c, ());
        graph
    }

    fn graph_abc_edges_ac_bc() -> StableGraph<&'static str, ()> {
        let mut graph = StableGraph::<&str, ()>::new();
        let a = graph.add_node("a");
        let b = graph.add_node("b");
        let c = graph.add_node("c");
        graph.add_edge(a, c, ());
        graph.add_edge(b, c, ());
        graph
    }

    // TODO: Be able to detect cycles during the tred and return an error.
    // fn graph_abc_edges_ac_bc_ca() -> StableGraph<&'static str, ()> {
    //     // This graph has a cycle. TRED is not well-defined for it.
    //     let mut graph = StableGraph::<&str, ()>::new();
    //     let a = graph.add_node("a");
    //     let b = graph.add_node("b");
    //     let c = graph.add_node("c");
    //     graph.add_edge(a, c, ());
    //     graph.add_edge(b, c, ());
    //     graph.add_edge(c, a, ());
    //     graph
    // }

    fn graph_abc_edges_ab_bc() -> StableGraph<&'static str, ()> {
        let mut graph = StableGraph::<&str, ()>::new();
        let a = graph.add_node("a");
        let b = graph.add_node("b");
        let c = graph.add_node("c");
        graph.add_edge(a, b, ());
        graph.add_edge(b, c, ());
        graph
    }

    fn graph_abcdef_edges_ab_bc_cd_ce_bf() -> StableGraph<&'static str, ()> {
        let mut graph = StableGraph::<&str, ()>::new();
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
        graph
    }

    fn graph_abc_edges_ab_bc_ac() -> StableGraph<&'static str, ()> {
        let mut graph = StableGraph::<&str, ()>::new();
        let a = graph.add_node("a");
        let b = graph.add_node("b");
        let c = graph.add_node("c");
        graph.add_edge(a, b, ());
        graph.add_edge(b, c, ());
        graph.add_edge(a, c, ());
        graph
    }

    fn graph_wikipedia() -> StableGraph<&'static str, ()> {
        // The graph from the Wikipedia article at
        // https://en.wikipedia.org/wiki/Transitive_reduction
        let mut graph = StableGraph::<&str, ()>::new();
        let a = graph.add_node("a");
        let b = graph.add_node("b");
        let c = graph.add_node("c");
        let d = graph.add_node("d");
        let e = graph.add_node("e");
        graph.add_edge(a, b, ());
        graph.add_edge(a, c, ());
        graph.add_edge(a, d, ());
        graph.add_edge(a, e, ());
        graph.add_edge(b, d, ());
        graph.add_edge(c, d, ());
        graph.add_edge(c, e, ());
        graph.add_edge(d, e, ());
        graph
    }

    fn graph_abcd_edges_ab_ac_bd_cd() -> StableGraph<&'static str, ()> {
        // The graph from the Wikipedia article at
        // https://en.wikipedia.org/wiki/Transitive_reduction
        let mut graph = StableGraph::<&str, ()>::new();
        let a = graph.add_node("a");
        let b = graph.add_node("b");
        let c = graph.add_node("c");
        let d = graph.add_node("d");
        graph.add_edge(a, b, ());
        graph.add_edge(a, c, ());
        graph.add_edge(b, d, ());
        graph.add_edge(c, d, ());
        graph
    }


    mod path_matrix_tests {
        use super::*;

        fn assert_matrix(matrix: &GraphMatrix, bits: usize) {
            let bitset = make_bitset(matrix.num_columns, bits);
            let expected_matrix = GraphMatrix::new(bitset, matrix.num_columns);
            assert_eq!(matrix, &expected_matrix);
        }

        #[test]
        pub fn cpm_graph_a() {
            let graph = graph_a();
            let pm = graph.get_path_matrix();
            assert_matrix(&pm, 0);
        }

        #[test]
        pub fn cpm_graph_ab() {
            let graph = graph_ab();
            let pm = graph.get_path_matrix();
            assert_matrix(&pm, 0);
        }

        #[test]
        pub fn cpm_graph_ab_edges_ab() {
            let graph = graph_ab_edges_ab();
            let pm = graph.get_path_matrix();
            assert_matrix(&pm, 0b_10);
        }

        #[test]
        pub fn cpm_graph_abc_edges_ac() {
            let graph = graph_abc_edges_ac();
            let pm = graph.get_path_matrix();
            assert_matrix(&pm, 0b_100);
        }

        #[test]
        pub fn cpm_graph_abc_edges_ac_bc() {
            let graph = graph_abc_edges_ac_bc();
            let pm = graph.get_path_matrix();
            assert_matrix(&pm, 0b_100_100);
        }

        // #[test]
        // pub fn cpm_graph_abc_axc_bxc_cxa() {
        //     // Don't test with cycles. The graph builder fn is commented out.
        //     let graph = graph_abc_edges_ac_bc_ca();
        //     let pm = graph.get_path_matrix();

        //     // For edge matrix:
        //     //        c b a
        //     //      c 0 0 1        (c,a)
        //     //      b 1 0 0        (b,c)
        //     //      a 1 0 0        (a,c)

        //     // For path matrix:
        //     //        c b a
        //     //      c 1 0 1        (c,a)  (c,c)
        //     //      b 1 0 1        (b,c)  (b,a)
        //     //      a 1 0 1        (a,c)  (a,a)
        //     //
        //     // The cycle between a and c adds these extra edges.
        //     assert_matrix(&pm, graph.node_count(), 0b_101_101_101);
        // }

        #[test]
        pub fn cpm_graph_abc_axb_bxc() {
            let graph = graph_abc_edges_ab_bc();
            let pm = graph.get_path_matrix();
            assert_matrix(&pm, 0b_100_110);
        }

        #[test]
        pub fn cpm_graph_abcdef() {
            let graph = graph_abcdef_edges_ab_bc_cd_ce_bf();
            let pm = graph.get_path_matrix();

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

            assert_matrix(&pm, 0b_011000_111100_111110);
        }
    }
}
