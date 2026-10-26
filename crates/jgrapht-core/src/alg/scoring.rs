//! Vertex and edge scoring: centralities, PageRank, coreness, clustering coefficients.

use super::util::Indexed;
use crate::graph::Graph;
use crate::util::OrderedF64;
use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap, HashSet, VecDeque};

fn scores<G: Graph>(ix: &Indexed<'_, G>, s: Vec<f64>) -> HashMap<G::Vertex, f64> {
    s.into_iter()
        .enumerate()
        .map(|(i, x)| (ix.vertex(i), x))
        .collect()
}

/// Degree of each vertex, optionally divided by `n - 1`.
pub fn degree_centrality<G: Graph>(g: &G, normalize: bool) -> HashMap<G::Vertex, f64> {
    let ix = Indexed::new(g);
    let d = (ix.n().max(2) - 1) as f64;
    let s = (0..ix.n())
        .map(|v| g.degree_of(ix.vs[v]) as f64 / if normalize { d } else { 1.0 })
        .collect();
    scores(&ix, s)
}

/// Single-source distances (BFS when unweighted, Dijkstra otherwise) over `out` adjacency.
fn distances<G: Graph>(ix: &Indexed<'_, G>, s: usize, weighted: bool) -> Vec<f64> {
    let mut dist = vec![f64::INFINITY; ix.n()];
    dist[s] = 0.0;
    if !weighted {
        let mut q = VecDeque::from([s]);
        while let Some(v) = q.pop_front() {
            for &(u, _) in &ix.out[v] {
                if dist[u].is_infinite() {
                    dist[u] = dist[v] + 1.0;
                    q.push_back(u);
                }
            }
        }
    } else {
        let mut heap = BinaryHeap::from([Reverse((OrderedF64(0.0), s))]);
        while let Some(Reverse((OrderedF64(d), v))) = heap.pop() {
            if d > dist[v] {
                continue;
            }
            for &(u, k) in &ix.out[v] {
                let nd = d + ix.w[k];
                if nd < dist[u] {
                    dist[u] = nd;
                    heap.push(Reverse((OrderedF64(nd), u)));
                }
            }
        }
    }
    dist
}

/// Closeness: reachable vertices over the sum of distances to them, scaled by the reachable
/// fraction (Wasserman–Faust) so disconnected graphs behave.
pub fn closeness_centrality<G: Graph>(g: &G, weighted: bool) -> HashMap<G::Vertex, f64> {
    let ix = Indexed::new(g);
    let n = ix.n();
    let s = (0..n)
        .map(|v| {
            let d = distances(&ix, v, weighted);
            let (sum, reach) = d
                .iter()
                .filter(|x| x.is_finite() && **x > 0.0)
                .fold((0.0, 0usize), |(s, c), x| (s + x, c + 1));
            if sum == 0.0 {
                0.0
            } else {
                (reach as f64 / sum) * (reach as f64 / (n - 1) as f64)
            }
        })
        .collect();
    scores(&ix, s)
}

/// Harmonic centrality: sum of inverse distances to all other vertices.
pub fn harmonic_centrality<G: Graph>(g: &G, weighted: bool) -> HashMap<G::Vertex, f64> {
    let ix = Indexed::new(g);
    let s = (0..ix.n())
        .map(|v| {
            distances(&ix, v, weighted)
                .iter()
                .filter(|x| x.is_finite() && **x > 0.0)
                .map(|x| 1.0 / x)
                .sum()
        })
        .collect();
    scores(&ix, s)
}

/// Betweenness of vertices and edges via Brandes' algorithm.
pub struct Betweenness<V, E> {
    pub vertices: HashMap<V, f64>,
    pub edges: HashMap<E, f64>,
}

