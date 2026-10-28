// (C) Copyright 2003-2026, by John V Sichi and JGraphT Contributors.
// Rust port (C) Copyright 2026, by Dimitrios Michail and jgrapht-rs Contributors.
//
// See the CONTRIBUTORS.md file distributed with this work for additional
// information regarding copyright ownership.
//
// This program and the accompanying materials are made available under the
// terms of the Eclipse Public License 2.0 which is available at
// http://www.eclipse.org/legal/epl-2.0, or the GNU Lesser General Public
// License v2.1 or later which is available at
// http://www.gnu.org/licenses/old-licenses/lgpl-2.1-standalone.html.
//
// SPDX-License-Identifier: EPL-2.0 OR LGPL-2.1-or-later

//! Vertex and edge scoring: centralities, PageRank, coreness, clustering coefficients.

use super::util::Indexed;
use crate::graph::Graph;
use crate::graph::{IncidenceGraph, IncomingGraph, OutgoingGraph, UndirectedGraph};
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
pub fn degree_centrality<G: IncidenceGraph>(g: &G, normalize: bool) -> HashMap<G::Vertex, f64> {
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
    super::metrics::number_of_triangles(g) as usize
}

// ---------------------------------------------------------------------------------------------
// EdgeBetweennessCentrality (JGraphT's class: Brandes' algorithm for edges)

use crate::GraphError;
use crate::graph::GraphExt;
use indexmap::IndexMap;
use rheaps::AddressableHeap;
use rheaps::tree::PairingHeap;

/// What to do when the number of shortest paths overflows a `long`.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverflowStrategy {
    IgnoreOverflow,
    ThrowExceptionOnOverflow,
}

/// Edge betweenness centrality.
///
/// A natural extension of betweenness to edges by counting the total shortest paths that pass
/// through an edge. See the paper: Ulrik Brandes: On Variants of Shortest-Path Betweenness
/// Centrality and their Generic Computation. Social Networks 30(2):136-145, 2008, for a nice
/// discussion of different variants of betweenness centrality. Note that this implementation does
/// not work for graphs which have multiple edges. Self-loops do not influence the result and are
/// thus ignored.
///
/// This implementation allows the user to compute centrality contributions only from a subset of the
/// graph vertices, i.e. to start shortest path computations only from a subset of the vertices. This
/// allows centrality approximations in big graphs. Note that in this case, the user is responsible
/// for any normalization necessary due to duplicate shortest paths that might occur in undirected
/// graphs.
///
/// # Rust notes
///
/// Edge betweenness: for every edge the sum over source/target pairs of the fraction of shortest
/// paths through it. Graphs with parallel edges are rejected.
pub struct EdgeBetweennessCentrality<'g, G: Graph> {
    graph: &'g G,
    start_vertices: Option<Vec<G::Vertex>>,
    overflow: OverflowStrategy,
    scores: Option<IndexMap<G::Edge, f64>>,
}

impl<'g, G: OutgoingGraph> EdgeBetweennessCentrality<'g, G> {
    /// Construct a new instance.
    pub fn new(graph: &'g G) -> Result<Self, GraphError> {
        Self::with_options(graph, OverflowStrategy::IgnoreOverflow, None)
    }

    /// `start_vertices` restricts the sources of the shortest-path searches; `None` uses every
    /// vertex (and halves the scores of undirected graphs).
    pub fn with_options(
        graph: &'g G,
        overflow: OverflowStrategy,
        start_vertices: Option<Vec<G::Vertex>>,
    ) -> Result<Self, GraphError> {
        if crate::graph::props::has_multiple_edges(graph) {
            return Err(GraphError::InvalidGraph(
                crate::GraphProperty::NoParallelEdges,
            ));
        }
        Ok(EdgeBetweennessCentrality {
            graph,
            start_vertices,
            overflow,
            scores: None,
        })
    }

    /// Scores of all edges, in the graph's edge order.
    pub fn scores(&mut self) -> Result<&IndexMap<G::Edge, f64>, GraphError> {
        if self.scores.is_none() {
            self.scores = Some(self.compute()?);
        }
        Ok(self.scores.as_ref().unwrap())
    }

    /// Get an edge score
    pub fn edge_score(&mut self, e: &G::Edge) -> Result<f64, GraphError> {
        if !self.graph.contains_edge(e) {
            return Err(GraphError::EdgeNotFound);
        }
        Ok(self.scores()?[e])
    }

    fn compute(&self) -> Result<IndexMap<G::Edge, f64>, GraphError> {
        let g = self.graph;
        let mut scores: IndexMap<G::Edge, f64> = g.edges().map(|e| (e.clone(), 0.0)).collect();
        let weighted = g.graph_type().weighted;
        let starts: Vec<G::Vertex> = match &self.start_vertices {
            Some(s) => s.clone(),
            None => g.vertices().cloned().collect(),
        };
        let divide_by_two = self.start_vertices.is_none() && !g.graph_type().directed;
        let mut stack: Vec<G::Vertex> = Vec::new();
        for source in &starts {
            let (pred, sigma) = if weighted {
                self.weighted_search(source, &mut stack)?
            } else {
                self.unweighted_search(source, &mut stack)?
            };
            // accumulate dependencies
            let mut delta: HashMap<G::Vertex, f64> =
                g.vertices().map(|v| (v.clone(), 0.0)).collect();
            while let Some(w) = stack.pop() {
                if let Some(w_pred) = pred.get(&w) {
                    for e in w_pred {
                        let v = g.opposite_vertex(e, &w).unwrap().clone();
                        let c = (sigma[&v] as f64 / sigma[&w] as f64) * (1.0 + delta[&w]);
                        *scores.get_mut(e).unwrap() += c;
                        *delta.get_mut(&v).unwrap() += c;
                    }
                }
            }
        }
        if divide_by_two {
            for s in scores.values_mut() {
                *s /= 2.0;
            }
        }
        Ok(scores)
    }

    fn add_paths(
        &self,
        sigma: &mut HashMap<G::Vertex, i64>,
        v: &G::Vertex,
        w: &G::Vertex,
    ) -> Result<(), GraphError> {
        let sum = sigma[w].wrapping_add(sigma[v]);
        if self.overflow == OverflowStrategy::ThrowExceptionOnOverflow && sum < 0 {
            return Err(GraphError::Overflow("long overflow".into()));
        }
        sigma.insert(w.clone(), sum);
        Ok(())
    }

