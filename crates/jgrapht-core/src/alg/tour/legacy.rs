//! Travelling salesman heuristics and an exact solver. Distances between two vertices are the
//! lightest edge between them; the graph should be complete for the heuristics to apply.

use crate::alg::spanning::kruskal;
use crate::alg::util::Indexed;
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

/// Christofides' 3/2-approximation for metric instances: MST, a minimum-weight perfect matching
/// of its odd-degree vertices (exact for up to 20 such vertices, greedy beyond), an Eulerian
/// circuit of the union and shortcutting.
pub fn christofides_tour<G: Graph>(g: &G) -> Result<GraphWalk<G::Vertex, G::Edge>> {
    use crate::{BaseGraph, DefaultEdge, GraphMut, GraphMutExt};
    let ix = Indexed::new(g);
    let n = ix.n();
    check(n)?;
    let d = matrix(&ix);
    let tree = kruskal(g);
    let mut multi = BaseGraph::<usize, DefaultEdge, crate::graph::Undirected>::multi();
    multi.add_vertices(0..n);
    let mut deg = vec![0usize; n];
    for e in &tree.edges {
        let k = ix.es.iter().position(|x| *x == e).unwrap();
        let (a, b) = ix.ends[k];
        multi.add_edge(&a, &b)?;
        deg[a] += 1;
        deg[b] += 1;
    }
    if tree.edges.len() + 1 != n {
        return Err(GraphError::InvalidArgument("graph is not connected".into()));
    }
    let odd: Vec<usize> = (0..n).filter(|&v| deg[v] % 2 == 1).collect();
    let pairs: Vec<(usize, usize)> = if odd.len() <= 20 {
        // DP over subsets of odd vertices
        let m = odd.len();
        let full = 1usize << m;
        let mut cost = vec![f64::INFINITY; full];
        let mut pick = vec![0usize; full];
        cost[0] = 0.0;
        for mask in 1..full {
            let i = mask.trailing_zeros() as usize;
            let rest = mask & (mask - 1);
            let mut others = rest;
            while others != 0 {
                let j = others.trailing_zeros() as usize;
                others &= others - 1;
                let c = d[odd[i]][odd[j]] + cost[rest & !(1 << j)];
                if c < cost[mask] {
                    cost[mask] = c;
                    pick[mask] = j;
                }
            }
        }
        let (mut mask, mut out) = (full - 1, Vec::new());
        while mask != 0 {
            let i = mask.trailing_zeros() as usize;
            let j = pick[mask];
            out.push((odd[i], odd[j]));
            mask &= !(1 << i);
            mask &= !(1 << j);
        }
        out
    } else {
        let mut left = odd.clone();
        let mut out = Vec::new();
        while let Some(a) = left.pop() {
            let (pos, _) = left
                .iter()
                .enumerate()
                .min_by(|x, y| d[a][*x.1].total_cmp(&d[a][*y.1]))
                .unwrap();
            out.push((a, left.swap_remove(pos)));
        }
        out
    };
    for (a, b) in pairs {
        multi.add_edge(&a, &b)?;
    }
    let circuit = crate::alg::eulerian::eulerian_cycle(&multi)
        .ok_or_else(|| GraphError::InvalidArgument("no Eulerian circuit".into()))?;
    let mut seen = vec![false; n];
    let mut order = Vec::new();
    for v in circuit.vertices {
        if !std::mem::replace(&mut seen[v], true) {
            order.push(v);
        }
    }
    to_walk(g, &ix, &order)
}
