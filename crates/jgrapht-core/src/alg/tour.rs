//! Travelling salesman heuristics and an exact solver. Distances between two vertices are the
//! lightest edge between them; the graph should be complete for the heuristics to apply.

use super::spanning::kruskal;
use super::util::Indexed;
use crate::graph::Graph;
use crate::walk::GraphWalk;
use crate::{GraphError, Result};

fn matrix<G: Graph>(ix: &Indexed<'_, G>) -> Vec<Vec<f64>> {
    let n = ix.n();
    let mut d = vec![vec![f64::INFINITY; n]; n];
    for (k, &(a, b)) in ix.ends.iter().enumerate() {
        if a != b {
            d[a][b] = d[a][b].min(ix.w[k]);
            if !ix.directed {
                d[b][a] = d[b][a].min(ix.w[k]);
            }
        }
    }
    d
}

fn to_walk<G: Graph>(
    g: &G,
    ix: &Indexed<'_, G>,
    order: &[usize],
) -> Result<GraphWalk<G::Vertex, G::Edge>> {
    let mut vs = ix.vertices_of(order);
    if let Some(first) = vs.first().cloned() {
        vs.push(first);
    }
    GraphWalk::from_vertices(g, vs).ok_or_else(|| {
        GraphError::InvalidArgument("graph has no Hamiltonian cycle along these edges".into())
    })
}

fn length(d: &[Vec<f64>], order: &[usize]) -> f64 {
    (0..order.len())
        .map(|i| d[order[i]][order[(i + 1) % order.len()]])
        .sum()
}

fn check(n: usize) -> Result<()> {
    if n < 3 {
        return Err(GraphError::InvalidArgument(
            "a tour needs at least 3 vertices".into(),
        ));
    }
    Ok(())
}