    #[allow(clippy::type_complexity)]
    fn unweighted_search(
        &self,
        source: &G::Vertex,
        stack: &mut Vec<G::Vertex>,
    ) -> Result<(HashMap<G::Vertex, Vec<G::Edge>>, HashMap<G::Vertex, i64>), GraphError> {
        let g = self.graph;
        let mut pred: HashMap<G::Vertex, Vec<G::Edge>> = HashMap::new();
        let mut dist: HashMap<G::Vertex, f64> = HashMap::new();
        let mut sigma: HashMap<G::Vertex, i64> = g.vertices().map(|v| (v.clone(), 0)).collect();
        sigma.insert(source.clone(), 1);
        dist.insert(source.clone(), 0.0);
        let mut queue = std::collections::VecDeque::from([source.clone()]);
        while let Some(v) = queue.pop_front() {
            stack.push(v.clone());
            let v_distance = dist[&v];
            for e in g.out_edges_of(&v) {
                let w = g.opposite_vertex(e, &v).unwrap();
                if *w == v {
                    continue;
                }
                if !dist.contains_key(w) {
                    dist.insert(w.clone(), v_distance + 1.0);
                    queue.push_back(w.clone());
                }
                if dist[w] == v_distance + 1.0 {
                    self.add_paths(&mut sigma, &v, w)?;
                    pred.entry(w.clone()).or_default().push(e.clone());
                }
            }
        }
        Ok((pred, sigma))
    }

    #[allow(clippy::type_complexity)]
    fn weighted_search(
        &self,
        source: &G::Vertex,
        stack: &mut Vec<G::Vertex>,
    ) -> Result<(HashMap<G::Vertex, Vec<G::Edge>>, HashMap<G::Vertex, i64>), GraphError> {
        let g = self.graph;
        let mut pred: HashMap<G::Vertex, Vec<G::Edge>> = HashMap::new();
        let mut handles = HashMap::new();
        let mut sigma: HashMap<G::Vertex, i64> = g.vertices().map(|v| (v.clone(), 0)).collect();
        let mut heap: PairingHeap<OrderedF64, G::Vertex> = PairingHeap::new();
        sigma.insert(source.clone(), 1);
        handles.insert(source.clone(), heap.insert(OrderedF64(0.0), source.clone()));
        let mut final_dist: HashMap<G::Vertex, f64> = HashMap::new();
        while let Some((OrderedF64(v_distance), v)) = heap.pop() {
            final_dist.insert(v.clone(), v_distance);
            stack.push(v.clone());
            for e in g.out_edges_of(&v) {
                let w = g.opposite_vertex(e, &v).unwrap();
                if *w == v {
                    continue;
                }
                let e_weight = g.weight_of(e);
                if e_weight < 0.0 {
                    return Err(GraphError::InvalidWeight(crate::WeightProblem::Negative));
                }
                let new_distance = v_distance + e_weight;
                let current: Option<f64> = handles.get(w).map(|h| {
                    final_dist
                        .get(w)
                        .copied()
                        .unwrap_or_else(|| heap.key(*h).map(|k| k.0).unwrap_or(f64::INFINITY))
                });
                let w_key = match current {
                    None => {
                        let h = heap.insert(OrderedF64(new_distance), w.clone());
                        handles.insert(w.clone(), h);
                        sigma.insert(w.clone(), 0);
                        pred.insert(w.clone(), Vec::new());
                        new_distance
                    }
                    Some(key)
                        if !final_dist.contains_key(w) && key.total_cmp(&new_distance).is_gt() =>
                    {
                        let h = handles[w];
                        heap.decrease_key(h, OrderedF64(new_distance)).ok();
                        sigma.insert(w.clone(), 0);
                        pred.insert(w.clone(), Vec::new());
                        new_distance
                    }
                    Some(key) => key,
                };
                if w_key.total_cmp(&new_distance).is_eq() {
                    self.add_paths(&mut sigma, &v, w)?;
                    pred.entry(w.clone()).or_default().push(e.clone());
                }
            }
        }
        Ok((pred, sigma))
    }
}

/// An interface for all algorithms which assign scores to vertices of a graph.
///
/// # Rust notes
///
/// Scores of all vertices, available after construction (`VertexScoringAlgorithm`).
pub trait VertexScoringAlgorithm<V, S: Clone = f64> {
    /// Get a map with the scores of all edges
    fn scores(&self) -> &HashMap<V, S>;

    fn vertex_score(&self, v: &V) -> Result<S, GraphError>
    where
        V: Eq + std::hash::Hash,
    {
        self.scores()
            .get(v)
            .cloned()
            .ok_or(GraphError::VertexNotFound)
    }
}

/// Shortest-path distances from every vertex, along outgoing or (with `incoming`) incoming edges.
/// Dijkstra is used unless some weight is negative, then Floyd–Warshall.
fn all_distances<G: OutgoingGraph>(g: &G, incoming: bool) -> Vec<Vec<f64>> {
    fn run<H: OutgoingGraph>(h: &H) -> Vec<Vec<f64>> {
        let ix = Indexed::new(h);
        let n = ix.n();
        if ix.w.iter().all(|&w| w >= 0.0) {
            (0..n).map(|v| distances(&ix, v, true)).collect()
        } else {
            let fw = crate::alg::shortestpath::floyd_warshall(h).ok();
            (0..n)
                .map(|i| {
                    (0..n)
                        .map(|j| match &fw {
                            Some(fw) => fw.weight(ix.vs[i], ix.vs[j]).unwrap_or(f64::INFINITY),
                            None => f64::NAN,
                        })
                        .collect()
                })
                .collect()
        }
    }
    let d = run(g);
    if incoming && g.graph_type().directed {
        // distances along incoming edges are the transposed matrix
        let n = d.len();
        (0..n).map(|i| (0..n).map(|j| d[j][i]).collect()).collect()
    } else {
        d
    }
}

/// Closeness centrality.
///
/// Computes the closeness centrality of each vertex of a graph. The closeness of a vertex $x$ is
/// defined as the reciprocal of the farness, that is $H(x)= 1 / \sum_{y \neq x} d(x,y)$, where
/// $d(x,y)$ is the shortest path distance from $x$ to $y$. When normalization is used, the score is
/// multiplied by $n-1$ where $n$ is the total number of vertices in the graph. For more details see
/// [wikipedia](https://en.wikipedia.org/wiki/Closeness_centrality) and
///
/// - Alex Bavelas. Communication patterns in task-oriented groups. J. Acoust. Soc. Am,
///   22(6):725–730, 1950.
///
/// This implementation computes by default the closeness centrality using outgoing paths and
/// normalizes the scores. This behavior can be adjusted by the constructor arguments.
///
/// When the graph is disconnected, the closeness centrality score equals $0$ for all vertices. In
/// the case of weakly connected digraphs, the closeness centrality of several vertices might be 0.
/// See `HarmonicCentrality` for a different approach in case of disconnected graphs.
///
/// Shortest paths are computed either by using Dijkstra's algorithm or Floyd-Warshall depending on
/// whether the graph has edges with negative edge weights. Thus, the running time is either $O(n (m
/// +n \log n))$ or $O(n^3)$ respectively, where $n$ is the number of vertices and $m$ the number of
/// edges of the graph.
///
/// # Rust notes
///
/// Closeness centrality (`ClosenessCentrality`): the inverse of the sum of distances to all other
/// vertices, scaled by `n - 1` when normalized. Unreachable vertices give a score of 0.
pub struct ClosenessCentrality<V> {
    scores: HashMap<V, f64>,
}

