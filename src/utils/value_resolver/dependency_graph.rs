use anyhow::{anyhow, Result};
use petgraph::algo::toposort;
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::EdgeRef;
use std::collections::HashMap;

/// Represents a path to a value in the YAML structure (e.g., "parent.child.grandchild")
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ValuePath(pub String);

impl ValuePath {
    pub fn new(path: &str) -> Self {
        ValuePath(path.to_string())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Dependency graph for value references using petgraph.
pub struct DependencyGraph {
    graph: DiGraph<String, ()>,
    node_indices: HashMap<String, NodeIndex>,
}

impl DependencyGraph {
    pub fn new() -> Self {
        Self {
            graph: DiGraph::new(),
            node_indices: HashMap::new(),
        }
    }

    /// Gets or creates a node for the given path
    fn get_or_create_node(&mut self, path: &str) -> NodeIndex {
        if let Some(&idx) = self.node_indices.get(path) {
            idx
        } else {
            let idx = self.graph.add_node(path.to_string());
            self.node_indices.insert(path.to_string(), idx);
            idx
        }
    }

    /// Adds a node (value path) to the graph
    pub fn add_node(&mut self, path: &ValuePath) {
        self.get_or_create_node(path.as_str());
    }

    /// Adds a dependency: `from` depends on `to`
    /// This means `to` must be resolved before `from`
    pub fn add_dependency(&mut self, from: &ValuePath, to: &ValuePath) {
        let from_idx = self.get_or_create_node(from.as_str());
        let to_idx = self.get_or_create_node(to.as_str());
        // Edge direction: to -> from (to must come before from in resolution order)
        self.graph.add_edge(to_idx, from_idx, ());
    }

    /// Returns topological order for resolution, or error with cycle description.
    /// The returned order has dependencies first (values that don't depend on others).
    pub fn topological_sort(&self) -> Result<Vec<ValuePath>> {
        match toposort(&self.graph, None) {
            Ok(sorted) => {
                let paths: Vec<ValuePath> = sorted
                    .into_iter()
                    .map(|idx| ValuePath::new(&self.graph[idx]))
                    .collect();
                Ok(paths)
            }
            Err(cycle) => {
                // Extract cycle information for the error message
                let cycle_node = &self.graph[cycle.node_id()];
                let cycle_path = self.find_cycle_path(cycle.node_id());
                Err(anyhow!(
                    "Circular dependency detected in values. Cycle involves: {}",
                    cycle_path.unwrap_or_else(|| cycle_node.clone())
                ))
            }
        }
    }

    /// Finds a cycle path starting from the given node for error reporting
    fn find_cycle_path(&self, start: NodeIndex) -> Option<String> {
        let mut visited = HashMap::new();
        let mut path = Vec::new();
        self.dfs_find_cycle(start, &mut visited, &mut path)
    }

    fn dfs_find_cycle(
        &self,
        node: NodeIndex,
        visited: &mut HashMap<NodeIndex, bool>,
        path: &mut Vec<String>,
    ) -> Option<String> {
        if let Some(&in_stack) = visited.get(&node) {
            if in_stack {
                // Found cycle - find where it starts in path
                let node_name = &self.graph[node];
                if let Some(pos) = path.iter().position(|p| p == node_name) {
                    let cycle: Vec<_> = path[pos..].to_vec();
                    return Some(format!("{} -> {}", cycle.join(" -> "), node_name));
                }
            }
            return None;
        }

        visited.insert(node, true);
        path.push(self.graph[node].clone());

        for edge in self.graph.edges(node) {
            if let Some(cycle) = self.dfs_find_cycle(edge.target(), visited, path) {
                return Some(cycle);
            }
        }

        path.pop();
        visited.insert(node, false);
        None
    }

    /// Returns the number of nodes in the graph (test utility)
    #[cfg(test)]
    pub fn node_count(&self) -> usize {
        self.graph.node_count()
    }

    /// Checks if the graph is empty (test utility)
    #[cfg(test)]
    pub fn is_empty(&self) -> bool {
        self.graph.node_count() == 0
    }
}

impl Default for DependencyGraph {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_topological_sort_simple() {
        let mut graph = DependencyGraph::new();
        // c depends on b, b depends on a
        // So resolution order should be: a, b, c
        graph.add_dependency(&ValuePath::new("c"), &ValuePath::new("b"));
        graph.add_dependency(&ValuePath::new("b"), &ValuePath::new("a"));

        let order = graph.topological_sort().unwrap();
        let paths: Vec<_> = order.iter().map(|p| p.as_str()).collect();

        // a should come before b, b before c
        let a_pos = paths.iter().position(|&x| x == "a").unwrap();
        let b_pos = paths.iter().position(|&x| x == "b").unwrap();
        let c_pos = paths.iter().position(|&x| x == "c").unwrap();

        assert!(a_pos < b_pos, "a should come before b");
        assert!(b_pos < c_pos, "b should come before c");
    }

    #[test]
    fn test_topological_sort_independent_nodes() {
        let mut graph = DependencyGraph::new();
        graph.add_node(&ValuePath::new("a"));
        graph.add_node(&ValuePath::new("b"));
        graph.add_node(&ValuePath::new("c"));

        let order = graph.topological_sort().unwrap();
        assert_eq!(order.len(), 3);
    }

    #[test]
    fn test_topological_sort_diamond_dependency() {
        let mut graph = DependencyGraph::new();
        // d depends on b and c, both depend on a
        //     a
        //    / \
        //   b   c
        //    \ /
        //     d
        graph.add_dependency(&ValuePath::new("b"), &ValuePath::new("a"));
        graph.add_dependency(&ValuePath::new("c"), &ValuePath::new("a"));
        graph.add_dependency(&ValuePath::new("d"), &ValuePath::new("b"));
        graph.add_dependency(&ValuePath::new("d"), &ValuePath::new("c"));

        let order = graph.topological_sort().unwrap();
        let paths: Vec<_> = order.iter().map(|p| p.as_str()).collect();

        let a_pos = paths.iter().position(|&x| x == "a").unwrap();
        let b_pos = paths.iter().position(|&x| x == "b").unwrap();
        let c_pos = paths.iter().position(|&x| x == "c").unwrap();
        let d_pos = paths.iter().position(|&x| x == "d").unwrap();

        assert!(a_pos < b_pos, "a should come before b");
        assert!(a_pos < c_pos, "a should come before c");
        assert!(b_pos < d_pos, "b should come before d");
        assert!(c_pos < d_pos, "c should come before d");
    }

    #[test]
    fn test_cycle_detection_simple() {
        let mut graph = DependencyGraph::new();
        // a -> b -> c -> a (cycle)
        graph.add_dependency(&ValuePath::new("a"), &ValuePath::new("b"));
        graph.add_dependency(&ValuePath::new("b"), &ValuePath::new("c"));
        graph.add_dependency(&ValuePath::new("c"), &ValuePath::new("a"));

        let result = graph.topological_sort();
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("Circular dependency"),
            "Error should mention circular dependency: {}",
            err_msg
        );
    }

    #[test]
    fn test_self_reference_cycle() {
        let mut graph = DependencyGraph::new();
        // a depends on itself
        graph.add_dependency(&ValuePath::new("a"), &ValuePath::new("a"));

        let result = graph.topological_sort();
        assert!(result.is_err());
    }

    #[test]
    fn test_two_node_cycle() {
        let mut graph = DependencyGraph::new();
        // a -> b -> a
        graph.add_dependency(&ValuePath::new("a"), &ValuePath::new("b"));
        graph.add_dependency(&ValuePath::new("b"), &ValuePath::new("a"));

        let result = graph.topological_sort();
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_graph() {
        let graph = DependencyGraph::new();
        let order = graph.topological_sort().unwrap();
        assert!(order.is_empty());
    }

    #[test]
    fn test_node_count() {
        let mut graph = DependencyGraph::new();
        assert_eq!(graph.node_count(), 0);

        graph.add_node(&ValuePath::new("a"));
        assert_eq!(graph.node_count(), 1);

        graph.add_dependency(&ValuePath::new("b"), &ValuePath::new("c"));
        assert_eq!(graph.node_count(), 3);
    }

    #[test]
    fn test_is_empty() {
        let mut graph = DependencyGraph::new();
        assert!(graph.is_empty());

        graph.add_node(&ValuePath::new("a"));
        assert!(!graph.is_empty());
    }

    #[test]
    fn test_duplicate_nodes() {
        let mut graph = DependencyGraph::new();
        graph.add_node(&ValuePath::new("a"));
        graph.add_node(&ValuePath::new("a"));
        graph.add_node(&ValuePath::new("a"));

        assert_eq!(graph.node_count(), 1);
    }

    #[test]
    fn test_nested_paths() {
        let mut graph = DependencyGraph::new();
        graph.add_dependency(
            &ValuePath::new("result.message"),
            &ValuePath::new("config.greeting"),
        );
        graph.add_dependency(
            &ValuePath::new("config.greeting"),
            &ValuePath::new("base.value"),
        );

        let order = graph.topological_sort().unwrap();
        let paths: Vec<_> = order.iter().map(|p| p.as_str()).collect();

        let base_pos = paths.iter().position(|&x| x == "base.value").unwrap();
        let config_pos = paths.iter().position(|&x| x == "config.greeting").unwrap();
        let result_pos = paths.iter().position(|&x| x == "result.message").unwrap();

        assert!(base_pos < config_pos);
        assert!(config_pos < result_pos);
    }
}
