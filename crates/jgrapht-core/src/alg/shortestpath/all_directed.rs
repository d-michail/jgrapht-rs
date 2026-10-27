use crate::graph::{DirectedGraph, Graph, GraphExt};
use crate::walk::GraphWalk;
use crate::{GraphError, Result};
use std::collections::{HashMap, HashSet, VecDeque};

/// Decides whether an edge may be appended to a partial path.
///
/// This is the Rust counterpart of JGraphT's `PathValidator`. The validator is deliberately not
/// called for zero-length paths.
pub trait PathValidator<G: Graph> {
    fn is_valid_path(&self, partial_path: &GraphWalk<G::Vertex, G::Edge>, edge: &G::Edge) -> bool;
}

impl<G, F> PathValidator<G> for F
where
    G: Graph,
    F: Fn(&GraphWalk<G::Vertex, G::Edge>, &G::Edge) -> bool,
{
    fn is_valid_path(&self, partial_path: &GraphWalk<G::Vertex, G::Edge>, edge: &G::Edge) -> bool {
        self(partial_path, edge)
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct AllowAllPaths;

impl<G: Graph> PathValidator<G> for AllowAllPaths {
    fn is_valid_path(&self, _: &GraphWalk<G::Vertex, G::Edge>, _: &G::Edge) -> bool {
        true
    }
}

/// Enumerates directed paths between one or more sources and targets.
///
/// As in Java's `AllDirectedPaths`, preprocessing decorates every usable edge with its minimum
/// remaining hop distance to a target. An optional forward BFS applies the exact sandwich prune
/// before enumeration. The preprocessing is `O(V + E)`; enumeration is necessarily proportional
/// to the number and total length of returned paths.
pub struct AllDirectedPaths<'g, G: DirectedGraph, P = AllowAllPaths> {
    graph: &'g G,
    path_validator: P,
    forward_pruning: bool,
}

impl<'g, G: DirectedGraph> AllDirectedPaths<'g, G, AllowAllPaths> {
    pub fn new(graph: &'g G) -> Self {
        Self {
            graph,
            path_validator: AllowAllPaths,
            forward_pruning: true,
        }
    }

    pub fn with_validator<P>(graph: &'g G, path_validator: P) -> AllDirectedPaths<'g, G, P>
    where
        P: PathValidator<G>,
    {
        AllDirectedPaths {
            graph,
            path_validator,
            forward_pruning: true,
        }
    }
}

impl<G: DirectedGraph, P: PathValidator<G>> AllDirectedPaths<'_, G, P> {
    pub fn set_forward_pruning(&mut self, forward_pruning: bool) {
        self.forward_pruning = forward_pruning;
    }

    pub fn is_forward_pruning(&self) -> bool {
        self.forward_pruning
    }

    pub fn get_all_paths(
        &self,
        source: &G::Vertex,
        target: &G::Vertex,
        simple_paths_only: bool,
        max_path_length: Option<usize>,
    ) -> Result<Vec<GraphWalk<G::Vertex, G::Edge>>> {
        self.get_all_paths_between(
            std::iter::once(source),
            std::iter::once(target),
            simple_paths_only,
            max_path_length,
        )
    }

    pub fn get_all_paths_between<'a>(
        &self,
        source_vertices: impl IntoIterator<Item = &'a G::Vertex>,
        target_vertices: impl IntoIterator<Item = &'a G::Vertex>,
        simple_paths_only: bool,
        max_path_length: Option<usize>,
    ) -> Result<Vec<GraphWalk<G::Vertex, G::Edge>>>
    where
        G::Vertex: 'a,
    {
        if !simple_paths_only && max_path_length.is_none() {
            return Err(GraphError::InvalidArgument(
                "non-simple path searches require a maximum path length".into(),
            ));
        }
        let sources: Vec<_> = source_vertices.into_iter().collect();
        let targets: HashSet<_> = target_vertices.into_iter().collect();
        if sources.is_empty() || targets.is_empty() {
            return Ok(Vec::new());
        }
        if sources.iter().any(|v| !self.graph.contains_vertex(v))
            || targets.iter().any(|v| !self.graph.contains_vertex(v))
        {
            return Err(GraphError::VertexNotFound);
        }

        let forward = self
            .forward_pruning
            .then(|| self.forward_distances(&sources, max_path_length));
        let edge_to_target =
            self.backward_edge_distances(&targets, forward.as_ref(), max_path_length);
        Ok(self.generate_paths(
            &sources,
            &targets,
            simple_paths_only,
            max_path_length,
            &edge_to_target,
        ))
    }

    fn forward_distances<'a>(
        &'a self,
        sources: &[&'a G::Vertex],
        max_path_length: Option<usize>,
    ) -> HashMap<&'a G::Vertex, usize> {
        let mut distance = HashMap::new();
        let mut queue = VecDeque::new();
        for &source in sources {
            if distance.insert(source, 0).is_none() {
                queue.push_back(source);
            }
        }
        while let Some(vertex) = queue.pop_front() {
            let d = distance[vertex];
            if max_path_length.is_some_and(|bound| d >= bound) {
                continue;
            }
            for (_, child) in self.graph.outgoing_of(vertex) {
                if !distance.contains_key(child) {
                    distance.insert(child, d + 1);
                    queue.push_back(child);
                }
            }
        }
        distance
    }

    fn backward_edge_distances<'a>(
        &'a self,
        targets: &HashSet<&'a G::Vertex>,
        forward: Option<&HashMap<&'a G::Vertex, usize>>,
        max_path_length: Option<usize>,
    ) -> HashMap<&'a G::Edge, usize> {
        let mut edge_distance = HashMap::new();
        if max_path_length == Some(0) {
            return edge_distance;
        }
        let mut vertex_distance = HashMap::new();
        let mut queue = VecDeque::new();
        for &target in targets {
            if forward.is_none_or(|f| f.contains_key(target))
                && vertex_distance.insert(target, 0).is_none()
            {
                queue.push_back(target);
            }
        }
        while let Some(vertex) = queue.pop_front() {
            let child_distance = vertex_distance[vertex] + 1;
            for edge in self.graph.in_edges_of(vertex) {
                let source = self.graph.edge_source(edge).expect("edge in graph");
                if let Some(forward) = forward {
                    let Some(&source_distance) = forward.get(source) else {
                        continue;
                    };
                    if max_path_length.is_some_and(|bound| source_distance > bound - child_distance)
                    {
                        continue;
                    }
                }
                edge_distance
                    .entry(edge)
                    .and_modify(|d| *d = (*d).min(child_distance))
                    .or_insert(child_distance);
                let improve = vertex_distance
                    .get(source)
                    .is_none_or(|&old| child_distance < old);
                if improve {
                    vertex_distance.insert(source, child_distance);
                    if max_path_length.is_none_or(|bound| child_distance < bound) {
                        queue.push_back(source);
                    }
                }
            }
        }
        edge_distance
    }

    fn generate_paths(
        &self,
        sources: &[&G::Vertex],
        targets: &HashSet<&G::Vertex>,
        simple_paths_only: bool,
        max_path_length: Option<usize>,
        edge_to_target: &HashMap<&G::Edge, usize>,
    ) -> Vec<GraphWalk<G::Vertex, G::Edge>> {
        let mut complete = Vec::new();
        let mut incomplete: VecDeque<GraphWalk<G::Vertex, G::Edge>> = VecDeque::new();

        for &source in sources {
            if targets.contains(source) {
                complete.push(GraphWalk::single(source.clone()));
            }
            if max_path_length == Some(0) {
                continue;
            }
            for (edge, target) in self.graph.outgoing_of(source) {
                let partial = GraphWalk::empty();
                if !self.path_validator.is_valid_path(&partial, edge) {
                    continue;
                }
                let path = GraphWalk::new(
                    vec![source.clone(), target.clone()],
                    vec![edge.clone()],
                    self.graph.edge_weight(edge).unwrap_or(1.0),
                );
                if targets.contains(target) {
                    complete.push(path.clone());
                }
                if edge_to_target.contains_key(edge)
                    && max_path_length.is_none_or(|bound| bound > 1)
                {
                    incomplete.push_back(path);
                }
            }
        }

        while let Some(path) = incomplete.pop_front() {
            let length = path.edges.len();
            let leaf = path.vertices.last().expect("non-empty path");
            let path_vertices: HashSet<_> = simple_paths_only
                .then(|| path.vertices.iter().collect())
                .unwrap_or_default();
            for (edge, child) in self.graph.outgoing_of(leaf) {
                let Some(&remaining) = edge_to_target.get(edge) else {
                    continue;
                };
                if max_path_length.is_some_and(|bound| remaining + length > bound) {
                    continue;
                }
                if simple_paths_only && path_vertices.contains(child) {
                    continue;
                }
                if !self.path_validator.is_valid_path(&path, edge) {
                    continue;
                }
                let mut next = path.clone();
                next.vertices.push(child.clone());
                next.edges.push(edge.clone());
                next.weight += self.graph.edge_weight(edge).unwrap_or(1.0);
                if targets.contains(child) {
                    complete.push(next.clone());
                }
                if max_path_length.is_none_or(|bound| next.edges.len() < bound) {
                    // Java uses addFirst here; this is a depth-first queue and bounds live memory.
                    incomplete.push_front(next);
                }
            }
        }
        complete
    }
}