impl<V: Eq + std::hash::Hash + Clone> ClosenessCentrality<V> {
    /// Construct a new instance. By default the centrality is normalized and computed using outgoing
    /// paths.
    pub fn new<G: Graph<Vertex = V> + OutgoingGraph>(g: &G) -> Self {
        Self::with_options(g, false, true)
    }

    pub fn with_options<G: Graph<Vertex = V> + OutgoingGraph>(
        g: &G,
        incoming: bool,
        normalize: bool,
    ) -> Self {
        let d = all_distances(g, incoming);
        let n = d.len();
        let ix_vertices: Vec<&V> = g.vertices().collect();
        let scores = (0..n)
            .map(|i| {
                let sum = (0..n).filter(|&j| j != i).fold(0.0, |a, j| a + d[i][j]);
                let score = if normalize {
                    (n - 1) as f64 / sum
                } else {
                    1.0 / sum
                };
                (ix_vertices[i].clone(), score)
            })
            .collect();
        ClosenessCentrality { scores }
    }
}

impl<V> VertexScoringAlgorithm<V> for ClosenessCentrality<V> {
    /// Get a map with the scores of all vertices
    fn scores(&self) -> &HashMap<V, f64> {
        &self.scores
    }
}

/// Harmonic centrality.
///
/// The harmonic centrality of a vertex $x$ is defined as $H(x)=\sum_{y \neq x} 1/d(x,y)$, where
/// $d(x,y)$ is the shortest path distance from $x$ to $y$. In case a distance $d(x,y)=\infinity$,
/// then $1/d(x,y)=0$. When normalization is used the score is divided by $n-1$ where $n$ is the
/// total number of vertices in the graph.
///
/// For details see the following papers:
///
/// - Yannick Rochat. Closeness centrality extended to unconnected graphs: The harmonic centrality
///   index. Applications of Social Network Analysis, 2009.
/// - Newman, Mark. 2003. The Structure and Function of Complex Networks. SIAM Review, 45(mars),
///   167–256.
///
/// and the [wikipedia](https://en.wikipedia.org/wiki/Closeness_centrality) article.
///
/// This implementation computes by default the centrality using outgoing paths and normalizes the
/// scores. This behavior can be adjusted by the constructor arguments.
///
/// Shortest paths are computed either by using Dijkstra's algorithm or Floyd-Warshall depending on
/// whether the graph has edges with negative edge weights. Thus, the running time is either $O(n (m
/// + n \log n))$ or $O(n^3)$ respectively, where $n$ is the number of vertices and $m$ the number of
///   edges of the graph.
///
/// # Rust notes
///
/// Harmonic centrality (`HarmonicCentrality`): the sum of inverse distances to all other
/// vertices, divided by `n - 1` when normalized.
pub struct HarmonicCentrality<V> {
    scores: HashMap<V, f64>,
}

impl<V: Eq + std::hash::Hash + Clone> HarmonicCentrality<V> {
    /// Construct a new instance. By default the centrality is normalized and computed using outgoing
    /// paths.
    pub fn new<G: Graph<Vertex = V> + OutgoingGraph>(g: &G) -> Self {
        Self::with_options(g, false, true)
    }

    pub fn with_options<G: Graph<Vertex = V> + OutgoingGraph>(
        g: &G,
        incoming: bool,
        normalize: bool,
    ) -> Self {
        let d = all_distances(g, incoming);
        let n = d.len();
        let vs: Vec<&V> = g.vertices().collect();
        let scores = (0..n)
            .map(|i| {
                let sum = (0..n)
                    .filter(|&j| j != i)
                    .fold(0.0, |a, j| a + 1.0 / d[i][j]);
                let score = if normalize && n > 1 {
                    sum / (n - 1) as f64
                } else {
                    sum
                };
                (vs[i].clone(), score)
            })
            .collect();
        HarmonicCentrality { scores }
    }
}

impl<V> VertexScoringAlgorithm<V> for HarmonicCentrality<V> {
    /// Get a map with the scores of all vertices
    fn scores(&self) -> &HashMap<V, f64> {
        &self.scores
    }
}

/// Computes the coreness of each vertex in an undirected graph.
///
/// A $k$-core of a graph $G$ is a maximal connected subgraph of $G$ in which all vertices have
/// degree at least $k$. Equivalently, it is one of the connected components of the subgraph of $G$
/// formed by repeatedly deleting all vertices of degree less than $k$. A vertex $u$ has coreness $c$
/// if it belongs to a $c$-core but not to any $(c+1)$-core.
///
/// If a non-empty k-core exists, then, clearly, $G$ has
/// [degeneracy](https://en.wikipedia.org/wiki/Degeneracy_(graph_theory)) at least $k$,
/// and the degeneracy of $G$ is the largest $k$ for which $G$ has a $k$-core.
///
/// As described in the following paper
///
/// - D. W. Matula and L. L. Beck. Smallest-last ordering and clustering and graph coloring
///   algorithms. Journal of the ACM, 30(3):417--427, 1983.
///
/// it is possible to find a vertex ordering of a finite graph $G$ that optimizes the coloring number
/// of the ordering, in linear time, by using a bucket queue to repeatedly find and remove the vertex
/// of smallest degree.
///
/// # Rust notes
///
/// Coreness (k-core numbers) of an undirected simple graph and its degeneracy (`Coreness`).
pub struct Coreness<V> {
    scores: HashMap<V, usize>,
    degeneracy: usize,
}

impl<V: Eq + std::hash::Hash + Clone> Coreness<V> {
    /// Constructor
    pub fn new<G: UndirectedGraph<Vertex = V> + OutgoingGraph>(g: &G) -> Result<Self, GraphError> {
        if !crate::graph::props::is_simple(g) {
            return Err(GraphError::InvalidGraph(crate::GraphProperty::Simple));
        }
        let scores = coreness(g);
        let degeneracy = scores.values().copied().max().unwrap_or(0);
        Ok(Coreness { scores, degeneracy })
    }

    /// Compute the degeneracy of a graph.
    ///
    /// The degeneracy of a graph is the smallest value of $k$ for which it is $k$-degenerate. In
    /// graph theory, a $k$-degenerate graph is an undirected graph in which every subgraph has a
    /// vertex of degree at most $k$: that is, some vertex in the subgraph touches $k$ or fewer of
    /// the subgraph's edges.
    pub fn degeneracy(&self) -> usize {
        self.degeneracy
    }
}