pub fn betweenness_centrality<G: Graph>(
    g: &G,
    weighted: bool,
    normalize: bool,
) -> Betweenness<G::Vertex, G::Edge> {
    let ix = Indexed::new(g);
    let n = ix.n();
    let mut cb = vec![0.0; n];
    let mut eb = vec![0.0; ix.es.len()];
    for s in 0..n {
        let (mut order, mut sigma, mut dist) =
            (Vec::new(), vec![0.0f64; n], vec![f64::INFINITY; n]);
        let mut preds: Vec<Vec<(usize, usize)>> = vec![Vec::new(); n]; // (vertex, edge)
        sigma[s] = 1.0;
        dist[s] = 0.0;
        if !weighted {
            let mut q = VecDeque::from([s]);
            while let Some(v) = q.pop_front() {
                order.push(v);
                for &(u, k) in &ix.out[v] {
                    if dist[u].is_infinite() {
                        dist[u] = dist[v] + 1.0;
                        q.push_back(u);
                    }
                    if dist[u] == dist[v] + 1.0 {
                        sigma[u] += sigma[v];
                        preds[u].push((v, k));
                    }
                }
            }
        } else {
            let mut heap = BinaryHeap::from([Reverse((OrderedF64(0.0), s))]);
            let mut done = vec![false; n];
            while let Some(Reverse((OrderedF64(d), v))) = heap.pop() {
                if done[v] {
                    continue;
                }
                done[v] = true;
                order.push(v);
                for &(u, k) in &ix.out[v] {
                    let nd = d + ix.w[k];
                    if nd < dist[u] - 1e-12 {
                        dist[u] = nd;
                        sigma[u] = sigma[v];
                        preds[u] = vec![(v, k)];
                        heap.push(Reverse((OrderedF64(nd), u)));
                    } else if (nd - dist[u]).abs() <= 1e-12 && !done[u] {
                        sigma[u] += sigma[v];
                        preds[u].push((v, k));
                    }
                }
            }
        }
        let mut delta = vec![0.0; n];
        for &w in order.iter().rev() {
            for &(v, k) in &preds[w] {
                let c = sigma[v] / sigma[w] * (1.0 + delta[w]);
                delta[v] += c;
                eb[k] += c;
            }
            if w != s {
                cb[w] += delta[w];
            }
        }
    }
    // undirected pairs are visited from both ends, so halve; normalising divides by the number
    // of ordered (directed) or unordered (undirected) pairs excluding the vertex itself
    let mut scale = if ix.directed { 1.0 } else { 0.5 };
    if normalize && n > 2 {
        scale /= ((n - 1) * (n - 2)) as f64 * if ix.directed { 1.0 } else { 0.5 };
    }
    Betweenness {
        vertices: scores(&ix, cb.iter().map(|x| x * scale).collect()),
        edges: (0..ix.es.len())
            .map(|k| {
                (
                    ix.es[k].clone(),
                    eb[k] * if ix.directed { 1.0 } else { 0.5 },
                )
            })
            .collect(),
    }
}

/// PageRank by power iteration. Edge weights are transition weights; dangling vertices spread
/// their rank uniformly.
pub fn pagerank<G: Graph>(
    g: &G,
    damping: f64,
    max_iterations: usize,
    tolerance: f64,
) -> HashMap<G::Vertex, f64> {
    let ix = Indexed::new(g);
    let n = ix.n();
    if n == 0 {
        return HashMap::new();
    }
    let out_w: Vec<f64> = (0..n)
        .map(|v| ix.out[v].iter().map(|&(_, k)| ix.w[k]).sum())
        .collect();
    let mut x = vec![1.0 / n as f64; n];
    for _ in 0..max_iterations {
        let dangling: f64 = (0..n).filter(|&v| out_w[v] == 0.0).map(|v| x[v]).sum();
        let mut next = vec![(1.0 - damping) / n as f64 + damping * dangling / n as f64; n];
        for v in 0..n {
            for &(u, k) in &ix.out[v] {
                next[u] += damping * x[v] * ix.w[k] / out_w[v];
            }
        }
        let diff: f64 = x.iter().zip(&next).map(|(a, b)| (a - b).abs()).sum();
        x = next;
        if diff < tolerance {
            break;
        }
    }
    scores(&ix, x)
}

/// Eigenvector centrality by power iteration (L2-normalised); each vertex is scored by the
/// scores of its in-neighbours.
pub fn eigenvector_centrality<G: Graph>(
    g: &G,
    max_iterations: usize,
    tolerance: f64,
) -> HashMap<G::Vertex, f64> {
    let ix = Indexed::new(g);
    let n = ix.n();
    let mut x = vec![1.0 / (n.max(1) as f64).sqrt(); n];
    for _ in 0..max_iterations {
        // shift by identity so bipartite graphs converge
        let mut next = x.clone();
        for v in 0..n {
            for &(u, k) in &ix.inn[v] {
                next[v] += x[u] * ix.w[k];
            }
        }
        let norm = next.iter().map(|a| a * a).sum::<f64>().sqrt();
        if norm == 0.0 {
            break;
        }
        next.iter_mut().for_each(|a| *a /= norm);
        let diff: f64 = x.iter().zip(&next).map(|(a, b)| (a - b).abs()).sum();
        x = next;
        if diff < tolerance {
            break;
        }
    }
    scores(&ix, x)
}

