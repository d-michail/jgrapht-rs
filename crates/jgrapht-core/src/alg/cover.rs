//! Vertex cover.

use super::matching::hopcroft_karp_mates;
use super::partition::two_color;
use super::util::Indexed;
use crate::graph::Graph;
use crate::{GraphError, Result};
use std::collections::{HashSet, VecDeque};

/// A vertex cover with its total vertex weight.
#[derive(Debug, Clone)]
pub struct VertexCover<V> {
    pub vertices: Vec<V>,
    pub weight: f64,
}

fn finish<G: Graph>(
    ix: &Indexed<'_, G>,
    cover: &[bool],
    weight: &dyn Fn(&G::Vertex) -> f64,
) -> VertexCover<G::Vertex> {
    let vertices: Vec<_> = (0..ix.n())
        .filter(|&v| cover[v])
        .map(|v| ix.vertex(v))
        .collect();
    let weight = vertices.iter().map(weight).sum();
    VertexCover { vertices, weight }
}

/// Greedy: repeatedly take the vertex covering the most uncovered edges.
pub fn greedy_vertex_cover<G: Graph>(g: &G) -> VertexCover<G::Vertex> {
    let ix = Indexed::new(g);
    let mut covered = vec![false; ix.es.len()];
    let mut cover = vec![false; ix.n()];
    loop {
        let best = (0..ix.n())
            .map(|v| {
                (
                    v,
                    ix.out[v]
                        .iter()
                        .chain(if ix.directed { &ix.inn[v][..] } else { &[] })
                        .filter(|&&(_, k)| !covered[k])
                        .count(),
                )
            })
            .max_by_key(|&(_, c)| c);
        match best {
            Some((v, c)) if c > 0 => {
                cover[v] = true;
                ix.out[v]
                    .iter()
                    .chain(&ix.inn[v])
                    .for_each(|&(_, k)| covered[k] = true);
            }
            _ => break,
        }
    }
    finish(&ix, &cover, &|_| 1.0)
}

/// Classic 2-approximation: take both endpoints of a maximal matching.
pub fn two_approximation_vertex_cover<G: Graph>(g: &G) -> VertexCover<G::Vertex> {
    let ix = Indexed::new(g);
    let mut cover = vec![false; ix.n()];
    for &(a, b) in &ix.ends {
        if !cover[a] && !cover[b] {
            cover[a] = true;
            cover[b] = true;
        }
    }
    finish(&ix, &cover, &|_| 1.0)
}

/// Bar-Yehuda–Even local-ratio 2-approximation for weighted vertex cover.
pub fn bar_yehuda_even_vertex_cover<G: Graph>(
    g: &G,
    weight: impl Fn(&G::Vertex) -> f64,
) -> VertexCover<G::Vertex> {
    let ix = Indexed::new(g);
    let mut residual: Vec<f64> = (0..ix.n()).map(|v| weight(ix.vs[v])).collect();
    for &(a, b) in &ix.ends {
        let d = residual[a].min(residual[b]);
        residual[a] -= d;
        residual[b] -= d;
    }
    let cover: Vec<bool> = residual.iter().map(|&r| r <= 1e-12).collect();
    finish(&ix, &cover, &weight)
}

/// Clarkson's greedy 2-approximation for weighted vertex cover (best weight/degree ratio).
pub fn clarkson_vertex_cover<G: Graph>(
    g: &G,
    weight: impl Fn(&G::Vertex) -> f64,
) -> VertexCover<G::Vertex> {
    let ix = Indexed::new(g);
    let n = ix.n();
    let mut w: Vec<f64> = (0..n).map(|v| weight(ix.vs[v])).collect();
    let mut alive: Vec<HashSet<usize>> = (0..n)
        .map(|v| {
            ix.out[v]
                .iter()
                .chain(&ix.inn[v])
                .map(|&(u, _)| u)
                .filter(|&u| u != v)
                .collect()
        })
        .collect();
    let mut cover = vec![false; n];
    loop {
        let best = (0..n).filter(|&v| !alive[v].is_empty()).min_by(|&a, &b| {
            (w[a] / alive[a].len() as f64).total_cmp(&(w[b] / alive[b].len() as f64))
        });
        let Some(v) = best else { break };
        let ratio = w[v] / alive[v].len() as f64;
        let nbs: Vec<usize> = alive[v].iter().copied().collect();
        for u in nbs {
            w[u] -= ratio;
            alive[u].remove(&v);
        }
        cover[v] = true;
        alive[v].clear();
        w[v] = 0.0;
    }
    finish(&ix, &cover, &weight)
}

/// Exact minimum vertex cover of a bipartite graph (König's theorem).
pub fn bipartite_vertex_cover<G: Graph>(g: &G) -> Result<VertexCover<G::Vertex>> {
    let ix = Indexed::new(g);
    let side = two_color(&ix)
        .ok_or_else(|| GraphError::InvalidArgument("graph is not bipartite".into()))?;
    let adj: Vec<Vec<usize>> = (0..ix.n())
        .map(|v| {
            ix.out[v]
                .iter()
                .chain(&ix.inn[v])
                .map(|&(u, _)| u)
                .filter(|&u| u != v)
                .collect()
        })
        .collect();
    let left: Vec<usize> = (0..ix.n()).filter(|&v| !side[v]).collect();
    let mate = hopcroft_karp_mates(&adj, &left);
    // vertices reachable from unmatched left vertices by alternating paths
    let mut z = vec![false; ix.n()];
    let mut q: VecDeque<usize> = left
        .iter()
        .copied()
        .filter(|&v| mate[v].is_none())
        .collect();
    q.iter().for_each(|&v| z[v] = true);
    while let Some(v) = q.pop_front() {
        if !side[v] {
            for &u in &adj[v] {
                if mate[v] != Some(u) && !z[u] {
                    z[u] = true;
                    q.push_back(u);
                }
            }
        } else if let Some(u) = mate[v] {
            if !z[u] {
                z[u] = true;
                q.push_back(u);
            }
        }
    }
    let cover: Vec<bool> = (0..ix.n())
        .map(|v| if side[v] { z[v] } else { !z[v] })
        .collect();
    Ok(finish(&ix, &cover, &|_| 1.0))
}

/// Whether every edge has an endpoint in `vs`.
pub fn is_vertex_cover<G: Graph>(g: &G, vs: &[G::Vertex]) -> bool {
    let set: HashSet<_> = vs.iter().collect();
    g.edges()
        .all(|e| set.contains(g.edge_source(e).unwrap()) || set.contains(g.edge_target(e).unwrap()))
}