/// Nearest neighbour heuristic from the first vertex.
pub fn nearest_neighbor_tour<G: Graph>(g: &G) -> Result<GraphWalk<G::Vertex, G::Edge>> {
    let ix = Indexed::new(g);
    check(ix.n())?;
    let d = matrix(&ix);
    let mut order = vec![0];
    let mut seen = vec![false; ix.n()];
    seen[0] = true;
    while order.len() < ix.n() {
        let cur = *order.last().unwrap();
        let next = (0..ix.n())
            .filter(|&v| !seen[v])
            .min_by(|&a, &b| d[cur][a].total_cmp(&d[cur][b]))
            .unwrap();
        seen[next] = true;
        order.push(next);
    }
    to_walk(g, &ix, &order)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Insertion {
    Nearest,
    Farthest,
    Cheapest,
}

/// Insertion heuristics: grow a sub-tour by inserting the nearest / farthest / cheapest vertex.
pub fn insertion_tour<G: Graph>(g: &G, rule: Insertion) -> Result<GraphWalk<G::Vertex, G::Edge>> {
    let ix = Indexed::new(g);
    let n = ix.n();
    check(n)?;
    let d = matrix(&ix);
    let mut tour = vec![0usize];
    let mut left: Vec<usize> = (1..n).collect();
    while !left.is_empty() {
        let dist_to_tour = |v: usize| tour.iter().map(|&t| d[t][v]).fold(f64::INFINITY, f64::min);
        let best_slot = |v: usize| {
            (0..tour.len())
                .map(|i| {
                    let (a, b) = (tour[i], tour[(i + 1) % tour.len()]);
                    (
                        i + 1,
                        d[a][v] + d[v][b] - if tour.len() > 1 { d[a][b] } else { 0.0 },
                    )
                })
                .min_by(|x, y| x.1.total_cmp(&y.1))
                .unwrap()
        };
        let pick = match rule {
            Insertion::Nearest => left
                .iter()
                .copied()
                .min_by(|&a, &b| dist_to_tour(a).total_cmp(&dist_to_tour(b))),
            Insertion::Farthest => left
                .iter()
                .copied()
                .max_by(|&a, &b| dist_to_tour(a).total_cmp(&dist_to_tour(b))),
            Insertion::Cheapest => left
                .iter()
                .copied()
                .min_by(|&a, &b| best_slot(a).1.total_cmp(&best_slot(b).1)),
        }
        .unwrap();
        let (slot, _) = best_slot(pick);
        tour.insert(slot, pick);
        left.retain(|&v| v != pick);
    }
    to_walk(g, &ix, &tour)
}

/// Double-tree 2-approximation (metric instances): pre-order walk of a minimum spanning tree.
pub fn double_tree_tour<G: Graph>(g: &G) -> Result<GraphWalk<G::Vertex, G::Edge>> {
    let ix = Indexed::new(g);
    check(ix.n())?;
    let tree = kruskal(g);
    let mut adj = vec![Vec::new(); ix.n()];
    for e in &tree.edges {
        let k = ix.es.iter().position(|x| *x == e).unwrap();
        let (a, b) = ix.ends[k];
        adj[a].push(b);
        adj[b].push(a);
    }
    let (mut order, mut seen, mut stack) = (Vec::new(), vec![false; ix.n()], vec![0]);
    while let Some(v) = stack.pop() {
        if !std::mem::replace(&mut seen[v], true) {
            order.push(v);
            stack.extend(adj[v].iter().rev().copied().filter(|&u| !seen[u]));
        }
    }
    if order.len() != ix.n() {
        return Err(GraphError::InvalidArgument("graph is not connected".into()));
    }
    to_walk(g, &ix, &order)
}

/// 2-opt local search starting from `tour` (a closed walk visiting every vertex once).
pub fn two_opt<G: Graph>(
    g: &G,
    tour: &GraphWalk<G::Vertex, G::Edge>,
) -> Result<GraphWalk<G::Vertex, G::Edge>> {
    let ix = Indexed::new(g);
    let d = matrix(&ix);
    let mut order: Vec<usize> = tour.vertices[..tour.vertices.len().saturating_sub(1)]
        .iter()
        .map(|v| ix.idx[v])
        .collect();
    check(order.len())?;
    let n = order.len();
    let mut improved = true;
    while improved {
        improved = false;
        for i in 0..n - 1 {
            for j in i + 2..n {
                if i == 0 && j == n - 1 {
                    continue;
                }
                let (a, b, c, e) = (order[i], order[i + 1], order[j], order[(j + 1) % n]);
                if d[a][c] + d[b][e] < d[a][b] + d[c][e] - 1e-12 {
                    order[i + 1..=j].reverse();
                    improved = true;
                }
            }
        }
    }
    debug_assert!(length(&d, &order).is_finite() || !d.iter().flatten().all(|x| x.is_finite()));
    to_walk(g, &ix, &order)
}

/// Exact Held–Karp dynamic programme, `O(2^n n^2)`; limited to 20 vertices.
pub fn held_karp_tour<G: Graph>(g: &G) -> Result<GraphWalk<G::Vertex, G::Edge>> {
    let ix = Indexed::new(g);
    let n = ix.n();
    check(n)?;
    if n > 20 {
        return Err(GraphError::InvalidArgument(
            "Held–Karp is limited to 20 vertices".into(),
        ));
    }
    let d = matrix(&ix);
    let full = 1usize << (n - 1);
    let mut dp = vec![vec![f64::INFINITY; n - 1]; full];
    let mut par = vec![vec![usize::MAX; n - 1]; full];
    for j in 0..n - 1 {
        dp[1 << j][j] = d[0][j + 1];
    }
    for mask in 1..full {
        for j in 0..n - 1 {
            if mask >> j & 1 == 0 || dp[mask][j].is_infinite() {
                continue;
            }
            for k in 0..n - 1 {
                if mask >> k & 1 == 1 {
                    continue;
                }
                let nm = mask | 1 << k;
                let c = dp[mask][j] + d[j + 1][k + 1];
                if c < dp[nm][k] {
                    dp[nm][k] = c;
                    par[nm][k] = j;
                }
            }
        }
    }
    let last = (0..n - 1)
        .min_by(|&a, &b| {
            (dp[full - 1][a] + d[a + 1][0]).total_cmp(&(dp[full - 1][b] + d[b + 1][0]))
        })
        .unwrap();
    if !(dp[full - 1][last] + d[last + 1][0]).is_finite() {
        return Err(GraphError::InvalidArgument(
            "graph has no Hamiltonian cycle".into(),
        ));
    }
    let (mut order, mut mask, mut j) = (vec![], full - 1, last);
    while j != usize::MAX {
        order.push(j + 1);
        let p = par[mask][j];
        mask &= !(1 << j);
        j = p;
    }
    order.push(0);
    order.reverse();
    to_walk(g, &ix, &order)
}
