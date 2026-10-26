use super::{Dijkstra, ShortestPathAlgorithm};
use crate::Result;
use crate::graph::Graph;
use crate::view::MaskSubgraph;
use crate::walk::GraphWalk;
use std::collections::HashSet;

/// Yen's algorithm: up to `k` loopless shortest paths from `source` to `sink`, in
/// non-decreasing weight order. Requires non-negative weights.
pub fn yen_k_shortest_paths<G: Graph>(
    g: &G,
    source: &G::Vertex,
    sink: &G::Vertex,
    k: usize,
) -> Result<Vec<GraphWalk<G::Vertex, G::Edge>>> {
    let mut accepted: Vec<GraphWalk<G::Vertex, G::Edge>> = Vec::new();
    let Some(first) = Dijkstra::new(g).path(source, sink)? else {
        return Ok(accepted);
    };
    if k == 0 {
        return Ok(accepted);
    }
    accepted.push(first);
    let mut candidates: Vec<GraphWalk<G::Vertex, G::Edge>> = Vec::new();

    while accepted.len() < k {
        let last = accepted.last().unwrap().clone();
        for i in 0..last.edges.len() {
            let spur = &last.vertices[i];
            let root_edges = &last.edges[..i];
            let banned_edges: HashSet<G::Edge> = accepted
                .iter()
                .filter(|p| p.edges.len() > i && p.edges[..i] == *root_edges)
                .map(|p| p.edges[i].clone())
                .collect();
            let banned_vertices: HashSet<&G::Vertex> = last.vertices[..i].iter().collect();
            let masked = MaskSubgraph::new(
                g,
                |v: &G::Vertex| banned_vertices.contains(v),
                |e: &G::Edge| banned_edges.contains(e),
            );
            let Some(tail) = Dijkstra::new(&masked).path(spur, sink)? else {
                continue;
            };
            let root_weight: f64 = root_edges
                .iter()
                .map(|e| g.edge_weight(e).unwrap_or(1.0))
                .sum();
            let mut vertices = last.vertices[..i].to_vec();
            vertices.extend(tail.vertices);
            let mut edges = root_edges.to_vec();
            edges.extend(tail.edges);
            let cand = GraphWalk {
                vertices,
                edges,
                weight: root_weight + tail.weight,
            };
            if !candidates
                .iter()
                .chain(&accepted)
                .any(|p| p.edges == cand.edges)
            {
                candidates.push(cand);
            }
        }
        let Some(best) = (0..candidates.len())
            .min_by(|&a, &b| candidates[a].weight.total_cmp(&candidates[b].weight))
        else {
            break;
        };
        accepted.push(candidates.swap_remove(best));
    }
    Ok(accepted)
}