impl<V> VertexScoringAlgorithm<V, usize> for Coreness<V> {
    /// Get a map with the scores of all vertices
    fn scores(&self) -> &HashMap<V, usize> {
        &self.scores
    }
}

/// Clustering coefficient. This implementation computes the global, the local and the average
/// clustering coefficient in an undirected or a directed network.
///
/// The
/// [local clustering coefficient](https://en.wikipedia.org/wiki/Clustering_coefficient#Local_clustering_coefficient) of a vertex in a graph quantifies how close its neighbors are to being
/// a clique. For a vertex $v$ it counts how many of its direct neighbors are connected by an edge
/// over the total number of neighbor pairs. In the case of undirected graphs the total number of
/// possible neighbor pairs is only half compared to directed graphs.
///
/// The local clustering coefficient of a graph was introduced in *D. J. Watts and Steven Strogatz
/// (June 1998). "Collective dynamics of 'small-world' networks". Nature. 393 (6684): 440–442.
/// doi:10.1038/30918*. It is simply the average of the local clustering coefficients of all the
/// vertices of the graph.
///
/// The global clustering coefficient of a graph is based on triplets of nodes. A triplet is three
/// graph nodes which are connected either by two edges or by three edges. A triplet which is
/// connected by two edges, is called an open triplet. A triplet which is connected with three edges
/// is called a closed triplet. The global clustering coefficient is defined as the number of closed
/// triplets over the total number of triplets (open and closed). It was introduced in *R. D. Luce
/// and A. D. Perry (1949). "A method of matrix analysis of group structure". Psychometrika. 14 (1):
/// 95–116. doi:10.1007/BF02289146*.
///
/// The running time is $O(|V| + \Delta(G)^2)$ where $|V|$ is the number of vertices and $\Delta(G)$
/// is the maximum degree of a vertex. The space complexity is $O(|V|)$.
///
/// # Rust notes
///
/// Local, average and global clustering coefficients (`ClusteringCoefficient`). In directed
/// graphs a vertex' neighbourhood is the union of predecessors and successors and the edges
/// among its members are counted with their direction.
pub struct ClusteringCoefficient<'g, G: Graph> {
    graph: &'g G,
    scores: HashMap<G::Vertex, f64>,
}

impl<'g, G: IncidenceGraph> ClusteringCoefficient<'g, G> {
    /// Construct a new instance
    pub fn new(graph: &'g G) -> Self {
        let scores = graph
            .vertices()
            .map(|v| (v.clone(), Self::local(graph, v)))
            .collect();
        ClusteringCoefficient { graph, scores }
    }

    fn local(g: &G, v: &G::Vertex) -> f64 {
        let hood: indexmap::IndexSet<&G::Vertex> = g
            .edges_of(v)
            .filter_map(|e| g.opposite_vertex(e, v))
            .collect();
        let k = hood.len() as f64;
        if k <= 1.0 {
            return 0.0;
        }
        let mut triplets = 0.0;
        for p in &hood {
            for q in &hood {
                if g.contains_edge_between(p, q) {
                    triplets += 1.0;
                }
            }
        }
        triplets / (k * (k - 1.0))
    }

    /// The mean of the local coefficients; 0 for the empty graph.
    pub fn average_clustering_coefficient(&self) -> f64 {
        if self.graph.vertex_count() == 0 {
            return 0.0;
        }
        self.scores.values().sum::<f64>() / self.graph.vertex_count() as f64
    }

    /// Three times the number of triangles over the number of connected triplets.
    pub fn global_clustering_coefficient(&self) -> f64 {
        let g = self.graph;
        let mut triplets = 0.0;
        for v in g.vertices() {
            if !g.graph_type().directed {
                let d = g.degree_of(v) as f64;
                triplets += d * (d - 1.0) / 2.0;
            } else {
                let preds: std::collections::HashSet<_> = g
                    .in_edges_of(v)
                    .filter_map(|e| g.opposite_vertex(e, v))
                    .collect();
                let succs: std::collections::HashSet<_> = g
                    .out_edges_of(v)
                    .filter_map(|e| g.opposite_vertex(e, v))
                    .collect();
                triplets += preds.len() as f64 * succs.len() as f64;
            }
        }
        3.0 * crate::alg::metrics::number_of_triangles(g) as f64 / triplets
    }
}

impl<G: Graph> VertexScoringAlgorithm<G::Vertex> for ClusteringCoefficient<'_, G> {
    /// Get a map with the local clustering coefficients of all vertices
    fn scores(&self) -> &HashMap<G::Vertex, f64> {
        &self.scores
    }
}

const MAX_ITERATIONS_DEFAULT: usize = 100;
const TOLERANCE_DEFAULT: f64 = 0.0001;

fn invalid(msg: &str) -> GraphError {
    GraphError::InvalidArgument(msg.into())
}

/// Weighted incoming neighbours of every vertex as indices, in the graph's iteration order.
fn incoming_lists<G: IncomingGraph>(g: &G, vs: &[&G::Vertex]) -> Vec<Vec<(usize, f64)>> {
    let idx: HashMap<&G::Vertex, usize> = vs.iter().enumerate().map(|(i, v)| (*v, i)).collect();
    vs.iter()
        .map(|v| {
            g.in_edges_of(v)
                .map(|e| (idx[g.opposite_vertex(e, v).unwrap()], g.weight_of(e)))
                .collect()
        })
        .collect()
}

/// Eigenvector-centrality implementation.
///
/// Eigenvector centrality, introduced in 1895 by Edmund Landau for chess tournaments, associates
/// with a (weighted) graph the left dominant eigenvector of its adjacency matrix. More information
/// can be found on [wikipedia](https://en.wikipedia.org/wiki/Eigenvector_centrality).
///
/// This is a simple iterative implementation of the
/// [power method](https://en.wikipedia.org/wiki/Power_iteration) which stops after a
/// given number of iterations or if centrality values between two iterations do not change more than
/// a predefined value (technically, we stop when the ℓ_2 norm of the difference
/// between the current estimate and the next one drops below a given threshold). Correspondingly,
/// the result will be ℓ_2-normalized.
///
/// Each iteration of the algorithm runs in linear time O(n+m) when n is the number of nodes and m
/// the number of edges of the graph. The maximum number of iterations can be adjusted by the caller.
/// The default value is `EigenvectorCentrality#MAX_ITERATIONS_DEFAULT`. Also in case of
/// weighted graphs, negative weights are not expected.
///
/// # Rust notes
///
/// Eigenvector centrality by the power method (`EigenvectorCentrality`).
pub struct EigenvectorCentrality<V> {
    scores: HashMap<V, f64>,
}

