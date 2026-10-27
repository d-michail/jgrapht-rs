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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverflowStrategy {
    IgnoreOverflow,
    ThrowExceptionOnOverflow,
}

/// Edge betweenness: for every edge the sum over source/target pairs of the fraction of shortest
/// paths through it. Graphs with parallel edges are rejected.
pub struct EdgeBetweennessCentrality<'g, G: Graph> {
    graph: &'g G,
    start_vertices: Option<Vec<G::Vertex>>,
    overflow: OverflowStrategy,
    scores: Option<IndexMap<G::Edge, f64>>,
}

impl<'g, G: Graph> EdgeBetweennessCentrality<'g, G> {
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
            return Err(GraphError::InvalidArgument(
                "Graphs with multiple edges not supported".into(),
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

    pub fn edge_score(&mut self, e: &G::Edge) -> Result<f64, GraphError> {
        if !self.graph.contains_edge(e) {
            return Err(GraphError::InvalidArgument(
                "Cannot return score of unknown edge".into(),
            ));
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
                let e_weight = g.edge_weight(e).unwrap_or(1.0);
                if e_weight < 0.0 {
                    return Err(GraphError::InvalidArgument(
                        "Negative edge weights are not allowed".into(),
                    ));
                }
                let new_distance = v_distance + e_weight;
                let current: Option<f64> = match handles.get(w) {
                    None => None,
                    Some(h) => Some(
                        final_dist
                            .get(w)
                            .copied()
                            .unwrap_or_else(|| heap.key(*h).map(|k| k.0).unwrap_or(f64::INFINITY)),
                    ),
                };
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
