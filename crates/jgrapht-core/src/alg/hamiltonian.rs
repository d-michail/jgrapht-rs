//! Hamiltonian cycles and paths by backtracking (exponential in the worst case).

use super::util::Indexed;
use crate::graph::Graph;
use crate::walk::GraphWalk;

fn search<G: Graph>(ix: &Indexed<'_, G>, start: usize, closed: bool) -> Option<Vec<usize>> {
    let n = ix.n();
    let mut path = vec![start];
    let mut on = vec![false; n];
    on[start] = true;
    // neighbours with few options first
    let order: Vec<Vec<usize>> = (0..n)
        .map(|v| {
            let mut o: Vec<usize> = ix.out[v]
                .iter()
                .map(|&(u, _)| u)
                .filter(|&u| u != v)
                .collect();
            o.sort_by_key(|&u| ix.out[u].len());
            o.dedup();
            o
        })
        .collect();
    fn rec(
        v: usize,
        n: usize,
        closed: bool,
        start: usize,
        order: &[Vec<usize>],
        on: &mut Vec<bool>,
        path: &mut Vec<usize>,
    ) -> bool {
        if path.len() == n {
            return !closed || order[v].contains(&start);
        }
        for &u in &order[v] {
            if !on[u] {
                on[u] = true;
                path.push(u);
                if rec(u, n, closed, start, order, on, path) {
                    return true;
                }
                path.pop();
                on[u] = false;
            }
        }
        false
    }
    rec(start, n, closed, start, &order, &mut on, &mut path).then_some(path)
}

fn walk<G: Graph>(
    g: &G,
    ix: &Indexed<'_, G>,
    order: &[usize],
    close: bool,
) -> Option<GraphWalk<G::Vertex, G::Edge>> {
    let mut vs = ix.vertices_of(order);
    if close {
        vs.push(vs[0].clone());
    }
    GraphWalk::from_vertices(g, vs)
}

/// A Hamiltonian cycle as a closed walk, if one exists (needs at least 3 vertices).
pub fn hamiltonian_cycle<G: Graph>(g: &G) -> Option<GraphWalk<G::Vertex, G::Edge>> {
    let ix = Indexed::new(g);
    if ix.n() < 3 {
        return None;
    }
    let order = search(&ix, 0, true)?;
    walk(g, &ix, &order, true)
}

/// A Hamiltonian path, if one exists.
pub fn hamiltonian_path<G: Graph>(g: &G) -> Option<GraphWalk<G::Vertex, G::Edge>> {
    let ix = Indexed::new(g);
    if ix.n() == 0 {
        return None;
    }
    for s in 0..ix.n() {
        if let Some(order) = search(&ix, s, false) {
            return walk(g, &ix, &order, false);
        }
    }
    None
}