impl<V: Eq + std::hash::Hash + Clone> EigenvectorCentrality<V> {
    /// Create and execute an instance of EigenvectorCentrality
    pub fn new<G: Graph<Vertex = V> + IncomingGraph>(g: &G) -> Self {
        Self::with_options(g, MAX_ITERATIONS_DEFAULT, TOLERANCE_DEFAULT).expect("default options")
    }

    pub fn with_options<G: Graph<Vertex = V> + IncomingGraph>(
        g: &G,
        max_iterations: usize,
        tolerance: f64,
    ) -> Result<Self, GraphError> {
        if max_iterations == 0 {
            return Err(GraphError::must_be_positive("maximum_iterations"));
        }
        if tolerance <= 0.0 {
            return Err(GraphError::must_be_positive("tolerance"));
        }
        let vs: Vec<&V> = g.vertices().collect();
        let n = vs.len();
        let incoming = incoming_lists(g, &vs);
        let mut scores = vec![(1.0 / n as f64).sqrt(); n];
        let mut next = vec![0.0; n];
        let mut l2_norm = tolerance;
        let mut remaining = max_iterations;
        while remaining > 0 && l2_norm >= tolerance {
            let mut sum_of_squares = 0.0;
            for v in 0..n {
                let value = incoming[v]
                    .iter()
                    .fold(0.0, |a, &(w, wt)| a + scores[w] * wt);
                sum_of_squares += value * value;
                next[v] = value;
            }
            let factor = 1.0 / f64::sqrt(sum_of_squares);
            let mut diffs2 = 0.0;
            for v in 0..n {
                next[v] *= factor;
                let d = scores[v] - next[v];
                diffs2 += d * d;
            }
            std::mem::swap(&mut scores, &mut next);
            l2_norm = diffs2.sqrt();
            remaining -= 1;
        }
        Ok(EigenvectorCentrality {
            scores: vs.into_iter().cloned().zip(scores).collect(),
        })
    }
}

impl<V> VertexScoringAlgorithm<V> for EigenvectorCentrality<V> {
    /// Get a map with the scores of all vertices
    fn scores(&self) -> &HashMap<V, f64> {
        &self.scores
    }
}

/// Katz centrality implementation.
///
/// The [wikipedia](https://en.wikipedia.org/wiki/Katz_centrality) article contains a nice
/// description of Katz centrality. Every path coming into a node contributes to its Katz centrality
/// by α^*k*, where α is the *damping factor* and *k*
/// is the length of the path.
///
/// This is a simple iterative implementation of Katz centrality which stops after a given number of
/// iterations or if the Katz centrality values between two iterations do not change more than a
/// predefined value. Each iteration increases the length of the paths contributing to the centrality
/// value. Note that unless the damping factor is smaller than the reciprocal of the
/// [spectral radius](https://en.wikipedia.org/wiki/Spectral_radius) of the adjacency
/// matrix, the computation will not converge.
///
/// This implementation makes it possible to provide an exogenous factor in the form of a
/// `ToDoubleFunction` mapping each vertex to its exogenous score. Each path is then multiplied
/// by the exogenous score of its starting vertex. The `default exogenous function` maps all vertices to one, as in standard Katz centrality.
///
/// Each iteration of the algorithm runs in linear time O(n+m) when n is the number of nodes and m
/// the number of edges of the graph. The maximum number of iterations can be adjusted by the caller.
/// The default value is `KatzCentrality#MAX_ITERATIONS_DEFAULT`. Also in case of weighted
/// graphs, negative weights are not expected.
///
/// # Rust notes
///
/// Katz centrality (`KatzCentrality`): `x = damping * A^T x + exogenous`.
pub struct KatzCentrality<V> {
    scores: HashMap<V, f64>,
}

impl<V: Eq + std::hash::Hash + Clone> KatzCentrality<V> {
    /// Create and execute an instance of KatzCentrality.
    pub fn new<G: Graph<Vertex = V> + IncomingGraph>(g: &G) -> Self {
        Self::with_damping(g, 0.01).expect("default options")
    }

    pub fn with_damping<G: Graph<Vertex = V> + IncomingGraph>(
        g: &G,
        damping_factor: f64,
    ) -> Result<Self, GraphError> {
        Self::with_options(
            g,
            damping_factor,
            |_| 1.0,
            MAX_ITERATIONS_DEFAULT,
            TOLERANCE_DEFAULT,
        )
    }

    pub fn with_options<G: Graph<Vertex = V> + IncomingGraph>(
        g: &G,
        damping_factor: f64,
        exogenous_factor: impl Fn(&V) -> f64,
        max_iterations: usize,
        tolerance: f64,
    ) -> Result<Self, GraphError> {
        if max_iterations == 0 {
            return Err(GraphError::must_be_positive("maximum_iterations"));
        }
        if damping_factor < 0.0 {
            return Err(invalid("Damping factor not valid"));
        }
        if tolerance <= 0.0 {
            return Err(GraphError::must_be_positive("tolerance"));
        }
        let vs: Vec<&V> = g.vertices().collect();
        let n = vs.len();
        let incoming = incoming_lists(g, &vs);
        let exo: Vec<f64> = vs.iter().map(|v| exogenous_factor(v)).collect();
        let mut scores = exo.clone();
        let mut next = vec![0.0; n];
        let mut max_change = tolerance;
        let mut remaining = max_iterations;
        while remaining > 0 && max_change >= tolerance {
            max_change = 0.0;
            for v in 0..n {
                let contribution = incoming[v]
                    .iter()
                    .fold(0.0, |a, &(w, wt)| a + damping_factor * scores[w] * wt);
                let new = contribution + exo[v];
                max_change = f64::max(max_change, (new - scores[v]).abs());
                next[v] = new;
            }
            std::mem::swap(&mut scores, &mut next);
            remaining -= 1;
        }
        Ok(KatzCentrality {
            scores: vs.into_iter().cloned().zip(scores).collect(),
        })
    }
}

impl<V> VertexScoringAlgorithm<V> for KatzCentrality<V> {
    /// Get a map with the scores of all vertices
    fn scores(&self) -> &HashMap<V, f64> {
        &self.scores
    }
}

