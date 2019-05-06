mod csv_output;
mod errors;
mod options;

use errors::AnalysisResult;
use options::Options;
use dnlib::prelude::*;
use std::fs;
use std::collections::{HashSet, HashMap};

fn main() {
    let options = options::get_options();

    if options.dump_config {
        Configuration::dump_defaults();
        std::process::exit(0);
    }

    match options.dir.as_ref() {
        Some(d) => if !d.exists() || !d.is_dir() {
            eprintln!("The directory {:?} does not exist or is a file.", d);
            std::process::exit(1);
        },
        None => {
            eprintln!("Please specify a DIR to scan");
            std::process::exit(1);
        }
    }

    let configuration = Configuration::new(options.dir.as_ref().unwrap());

    let start = std::time::Instant::now();
    run_analysis_and_print_result(&options, &configuration);
    if options.verbose {
        println!("Total Time = {:?}", start.elapsed());
    }
}

pub fn run_analysis_and_print_result(options: &Options, configuration: &Configuration) {
    match run_analysis(options, configuration) {
        Ok(_) => if options.verbose { println!("Analysis completed without errors") },
        Err(e) => {
            eprintln!("Error occurred {:#?}", e);
            std::process::exit(1);
        }
    }
}

pub fn run_analysis(options: &Options, configuration: &Configuration) -> AnalysisResult<()> {
    let dir = options.dir.as_ref().unwrap();
    let analysis = Analysis::new(&dir, configuration)?;
    if analysis.is_empty() {
        println!(
            "Did not find any .sln or .csproj files under {}",
            dir.display()
        );
    }

    if options.verbose {
        println!("Discovered files in {:?}", analysis.disk_walk_duration);
        println!("Loaded {} solutions in {:?}", analysis.num_solutions(), analysis.solution_load_duration);
        println!("Loaded {} linked projects and {} orphaned projects in {:?}",
            analysis.num_linked_projects(),
            analysis.num_orphaned_projects(),
            analysis.project_load_duration);
    }

    let start = std::time::Instant::now();
    csv_output::write_files(&analysis)?;
    if options.verbose {
        println!("CSV files written in {:?}", start.elapsed());
    }


    let start = std::time::Instant::now();
    let analysis_graph = make_analysis_graph(&analysis);
    let analysis_dot = Dot::with_config(&analysis_graph, &[Config::EdgeNoLabel]);
    fs::write("analysis.dot", analysis_dot.to_string())?;
    if options.verbose {
        println!("analysis.dot written in {:?}", start.elapsed());
    }

    // We start at the 'bottom' of the graph, and work up.
    let mut sorted_nodes = toposort(&analysis_graph, None).unwrap();
    sorted_nodes.reverse();

    // Effectively: Project -> HashSet<Project>
    let mut projects_to_children = HashMap::new();

    for node_id in sorted_nodes.iter().take(5) {
        let node = analysis_graph[*node_id];
        if let Node::Project(p) = node {
            println!("  Project Node = {:?}, {:?}", p, node_id);

            let mut all_child_projects = HashSet::<NodeIndex>::new();

            let children = analysis_graph.neighbors(*node_id);
            let children_2 = children.clone();
            for child_idx in children {
                let child = analysis_graph[child_idx];
                println!("    child = {:?}, {:?}", child, child_idx);
                match projects_to_children.get(&child_idx) {
                    Some(cx) => {
                        all_child_projects.extend(cx);
                    },
                    None => {},
                }
            }

            // If p has a direct reference R to anything in all_child_projects, then
            // R is transitively redundant.
            for child_idx in children_2 {
                if all_child_projects.contains(&child_idx) {
                    let child = analysis_graph[child_idx];
                    println!("    redundant child = {:?}, {:?}", child, child_idx);
                }
            }

            // Must insert child itself as basis case. Otherwise entire
            // set ends up empty...
            all_child_projects.insert(*node_id);
            // Store this for use when calculating for later nodes.
            projects_to_children.insert(node_id, all_child_projects);
        }
    }

    println!("Mapping = {:#?}", projects_to_children);

    Ok(())
}


#[cfg(test)]
/// Perform a [transitive reduction](https://en.wikipedia.org/wiki/Transitive_reduction)
/// on `graph`. A new graph is returned.
fn transitive_reduction<N, E, Ty, Ix>(graph: &Graph<N, E, Ty, Ix>)
-> (Graph<N, E, Ty, Ix>, u8)
where
    Ty: EdgeType,
    Ix: IndexType

{
    let mut reduced_graph = Graph::<N, E, Ty, Ix>::with_capacity(
        graph.node_count(), graph.edge_count()
    );


    // Sort the nodes topologically. We will work through the nodes
    // from leafs (terminal nodes) ascending the graph until we
    // reach the root(s). Note that petgraph puts the roots *first*
    // in this vector, so we have to iterate it backwards (which is
    // just as quick as iterating it forwards, and avoids a sort.)
    let sorted_nodes = toposort(graph, None).expect("Graph must not have cycles.");



    (reduced_graph, 0)
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn tred_empty_graph() {
        let graph = Graph::<&str, ()>::new();
        let reduction = transitive_reduction(&graph).0;
        assert_eq!(reduction.node_count(), 0);
        assert_eq!(reduction.edge_count(), 0);
    }

    #[test]
    pub fn tred_singleton_graph_without_edge() {
        let mut graph = Graph::<&str, ()>::new();
        graph.add_node("a");
        let reduction = transitive_reduction(&graph).0;
        assert_eq!(reduction.node_count(), 1);
        assert_eq!(reduction.edge_count(), 0);
    }

    /*
    toposort requires the graph to be a DAG.
    #[test]
    pub fn tred_singleton_graph_with_edge() {
        let mut graph = Graph::<&str, ()>::new();
        let a_idx = graph.add_node("a");
        input.add_edge(a_idx, a_idx, ());
        let reduction = transitive_reduction(&graph).0;
        assert_eq!(reduction.node_count(), 1);
        assert_eq!(reduction.edge_count(), 1);
    }
    */


}