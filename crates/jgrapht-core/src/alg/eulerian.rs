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
    euler_walk(g, true)
}

/// An Eulerian trail (open or closed) using every edge exactly once.
pub fn eulerian_path<G: Graph>(g: &G) -> Option<GraphWalk<G::Vertex, G::Edge>> {
    euler_walk(g, false)
}

fn euler_walk<G: Graph>(g: &G, closed: bool) -> Option<GraphWalk<G::Vertex, G::Edge>> {
    let ix = Indexed::new(g);
    let n = ix.n();
    if n == 0 {
        return None;
    }
    // vertices that must start / end a trail
    let (mut starts, mut ends, mut odd) = (Vec::new(), Vec::new(), Vec::new());
    for v in 0..n {
        if ix.directed {
            let (o, i) = (ix.out[v].len(), ix.inn[v].len());
            match o.cmp(&i) {
                std::cmp::Ordering::Equal => {}
                std::cmp::Ordering::Greater if o - i == 1 => starts.push(v),
                std::cmp::Ordering::Less if i - o == 1 => ends.push(v),
                _ => return None,
            }
        } else {
            let loops = ix.out[v].iter().filter(|&&(u, _)| u == v).count();
            if (ix.out[v].len() + loops) % 2 == 1 {
                odd.push(v);
            }
        }
    }
    let first_edge_vertex = (0..n).find(|&v| !ix.out[v].is_empty()).unwrap_or(0);
    let start = if closed {
        if !starts.is_empty() || !ends.is_empty() || !odd.is_empty() {
            return None;
        }
        first_edge_vertex
    } else if ix.directed {
        match (starts.len(), ends.len()) {
            (0, 0) => first_edge_vertex,
            (1, 1) => starts[0],
            _ => return None,
        }
    } else {
        match odd.len() {
            0 => first_edge_vertex,
            2 => odd[0],
            _ => return None,
        }
    };
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
        vertices,
        edges,
        weight,
    })
}