/// PageRank implementation.
///
/// The [wikipedia](https://en.wikipedia.org/wiki/PageRank) article contains a nice
/// description of PageRank. The method can be found on the article: Sergey Brin and Larry Page: The
/// Anatomy of a Large-Scale Hypertextual Web Search Engine. Proceedings of the 7th World-Wide Web
/// Conference, Brisbane, Australia, April 1998. See also the following
/// [page](http://infolab.stanford.edu/~backrub/google.html).
///
/// This is a simple iterative implementation of PageRank which stops after a given number of
/// iterations or if the PageRank values between two iterations do not change more than a predefined
/// value. The implementation uses the variant which divides by the number of nodes, thus forming a
/// probability distribution over graph nodes.
///
/// Each iteration of the algorithm runs in linear time $O(n+m)$ when $n$ is the number of nodes and
/// $m$ the number of edges of the graph. The maximum number of iterations can be adjusted by the
/// caller. The default value is `PageRank#MAX_ITERATIONS_DEFAULT`.
///
/// If the graph is a weighted graph, a weighted variant is used where the probability of following
/// an edge e out of node $v$ is equal to the weight of $e$ over the sum of weights of all outgoing
/// edges of $v$.
///
/// # Rust notes
///
/// PageRank (`PageRank`), weighted when the graph is.
pub struct PageRank<V> {
    scores: HashMap<V, f64>,
}

impl<V: Eq + std::hash::Hash + Clone> PageRank<V> {
    /// Create and execute an instance of PageRank.
    pub fn new<G: Graph<Vertex = V> + OutgoingGraph + IncomingGraph>(g: &G) -> Self {
        Self::with_options(g, 0.85, MAX_ITERATIONS_DEFAULT, TOLERANCE_DEFAULT)
            .expect("default options")
    }

    pub fn with_options<G: Graph<Vertex = V> + OutgoingGraph + IncomingGraph>(
        g: &G,
        damping_factor: f64,
        max_iterations: usize,
        tolerance: f64,
    ) -> Result<Self, GraphError> {
        if max_iterations == 0 {
            return Err(GraphError::must_be_positive("maximum_iterations"));
        }
        if !(0.0..=1.0).contains(&damping_factor) {
            return Err(invalid("Damping factor not valid"));
        }
        if tolerance <= 0.0 {
            return Err(GraphError::must_be_positive("tolerance"));
        }
        let vs: Vec<&V> = g.vertices().collect();
        let n = vs.len();
        let weighted = g.graph_type().weighted;
        let out_degree: Vec<usize> = vs.iter().map(|v| g.out_degree_of(v)).collect();
        let incoming = incoming_lists(g, &vs);
        let mut weight_sum = vec![0.0; n];
        if weighted {
            for list in &incoming {
                for &(w, wt) in list {
                    weight_sum[w] += wt;
                }
            }
        }
        let mut cur = vec![1.0 / n as f64; n];
        let mut next = vec![0.0; n];
        let mut max_change = tolerance;
        let mut iterations = max_iterations;
        while iterations > 0 && max_change >= tolerance {
            let mut r = 0.0;
            for i in 0..n {
                r += if out_degree[i] > 0 {
                    (1.0 - damping_factor) * cur[i]
                } else {
                    cur[i]
                };
            }
            r /= n as f64;
            max_change = 0.0;
            for i in 0..n {
                let mut contribution = 0.0;
                for &(w, wt) in &incoming[i] {
                    contribution += if weighted {
                        damping_factor * cur[w] * wt / weight_sum[w]
                    } else {
                        damping_factor * cur[w] / out_degree[w] as f64
                    };
                }
                let new = r + contribution;
                max_change = f64::max(max_change, (new - cur[i]).abs());
                next[i] = new;
            }
            std::mem::swap(&mut cur, &mut next);
            iterations -= 1;
        }
        Ok(PageRank {
            scores: vs.into_iter().cloned().zip(cur).collect(),
        })
    }
}

impl<V> VertexScoringAlgorithm<V> for PageRank<V> {
    /// Get a map with the scores of all vertices
    fn scores(&self) -> &HashMap<V, f64> {
        &self.scores
    }
}

/// Betweenness centrality.
///
/// Computes the betweenness centrality of each vertex of a graph. The betweenness centrality of a
/// node $v$ is given by the expression: $g(v)= \sum_{s \neq v \neq
/// t}\frac{\sigma_{st}(v)}{\sigma_{st}}$ where $\sigma_{st}$ is the total number of shortest paths
/// from node $s$ to node $t$ and $\sigma_{st}(v)$ is the number of those paths that pass through
/// $v$. For more details see
/// [wikipedia](https://en.wikipedia.org/wiki/Betweenness_centrality).
///
/// The algorithm is based on
///
/// - Brandes, Ulrik (2001). "A faster algorithm for betweenness centrality". Journal of
///   Mathematical Sociology. 25 (2): 163–177.
///
/// The running time is $O(nm)$ and $O(nm +n^2 \log n)$ for unweighted and weighted graph
/// respectively, where $n$ is the number of vertices and $m$ the number of edges of the graph. The
/// space complexity is $O(n + m)$.
///
/// Note that this running time assumes that arithmetic is performed between numbers whose
/// representation needs a number of bits which is logarithmic in the instance size. There are
/// instances where this is not true and path counters might grow super exponential. This class
/// allows the user to adjust whether an exception is thrown in case overflow occurs. Default
/// behavior is to ignore overflow issues.
///
/// # Rust notes
///
/// Vertex betweenness centrality by Brandes' algorithm (`BetweennessCentrality`). Weighted
/// graphs (by their `GraphType`) use a pairing heap, others a plain queue. Undirected scores are
/// halved; with `normalize` they are divided by `(n - 1)(n - 2)`.
pub struct BetweennessCentrality<V> {
    scores: HashMap<V, f64>,
}

impl<V: Eq + std::hash::Hash + Clone> BetweennessCentrality<V> {
    /// Construct a new instance.
    pub fn new<G: Graph<Vertex = V> + OutgoingGraph>(g: &G) -> Result<Self, GraphError> {
        Self::with_options(g, false, OverflowStrategy::IgnoreOverflow)
    }

    pub fn normalized<G: Graph<Vertex = V> + OutgoingGraph>(g: &G) -> Result<Self, GraphError> {
        Self::with_options(g, true, OverflowStrategy::IgnoreOverflow)
    }

    pub fn with_options<G: Graph<Vertex = V> + OutgoingGraph>(
        g: &G,
        normalize: bool,
        overflow: OverflowStrategy,
    ) -> Result<Self, GraphError> {
        let vs: Vec<&V> = g.vertices().collect();
        let n = vs.len();
        let idx: HashMap<&V, usize> = vs.iter().enumerate().map(|(i, v)| (*v, i)).collect();
        let out: Vec<Vec<(usize, f64)>> = vs
            .iter()
            .map(|v| {
                Ok(g.out_edges_of(v)
                    .map(|e| (idx[g.opposite_vertex(e, v).unwrap()], g.weight_of(e)))
                    .collect())
            })
            .collect::<Result<_, GraphError>>()?;
        let weighted = g.graph_type().weighted;
        let mut scores = vec![0.0; n];
        for s in 0..n {
            Self::accumulate(s, &out, weighted, overflow, &mut scores)?;
        }
        if !g.graph_type().directed {
            scores.iter_mut().for_each(|x| *x /= 2.0);
        }
        if normalize {
            let factor = (n as i64 - 1) * (n as i64 - 2);
            if factor != 0 {
                scores.iter_mut().for_each(|x| *x /= factor as f64);
            }
        }
        Ok(BetweennessCentrality {
            scores: vs.into_iter().cloned().zip(scores).collect(),
        })
    }

