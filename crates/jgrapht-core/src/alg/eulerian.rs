//! Eulerian circuits (Hierholzer).

use super::util::Indexed;
use crate::graph::Graph;
use crate::walk::GraphWalk;

/// Whether an Eulerian circuit exists: every vertex has even degree (equal in/out degree for
/// directed graphs) and all edges lie in one connected component.
pub fn is_eulerian<G: Graph>(g: &G) -> bool {
    eulerian_cycle(g).is_some()
}

/// An Eulerian circuit as a closed walk using every edge exactly once.
pub fn eulerian_cycle<G: Graph>(g: &G) -> Option<GraphWalk<G::Vertex, G::Edge>> {
    let ix = Indexed::new(g);
    let n = ix.n();
    if n == 0 {
        return None;
    }
    for v in 0..n {
        let ok = if ix.directed {
            ix.out[v].len() == ix.inn[v].len()
        } else {
            let loops = ix.out[v].iter().filter(|&&(u, _)| u == v).count();
            (ix.out[v].len() + loops) % 2 == 0
        };
        if !ok {
            return None;
        }
    }
    let start = (0..n).find(|&v| !ix.out[v].is_empty()).unwrap_or(0);
    let mut used = vec![false; ix.es.len()];
    let mut ptr = vec![0usize; n];
    let mut stack = vec![(start, None::<usize>)];
    let mut circuit = Vec::new();
    while let Some(&(v, e)) = stack.last() {
        while ptr[v] < ix.out[v].len() && used[ix.out[v][ptr[v]].1] {
            ptr[v] += 1;
        }
        if ptr[v] < ix.out[v].len() {
            let (u, k) = ix.out[v][ptr[v]];
            used[k] = true;
            stack.push((u, Some(k)));
        } else {
            circuit.push((v, e));
            stack.pop();
        }
    }
    if circuit.len() != ix.es.len() + 1 {
        return None;
    }
    circuit.reverse();
    let vertices: Vec<_> = circuit.iter().map(|&(v, _)| ix.vertex(v)).collect();
    let edges: Vec<_> = circuit
        .iter()
        .filter_map(|&(_, e)| e)
        .map(|k| ix.es[k].clone())
        .collect();
    let weight = circuit
        .iter()
        .filter_map(|&(_, e)| e)
        .map(|k| ix.w[k])
        .sum();
    Some(GraphWalk {
        start: vertices[0].clone(),
        end: vertices[vertices.len() - 1].clone(),
        vertices,
        edges,
        weight,
    })
}