/// Katz centrality `x = alpha * A^T x + beta`.
pub fn katz_centrality<G: Graph>(
    g: &G,
    alpha: f64,
    beta: f64,
    max_iterations: usize,
    tolerance: f64,
) -> HashMap<G::Vertex, f64> {
    let ix = Indexed::new(g);
    let n = ix.n();
    let mut x = vec![beta; n];
    for _ in 0..max_iterations {
        let mut next = vec![beta; n];
        for v in 0..n {
            for &(u, _) in &ix.inn[v] {
                next[v] += alpha * x[u];
            }
        }
        let diff: f64 = x.iter().zip(&next).map(|(a, b)| (a - b).abs()).sum();
        x = next;
        if diff < tolerance {
            break;
        }
    }
    scores(&ix, x)
}

/// Coreness (k-core number) of each vertex, ignoring direction, loops and parallel edges.
/// The graph's degeneracy is the maximum value.
pub fn coreness<G: Graph>(g: &G) -> HashMap<G::Vertex, usize> {
    let ix = Indexed::new(g);
    let n = ix.n();
    let nb: Vec<HashSet<usize>> = (0..n)
        .map(|v| {
            ix.out[v]
                .iter()
                .chain(&ix.inn[v])
                .map(|&(u, _)| u)
                .filter(|&u| u != v)
                .collect()
        })
        .collect();
    let mut deg: Vec<usize> = nb.iter().map(HashSet::len).collect();
    let (mut core, mut removed, mut k) = (vec![0; n], vec![false; n], 0);
    for _ in 0..n {
        let v = (0..n)
            .filter(|&v| !removed[v])
            .min_by_key(|&v| deg[v])
            .unwrap();
        k = k.max(deg[v]);
        core[v] = k;
        removed[v] = true;
        nb[v]
            .iter()
            .filter(|&&u| !removed[u])
            .for_each(|&u| deg[u] -= 1);
    }
    (0..n).map(|v| (ix.vertex(v), core[v])).collect()
}

fn undirected_neighbors<G: Graph>(ix: &Indexed<'_, G>) -> Vec<HashSet<usize>> {
    (0..ix.n())
        .map(|v| {
            ix.out[v]
                .iter()
                .chain(&ix.inn[v])
                .map(|&(u, _)| u)
                .filter(|&u| u != v)
                .collect()
        })
        .collect()
}

/// Local clustering coefficient of every vertex (direction ignored).
pub fn local_clustering_coefficients<G: Graph>(g: &G) -> HashMap<G::Vertex, f64> {
    let ix = Indexed::new(g);
    let nb = undirected_neighbors(&ix);
    let s = (0..ix.n())
        .map(|v| {
            let d = nb[v].len();
            if d < 2 {
                return 0.0;
            }
            let links = nb[v]
                .iter()
                .map(|u| nb[*u].iter().filter(|w| nb[v].contains(w)).count())
                .sum::<usize>()
                / 2;
            links as f64 / (d * (d - 1) / 2) as f64
        })
        .collect();
    scores(&ix, s)
}

/// Average of the local clustering coefficients.
pub fn average_clustering_coefficient<G: Graph>(g: &G) -> f64 {
    let c = local_clustering_coefficients(g);
    if c.is_empty() {
        0.0
    } else {
        c.values().sum::<f64>() / c.len() as f64
    }
}

/// Global clustering coefficient (transitivity): 3 * triangles / connected triples.
pub fn global_clustering_coefficient<G: Graph>(g: &G) -> f64 {
    let ix = Indexed::new(g);
    let nb = undirected_neighbors(&ix);
    let triples: usize = nb
        .iter()
        .map(|s| s.len() * s.len().saturating_sub(1) / 2)
        .sum();
    if triples == 0 {
        return 0.0;
    }
    (3 * count_triangles_sets(&nb)) as f64 / triples as f64
}

fn count_triangles_sets(nb: &[HashSet<usize>]) -> usize {
    (0..nb.len())
        .map(|v| {
            nb[v]
                .iter()
                .filter(|&&u| u > v)
                .map(|&u| {
                    nb[u]
                        .iter()
                        .filter(|&&w| w > u && nb[v].contains(&w))
                        .count()
                })
                .sum::<usize>()
        })
        .sum()
}

/// Number of triangles (undirected view).
pub fn count_triangles<G: Graph>(g: &G) -> usize {
    count_triangles_sets(&undirected_neighbors(&Indexed::new(g)))
}