    fn accumulate(
        s: usize,
        out: &[Vec<(usize, f64)>],
        weighted: bool,
        overflow: OverflowStrategy,
        scores: &mut [f64],
    ) -> Result<(), GraphError> {
        let n = out.len();
        let mut stack: Vec<usize> = Vec::new();
        let mut predecessors: Vec<Vec<usize>> = vec![Vec::new(); n];
        let mut sigma = vec![0i64; n];
        sigma[s] = 1;
        let mut distance = vec![f64::INFINITY; n];
        distance[s] = 0.0;
        let mut fifo = VecDeque::from([s]);
        let mut heap: PairingHeap<OrderedF64, usize> = PairingHeap::new();
        let mut handles: HashMap<usize, _> = HashMap::new();
        if weighted {
            handles.insert(s, heap.insert(OrderedF64(0.0), s));
        }
        loop {
            let v = if weighted {
                match heap.pop() {
                    Some((_, v)) => v,
                    None => break,
                }
            } else {
                match fifo.pop_front() {
                    Some(v) => v,
                    None => break,
                }
            };
            stack.push(v);
            for &(w, e_weight) in &out[v] {
                if e_weight < 0.0 {
                    return Err(GraphError::InvalidWeight(crate::WeightProblem::Negative));
                }
                let d = distance[v] + e_weight;
                if distance[w] == f64::INFINITY {
                    if weighted {
                        handles.insert(w, heap.insert(OrderedF64(d), w));
                    } else {
                        fifo.push_back(w);
                    }
                    distance[w] = d;
                    sigma[w] = sigma[v];
                    predecessors[w].push(v);
                } else if distance[w] == d {
                    let sum = sigma[w].wrapping_add(sigma[v]);
                    if overflow == OverflowStrategy::ThrowExceptionOnOverflow && sum < 0 {
                        return Err(GraphError::Overflow("long overflow".into()));
                    }
                    sigma[w] = sum;
                    predecessors[w].push(v);
                } else if distance[w] > d {
                    if weighted {
                        heap.decrease_key(handles[&w], OrderedF64(d)).ok();
                    }
                    distance[w] = d;
                    sigma[w] = sigma[v];
                    predecessors[w].clear();
                    predecessors[w].push(v);
                }
            }
        }
        let mut dependency = vec![0.0; n];
        while let Some(w) = stack.pop() {
            for &v in &predecessors[w] {
                dependency[v] += sigma[v] as f64 / sigma[w] as f64 * (1.0 + dependency[w]);
            }
            if w != s {
                scores[w] += dependency[w];
            }
        }
        Ok(())
    }
}

impl<V> VertexScoringAlgorithm<V> for BetweennessCentrality<V> {
    /// Get a map with the scores of all vertices
    fn scores(&self) -> &HashMap<V, f64> {
        &self.scores
    }
}

/// Betweenness centrality with arbitrary precision arithmetic.
///
/// Computes the betweenness centrality of each vertex of a graph. The betweenness centrality of a
/// node $v$ is given by the expression: $g(v)= \sum_{s \neq v \neq
/// t}\frac{\sigma_{st}(v)}{\sigma_{st}}$ where $\sigma_{st}$ is the total number of shortest paths
/// from node $s$ to node $t$ and $\sigma_{st}(v)$ is the number of those paths that pass through
/// $v$. For more details see
/// [wikipedia](https://en.wikipedia.org/wiki/Betweenness_centrality).
///
/// The algorithm is based on
///
/// - Brandes, Ulrik (2001). "A faster algorithm for betweenness centrality". Journal of
///   Mathematical Sociology. 25 (2): 163–177.
///
/// The running time is $O(nm)$ and $O(nm +n^2 \log n)$ for unweighted and weighted graph
/// respectively, where $n$ is the number of vertices and $m$ the number of edges of the graph. The
/// space complexity is $O(n + m)$.
///
/// Note that this running time assumes that arithmetic is performed between numbers whose
/// representation needs a number of bits which is logarithmic in the instance size. There are
/// instances where this is not true, and thus it is not safe to assume that arithmetic takes
/// constant time.
///
/// This class uses arbitrary precision arithmetic (except for the execution of Dijkstra's
/// algorithm). The precision can be adjusted by the constructor parameters.
///
/// # Rust notes
///
/// Java-compatible arbitrary-precision betweenness centrality (`ApBetweennessCentrality`).
///
/// Scores use a pure-Rust [`dashu_float::FBig`] with radix 10 and rounding toward zero, matching
/// Java Apfloat's arithmetic technique and significant-decimal-digit precision. The default is
/// [`APFLOAT_INFINITE_PRECISION`]; as in Apfloat, that mode accepts exact terminating decimal
/// divisions and rejects non-terminating expansions. Finite precision bounds number size and
/// arithmetic cost, while unlimited precision can require representations larger than logarithmic
/// in the graph size.
pub struct ApBetweennessCentrality<V> {
    scores: HashMap<V, ApFloat>,
}

/// Decimal arbitrary-precision value used by [`ApBetweennessCentrality`].
pub type ApFloat = dashu_float::FBig<dashu_float::round::mode::Zero, 10>;

/// Sentinel selecting Apfloat-compatible unlimited precision.
pub const APFLOAT_INFINITE_PRECISION: usize = usize::MAX;

fn ap_precision(precision: usize) -> Result<usize, GraphError> {
    match precision {
        0 => Err(GraphError::must_be_positive("precision")),
        APFLOAT_INFINITE_PRECISION => Ok(0),
        finite => Ok(finite),
    }
}

fn ap_integer(value: i64, precision: usize) -> ApFloat {
    ApFloat::from(value).with_precision(precision).value()
}

fn gcd(mut a: dashu_int::UBig, mut b: dashu_int::UBig) -> dashu_int::UBig {
    while b != dashu_int::UBig::ZERO {
        let remainder = &a % &b;
        a = b;
        b = remainder;
    }
    a
}

