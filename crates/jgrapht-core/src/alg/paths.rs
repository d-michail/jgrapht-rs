//! Compatibility helpers backed by the faithful shortest-path classes.

use super::shortestpath::disjoint::vertex_disjoint_paths;
use super::shortestpath::{AllDirectedPaths, SuurballeKDisjointShortestPaths};
use super::util::Indexed;
use crate::Result;
use crate::graph::DirectedGraph;
use crate::walk::GraphWalk;

/// All simple directed paths from `source` to `sink`.
pub fn all_simple_paths<G: DirectedGraph>(
    graph: &G,
    source: &G::Vertex,
    sink: &G::Vertex,
    max_edges: Option<usize>,
) -> Vec<GraphWalk<G::Vertex, G::Edge>> {
    AllDirectedPaths::new(graph)
        .get_all_paths(source, sink, true, max_edges)
        .unwrap_or_default()
}

/// A negative-weight cycle, without repeating its first vertex, if one exists.
///
/// Bellman-Ford starts every vertex at distance zero, equivalently using a synthetic zero-weight
/// super-source, and therefore detects cycles in disconnected components too.
pub fn find_negative_cycle<G: DirectedGraph>(graph: &G) -> Option<Vec<G::Vertex>> {
    let indexed = Indexed::new(graph);
    let n = indexed.n();
    let mut distance = vec![0.0; n];
    let mut predecessor = vec![usize::MAX; n];
    let mut last = usize::MAX;
    for _ in 0..=n {
        last = usize::MAX;
        for vertex in 0..n {
            for &(target, edge) in &indexed.out[vertex] {
                if distance[vertex] + indexed.w[edge] < distance[target] {
                    distance[target] = distance[vertex] + indexed.w[edge];
                    predecessor[target] = vertex;
                    last = target;
                }
            }
        }
        if last == usize::MAX {
            return None;
        }
    }
    for _ in 0..n {
        last = predecessor[last];
    }
    let mut cycle = vec![last];
    let mut vertex = predecessor[last];
    while vertex != last {
        cycle.push(vertex);
        vertex = predecessor[vertex];
    }
    cycle.reverse();
    Some(indexed.vertices_of(&cycle))
}

/// Up to `k` minimum-total-weight edge-disjoint paths, via Suurballe's class.
pub fn edge_disjoint_shortest_paths<G: DirectedGraph>(
    graph: &G,
    source: &G::Vertex,
    sink: &G::Vertex,
    k: usize,
) -> Result<Vec<GraphWalk<G::Vertex, G::Edge>>> {
    SuurballeKDisjointShortestPaths::new(graph).get_paths(source, sink, k)
}

/// Up to `k` minimum-total-weight internally vertex-disjoint paths.
pub fn vertex_disjoint_shortest_paths<G: DirectedGraph>(
    graph: &G,
    source: &G::Vertex,
    sink: &G::Vertex,
    k: usize,
) -> Result<Vec<GraphWalk<G::Vertex, G::Edge>>> {
    vertex_disjoint_paths(graph, source, sink, k)
}