/// Divide two non-negative decimal floats using Apfloat's unlimited-precision rule.
fn ap_divide(lhs: &ApFloat, rhs: &ApFloat, precision: usize) -> Result<ApFloat, GraphError> {
    if precision != 0 {
        return Ok(lhs.clone() / rhs.clone());
    }
    if rhs.repr().significand() == &dashu_int::IBig::ZERO {
        return Err(invalid("Division by zero"));
    }
    if lhs.repr().significand() == &dashu_int::IBig::ZERO {
        return Ok(ap_integer(0, 0));
    }

    // Brandes path counts and dependencies are non-negative, so their significands convert
    // directly to UBig. Reduce the fraction before testing whether its decimal expansion ends.
    let mut numerator = dashu_int::UBig::try_from(lhs.repr().significand().clone())
        .expect("betweenness operands are non-negative");
    let mut denominator = dashu_int::UBig::try_from(rhs.repr().significand().clone())
        .expect("betweenness operands are non-negative");
    let common = gcd(numerator.clone(), denominator.clone());
    numerator /= &common;
    denominator /= common;

    let mut twos = 0usize;
    while &denominator % 2u8 == 0 {
        denominator /= 2u8;
        twos += 1;
    }
    let mut fives = 0usize;
    while &denominator % 5u8 == 0 {
        denominator /= 5u8;
        fives += 1;
    }
    if denominator != dashu_int::UBig::ONE {
        return Err(invalid(
            "Non-terminating decimal expansion at infinite precision",
        ));
    }

    let scale = twos.max(fives);
    numerator *= dashu_int::UBig::from(2u8).pow(scale - twos);
    numerator *= dashu_int::UBig::from(5u8).pow(scale - fives);
    let exponent = lhs
        .repr()
        .exponent()
        .checked_sub(rhs.repr().exponent())
        .and_then(|e| e.checked_sub(scale as isize))
        .ok_or_else(|| GraphError::Overflow("decimal exponent does not fit isize".into()))?;
    Ok(ApFloat::from_repr(
        dashu_float::Repr::new(numerator.into(), exponent),
        dashu_float::Context::new(0),
    ))
}

impl<V: Eq + std::hash::Hash + Clone> ApBetweennessCentrality<V> {
    /// Construct a new instance.
    pub fn new<G: Graph<Vertex = V> + OutgoingGraph>(g: &G) -> Result<Self, GraphError> {
        Self::with_normalize(g, false)
    }

    pub fn with_normalize<G: Graph<Vertex = V> + OutgoingGraph>(
        g: &G,
        normalize: bool,
    ) -> Result<Self, GraphError> {
        Self::with_precision(g, normalize, APFLOAT_INFINITE_PRECISION)
    }

    /// Construct with an Apfloat-compatible number of significant decimal digits.
    pub fn with_precision<G: Graph<Vertex = V> + OutgoingGraph>(
        g: &G,
        normalize: bool,
        precision: usize,
    ) -> Result<Self, GraphError> {
        let precision = ap_precision(precision)?;

        let vs: Vec<&V> = g.vertices().collect();
        let n = vs.len();
        let idx: HashMap<&V, usize> = vs.iter().enumerate().map(|(i, v)| (*v, i)).collect();
        let out: Vec<Vec<(usize, f64)>> = vs
            .iter()
            .map(|v| {
                Ok(g.out_edges_of(v)
                    .map(|e| (idx[g.opposite_vertex(e, v).unwrap()], g.weight_of(e)))
                    .collect())
            })
            .collect::<Result<_, GraphError>>()?;
        let weighted = g.graph_type().weighted;
        let zero = ap_integer(0, precision);
        let one = ap_integer(1, precision);
        let two = ap_integer(2, precision);
        let mut scores = vec![zero.clone(); n];
        for s in 0..n {
            let mut stack: Vec<usize> = Vec::new();
            let mut predecessors: Vec<Vec<usize>> = vec![Vec::new(); n];
            let mut sigma = vec![zero.clone(); n];
            sigma[s] = one.clone();
            let mut distance = vec![f64::INFINITY; n];
            distance[s] = 0.0;
            let mut fifo = VecDeque::from([s]);
            let mut heap: PairingHeap<OrderedF64, usize> = PairingHeap::new();
            let mut handles = HashMap::new();
            if weighted {
                handles.insert(s, heap.insert(OrderedF64(0.0), s));
            }
            loop {
                let next = if weighted {
                    heap.pop().map(|(_, v)| v)
                } else {
                    fifo.pop_front()
                };
                let Some(v) = next else { break };
                stack.push(v);
                for &(w, e_weight) in &out[v] {
                    if e_weight < 0.0 {
                        return Err(GraphError::InvalidWeight(crate::WeightProblem::Negative));
                    }
                    let d = distance[v] + e_weight;
                    if distance[w] == f64::INFINITY {
                        if weighted {
                            handles.insert(w, heap.insert(OrderedF64(d), w));
                        } else {
                            fifo.push_back(w);
                        }
                        distance[w] = d;
                        sigma[w] = sigma[v].clone();
                        predecessors[w].push(v);
                    } else if distance[w] == d {
                        let paths_via_v = sigma[v].clone();
                        sigma[w] += paths_via_v;
                        predecessors[w].push(v);
                    } else if distance[w] > d {
                        if weighted {
                            heap.decrease_key(handles[&w], OrderedF64(d)).ok();
                        }
                        distance[w] = d;
                        sigma[w] = sigma[v].clone();
                        predecessors[w].clear();
                        predecessors[w].push(v);
                    }
                }
            }
            let mut dependency = vec![zero.clone(); n];
            while let Some(w) = stack.pop() {
                for &v in &predecessors[w] {
                    let ratio = ap_divide(&sigma[v], &sigma[w], precision)?;
                    let add = ratio * (dependency[w].clone() + one.clone());
                    dependency[v] += add;
                }
                if w != s {
                    let d = dependency[w].clone();
                    scores[w] += d;
                }
            }
        }
        if !g.graph_type().directed {
            for score in &mut scores {
                *score = ap_divide(score, &two, precision)?;
            }
        }
        if normalize && n > 2 {
            let nf = (n as i64 - 1).checked_mul(n as i64 - 2).ok_or_else(|| {
                GraphError::Overflow("normalization factor does not fit i64".into())
            })?;
            let nf = ap_integer(nf, precision);
            for score in &mut scores {
                *score = ap_divide(score, &nf, precision)?;
            }
        }
        Ok(ApBetweennessCentrality {
            scores: vs.into_iter().cloned().zip(scores).collect(),
        })
    }
}

impl<V: Eq + std::hash::Hash> ApBetweennessCentrality<V> {
    /// Scores computed using the configured decimal precision.
    pub fn scores(&self) -> &HashMap<V, ApFloat> {
        &self.scores
    }

    /// Backwards-compatible name retained from the former exact-rational implementation.
    pub fn exact_scores(&self) -> &HashMap<V, ApFloat> {
        self.scores()
    }
}

impl<V> VertexScoringAlgorithm<V, ApFloat> for ApBetweennessCentrality<V> {
    /// Get a map with the scores of all vertices
    fn scores(&self) -> &HashMap<V, ApFloat> {
        &self.scores
    }
}
