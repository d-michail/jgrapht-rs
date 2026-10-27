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

//! Graph clustering.

use crate::graph::OutgoingGraph;
use crate::graph::IncidenceGraph;
use super::spanning::kruskal;
use super::util::Indexed;
use crate::graph::Graph;
use crate::util::UnionFind;
use crate::{GraphError, Result};
use rand::Rng;
use rand::seq::SliceRandom;
use std::collections::HashMap;

fn groups<G: Graph>(ix: &Indexed<'_, G>, label: &[usize]) -> Vec<Vec<G::Vertex>> {
    let mut by: HashMap<usize, Vec<usize>> = HashMap::new();
    for (v, &l) in label.iter().enumerate() {
        by.entry(l).or_default().push(v);
    }
    let mut cs: Vec<Vec<usize>> = by.into_values().collect();
    cs.sort_by_key(|c| c[0]);
    cs.iter().map(|c| ix.vertices_of(c)).collect()
}

/// Asynchronous label propagation (Raghavan et al.): every vertex repeatedly adopts the most
/// frequent label among its neighbours (edge weights count), ties broken at random.
pub fn label_propagation<G: Graph>(
    g: &G,
    max_iterations: usize,
    rng: &mut impl Rng,
) -> Vec<Vec<G::Vertex>> {
    let ix = Indexed::new(g);
    let n = ix.n();
    let mut label: Vec<usize> = (0..n).collect();
    let mut order: Vec<usize> = (0..n).collect();
    for _ in 0..max_iterations {
        order.shuffle(rng);
        let mut changed = false;
        for &v in &order {
            let mut score: HashMap<usize, f64> = HashMap::new();
            for &(u, k) in ix.out[v]
                .iter()
                .chain(if ix.directed { &ix.inn[v][..] } else { &[] })
            {
                if u != v {
                    *score.entry(label[u]).or_default() += ix.w[k];
                }
            }
            let Some(best) = score.values().copied().max_by(|a, b| a.total_cmp(b)) else {
                continue;
            };
            let ties: Vec<usize> = score
                .iter()
                .filter(|(_, s)| **s == best)
                .map(|(l, _)| *l)
                .collect();
            if !ties.contains(&label[v]) {
                label[v] = ties[rng.random_range(0..ties.len())];
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    groups(&ix, &label)
}

/// k-spanning-tree clustering: build a minimum spanning forest and cut its heaviest edges
/// until `k` clusters remain.
pub fn k_spanning_tree_clustering<G: Graph>(g: &G, k: usize) -> Result<Vec<Vec<G::Vertex>>> {
    let ix = Indexed::new(g);
    if k == 0 || k > ix.n() {
        return Err(GraphError::InvalidArgument(
            "k must be between 1 and the number of vertices".into(),
        ));
    }
    let tree = kruskal(g);
    let mut edges: Vec<usize> = tree
        .edges
        .iter()
        .map(|e| ix.es.iter().position(|x| *x == e).unwrap())
        .collect();
    edges.sort_by(|&a, &b| ix.w[a].total_cmp(&ix.w[b]));
    let mut uf = UnionFind::new(ix.n());
    for &e in &edges {
        uf.union(ix.ends[e].0, ix.ends[e].1);
    }
    // the forest may already have more than one component; cut heaviest edges until k remain
    let mut keep = edges.len();
    while uf.set_count() < k && keep > 0 {
        keep -= 1;
        uf = UnionFind::new(ix.n());
        for &e in &edges[..keep] {
            uf.union(ix.ends[e].0, ix.ends[e].1);
        }
    }
    if uf.set_count() > k {
        return Err(GraphError::InvalidArgument(
            "graph has more components than k".into(),
        ));
    }
    let label: Vec<usize> = (0..ix.n()).map(|v| uf.find(v)).collect();
    Ok(groups(&ix, &label))
}

// ---------------------------------------------------------------------------------------------
// JGraphT's clustering classes

use super::interfaces::{Clustering, ClusteringAlgorithm};
use super::scoring::{EdgeBetweennessCentrality, OverflowStrategy};
use crate::graph::{GraphExt, props};
use crate::util::OrderedF64;
use crate::util::java::{
    RandomSource, java_hash_order, java_hash_set_order, java_shuffle, java_string_hash,
};
use crate::view::MaskSubgraph;
use indexmap::{IndexMap, IndexSet};
use rheaps::AddressableHeap;
use rheaps::tree::PairingHeap;
use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet, HashSet};

/// A [modularity](https://en.wikipedia.org/wiki/Modularity_(networks)) measurer.
///
/// This is a utility class which computes the modularity function. It takes as input a list of
/// vertex classes $C$ and a graph $G$ and calculates: $Q = \frac{1}{2m} \sum_{ij} \left( A_{ij} -
/// \frac{k_i k_j}{2m} \right) \delta(C_i, C_j)$. Here $m$ is the total number of edges and $k_i$ is
/// the degree of vertex $i$. $A_{ij}$ is either $1$ or $0$ depending on whether edge $(i,j)$ belongs
/// to the graph and $\delta(C_i, C_j)$ is 1 if vertices $i$ and $j$ belong to the same class, $0$
/// otherwise.
///
/// # Rust notes
///
/// Modularity of a partition of an undirected (possibly weighted) graph
/// (`UndirectedModularityMeasurer`).
pub struct UndirectedModularityMeasurer<'g, G: Graph> {
    graph: &'g G,
    m: f64,
    degrees: HashMap<G::Vertex, f64>,
}

impl<'g, G: IncidenceGraph> UndirectedModularityMeasurer<'g, G> {
    /// Construct a new measurer
    pub fn new(graph: &'g G) -> Result<Self> {
        props::require_undirected(graph)?;
        let mut degrees = HashMap::new();
        let m;
        if graph.graph_type().weighted {
            m = graph
                .edges()
                .map(|e| graph.edge_weight(e).unwrap_or(1.0))
                .sum();
            for v in graph.vertices() {
                let mut sum = 0.0;
                for e in graph.out_edges_of(v) {
                    let u = graph.opposite_vertex(e, v).unwrap();
                    let w = graph.edge_weight(e).unwrap_or(1.0);
                    sum += if u == v { 2.0 * w } else { w };
                }
                degrees.insert(v.clone(), sum);
            }
        } else {
            m = graph.edge_count() as f64;
            for v in graph.vertices() {
                degrees.insert(v.clone(), graph.degree_of(v) as f64);
            }
        }
        Ok(UndirectedModularityMeasurer { graph, m, degrees })
    }

    /// The modularity of `partitions`, which must cover every vertex.
    pub fn modularity(&self, partitions: &[HashSet<G::Vertex>]) -> Result<f64> {
        const INVALID: &str = "Invalid partition of vertices";
        let total = partitions.len();
        let mut vertex_partition: HashMap<&G::Vertex, usize> = HashMap::new();
        let mut weighted_degree = vec![0.0; total];
        for (p, partition) in partitions.iter().enumerate() {
            for v in partition {
                vertex_partition.insert(v, p);
                let d = self
                    .degrees
                    .get(v)
                    .ok_or_else(|| GraphError::InvalidArgument(INVALID.into()))?;
                weighted_degree[p] += d;
            }
        }
        let mut edge_weight = vec![0.0; total];
        for e in self.graph.edges() {
            let (v, u) = (
                self.graph.edge_source(e).unwrap(),
                self.graph.edge_target(e).unwrap(),
            );
            let pv = vertex_partition
                .get(v)
                .ok_or_else(|| GraphError::InvalidArgument(INVALID.into()))?;
            let pu = vertex_partition
                .get(u)
                .ok_or_else(|| GraphError::InvalidArgument(INVALID.into()))?;
            if pv == pu {
                edge_weight[*pv] += self.graph.edge_weight(e).unwrap_or(1.0);
            }
        }
        let mut modularity = 0.0;
        for p in 0..total {
            let expected = weighted_degree[p] * weighted_degree[p] / (2.0 * self.m);
            modularity += 2.0 * edge_weight[p] - expected;
        }
        Ok(modularity / (2.0 * self.m))
    }
}

/// The Girvan-Newman clustering algorithm.
///
/// The algorithm is described in: Girvan, Michelle, and Mark EJ Newman. "Community structure in
/// social and biological networks." Proceedings of the national academy of sciences 99.12 (2002):
/// 7821-7826.
///
/// Running time is $O(m^2 n)$ or $O(m^2n + m n^2 \log n)$ for weighted graphs.
///
/// # Rust notes
///
/// Girvan–Newman divisive clustering: removes the edge of highest betweenness until the graph
/// has `k` connected components (`GirvanNewmanClustering`).
pub struct GirvanNewmanClustering<'g, G: Graph> {
    graph: &'g G,
    k: usize,
    overflow: OverflowStrategy,
    start_vertices: Option<Vec<G::Vertex>>,
}

impl<'g, G: Graph> GirvanNewmanClustering<'g, G> {
    /// Create a new clustering algorithm.
    pub fn new(graph: &'g G, k: usize) -> Result<Self> {
        Self::with_options(graph, k, OverflowStrategy::ThrowExceptionOnOverflow, None)
    }

    pub fn with_options(
        graph: &'g G,
        k: usize,
        overflow: OverflowStrategy,
        start_vertices: Option<Vec<G::Vertex>>,
    ) -> Result<Self> {
        if k < 1 || k > graph.vertex_count() {
            return Err(GraphError::InvalidArgument(
                "Illegal number of clusters".into(),
            ));
        }
        Ok(GirvanNewmanClustering {
            graph,
            k,
            overflow,
            start_vertices,
        })
    }
}

impl<G: OutgoingGraph> ClusteringAlgorithm<G::Vertex> for GirvanNewmanClustering<'_, G> {
    /// Computes a clustering.
    fn clustering(&self) -> Result<Clustering<G::Vertex>> {
        let g = self.graph;
        let mut removed: HashSet<G::Edge> = HashSet::new();
        loop {
            let view =
                MaskSubgraph::new(g, |_: &G::Vertex| false, |e: &G::Edge| removed.contains(e));
            let ccs = super::connectivity::connected_components(&view);
            if ccs.len() == self.k {
                return Ok(Clustering::new(
                    ccs.into_iter().map(|c| c.into_iter().collect()).collect(),
                ));
            }
            let mut bc = EdgeBetweennessCentrality::with_options(
                &view,
                self.overflow,
                self.start_vertices.clone(),
            )?;
            let mut max_edge: Option<G::Edge> = None;
            let mut max_centrality = 0.0f64;
            for (e, &score) in bc.scores()? {
                if score.total_cmp(&max_centrality).is_gt() || max_edge.is_none() {
                    max_edge = Some(e.clone());
                    max_centrality = score;
                }
            }
            let Some(edge) = max_edge else {
                return Err(GraphError::InvalidArgument("no edge left to remove".into()));
            };
            drop(bc);
            removed.insert(edge);
        }
    }
}

/// The k spanning tree clustering algorithm.
///
/// The algorithm finds a minimum spanning tree $T$ using Prim's algorithm, then executes Kruskal's
/// algorithm only on the edges of $T$ until $k$ trees are formed. The resulting trees are the final
/// clusters. The total running time is $O(m + n \log n)$.
///
/// The algorithm is strongly related to single linkage cluster analysis, also known as single-link
/// clustering. For more information see: J. C. Gower and G. J. S. Ross. Minimum Spanning Trees and
/// Single Linkage Cluster Analysis. Journal of the Royal Statistical Society. Series C (Applied
/// Statistics), 18(1):54--64, 1969.
///
/// # Rust notes
///
/// Clustering by cutting the heaviest edges of a minimum spanning tree (`KSpanningTreeClustering`).
pub struct KSpanningTreeClustering<'g, G: Graph> {
    graph: &'g G,
    k: usize,
}

impl<'g, G: Graph> KSpanningTreeClustering<'g, G> {
    /// Create a new clustering algorithm.
    pub fn new(graph: &'g G, k: usize) -> Result<Self> {
        props::require_undirected(graph)?;
        if k < 1 || k > graph.vertex_count() {
            return Err(GraphError::InvalidArgument(
                "Illegal number of clusters".into(),
            ));
        }
        Ok(KSpanningTreeClustering { graph, k })
    }
}

impl<G: Graph> ClusteringAlgorithm<G::Vertex> for KSpanningTreeClustering<'_, G> {
    /// Computes a clustering.
    fn clustering(&self) -> Result<Clustering<G::Vertex>> {
        let g = self.graph;
        let mut edges = super::spanning::prim(g).edges;
        edges.sort_by(|a, b| {
            g.edge_weight(a)
                .unwrap_or(1.0)
                .total_cmp(&g.edge_weight(b).unwrap_or(1.0))
        });
        let mut forest = super::util::UnionFind::new(g.vertices().cloned());
        for edge in edges {
            if forest.number_of_sets() == self.k {
                break;
            }
            let (s, t) = (g.edge_source(&edge).unwrap(), g.edge_target(&edge).unwrap());
            if forest.find(s)? == forest.find(t)? {
                continue;
            }
            forest.union(s, t)?;
        }
        let mut cluster_map: IndexMap<G::Vertex, IndexSet<G::Vertex>> = IndexMap::new();
        for v in g.vertices() {
            let rv = forest.find(v)?;
            cluster_map.entry(rv).or_default().insert(v.clone());
        }
        Ok(Clustering::new(cluster_map.into_values().collect()))
    }
}

/// A naive implementation of greedy modularity maximization for community detection.
///
/// The algorithm partitions the vertices of an undirected graph into communities by greedily
/// maximizing the [modularity](https://en.wikipedia.org/wiki/Modularity_(networks)) of
/// possible communities. Greedy modularity maximization begins with each node in its own community
/// and repeatedly joins the pair of communities that lead to the largest modularity until no further
/// increase in modularity is possible (a maximum).
///
/// This implementation is simple but computationally expensive, with a worst-case complexity of
/// **O(n^4)**. It is intended as an easy-to-understand reference implementation rather than a
/// performance-optimized solution.
///
/// # Rust notes
///
/// Greedy agglomerative modularity maximisation by exhaustive pair merging, `O(n^3)` merges to
/// try (`NaiveGreedyModularityAlgorithm`); mostly useful as a reference.
pub struct NaiveGreedyModularityAlgorithm<'g, G: Graph> {
    graph: &'g G,
}

impl<'g, G: Graph> NaiveGreedyModularityAlgorithm<'g, G> {
    /// Create a new Naive Greedy clustering algorithm.
    pub fn new(graph: &'g G) -> Result<Self> {
        props::require_undirected(graph)?;
        Ok(NaiveGreedyModularityAlgorithm { graph })
    }
}

impl<G: IncidenceGraph> ClusteringAlgorithm<G::Vertex> for NaiveGreedyModularityAlgorithm<'_, G> {
    /// Computes a clustering.
    fn clustering(&self) -> Result<Clustering<G::Vertex>> {
        let mut communities: Vec<HashSet<G::Vertex>> = self
            .graph
            .vertices()
            .map(|v| HashSet::from([v.clone()]))
            .collect();
        let measurer = UndirectedModularityMeasurer::new(self.graph)?;
        let mut old: Option<f64> = None;
        let mut cur = measurer.modularity(&communities)?;
        while old.is_none_or(|o| o < cur) {
            old = Some(cur);
            let mut best: Option<Vec<HashSet<G::Vertex>>> = None;
            for i in 0..communities.len() {
                for j in 0..communities.len() {
                    if j <= i {
                        continue;
                    }
                    let mut trial: Vec<HashSet<G::Vertex>> = (0..communities.len())
                        .filter(|&k| k != i && k != j)
                        .map(|k| communities[k].clone())
                        .collect();
                    let merge: HashSet<G::Vertex> =
                        communities[i].union(&communities[j]).cloned().collect();
                    trial.push(merge);
                    let modularity = measurer.modularity(&trial)?;
                    if modularity >= cur {
                        cur = modularity;
                        best = Some(trial);
                    }
                }
            }
            match best {
                None => break,
                Some(b) => communities = b,
            }
        }
        Ok(Clustering::new(
            communities
                .into_iter()
                .map(|c| c.into_iter().collect())
                .collect(),
        ))
    }
}

type MaxHeap = PairingHeap<Reverse<OrderedF64>, (usize, usize)>;
type Handle = <MaxHeap as AddressableHeap<Reverse<OrderedF64>, (usize, usize)>>::Handle;

/// The Greedy Modularity algorithm.
///
/// The algorithm partitions the vertices of an undirected graph into communities by greedily
/// maximizing the [modularity](https://en.wikipedia.org/wiki/Modularity_(networks)) of
/// possible communities. Greedy modularity maximization begins with each node in its own community
/// and repeatedly joins the pair of communities that lead to the largest modularity until no further
/// increase in modularity is possible (a maximum).
///
/// The algorithm is due to Clauset, Newman and Moore. It is described in detail in the following
/// [paper](https://doi.org/10.1103/PhysRevE.70.066111):
///
/// - Clauset, A., Newman, M. E., & Moore, C. “Finding community structure in very large
/// networks.” Physical Review E 70(6), 2004.
///
/// # Rust notes
///
/// Clauset–Newman–Moore greedy modularity maximisation with heaps (`GreedyModularityAlgorithm`),
/// for directed and undirected graphs.
///
/// Communities are processed in graph vertex order wherever JGraphT iterates Java hash or tree
/// sets of (integer) vertices, and the result lists them by the position of their representative.
pub struct GreedyModularityAlgorithm<'g, G: Graph> {
    graph: &'g G,
    java_hash: Option<Box<dyn Fn(&G::Vertex) -> i32 + 'g>>,
}

impl<'g, G: Graph> GreedyModularityAlgorithm<'g, G> {
    /// Create a new clustering algorithm.
    pub fn new(graph: &'g G) -> Self {
        GreedyModularityAlgorithm {
            graph,
            java_hash: None,
        }
    }

    /// Visits the neighbours of a merge in the order JGraphT's `HashSet` does, given the vertices'
    /// Java `hashCode` (for `Integer` vertices: the value). Without it neighbours are visited in
    /// graph order, which differs from Java once vertex values exceed the hash table size; ties
    /// between equal modularity gains are then resolved differently.
    pub fn with_java_hash(mut self, hash: impl Fn(&G::Vertex) -> i32 + 'g) -> Self {
        self.java_hash = Some(Box::new(hash));
        self
    }
}

impl<G: IncidenceGraph> ClusteringAlgorithm<G::Vertex> for GreedyModularityAlgorithm<'_, G> {
    /// Computes a clustering.
    fn clustering(&self) -> Result<Clustering<G::Vertex>> {
        let g = self.graph;
        let vs: Vec<&G::Vertex> = g.vertices().collect();
        let index: HashMap<&G::Vertex, usize> =
            vs.iter().enumerate().map(|(i, v)| (*v, i)).collect();
        let mut communities: BTreeMap<usize, Vec<usize>> =
            (0..vs.len()).map(|i| (i, vec![i])).collect();
        let m = g.edge_count() as f64;

        // dQ[i][j]: change of modularity when merging communities i and j
        let mut dq: BTreeMap<usize, BTreeMap<usize, f64>> = BTreeMap::new();
        for e in g.edges() {
            let (vi, vj) = (
                index[g.edge_source(e).unwrap()],
                index[g.edge_target(e).unwrap()],
            );
            if vi == vj {
                continue;
            }
            let ki = g.degree_of(vs[vi]) as i64;
            let kj = g.degree_of(vs[vj]) as i64;
            let value = (1.0 / m) - ((ki * kj) as f64 / (2.0 * m * m));
            dq.entry(vi).or_default().insert(vj, value);
            dq.entry(vj).or_default().insert(vi, value);
        }

        let mut dq_heap: HashMap<usize, MaxHeap> = HashMap::new();
        let mut dq_handles: HashMap<usize, HashMap<usize, Handle>> = HashMap::new();
        let mut max_heap: MaxHeap = PairingHeap::new();
        let mut max_handles: HashMap<usize, Handle> = HashMap::new();
        for (&vi, columns) in &dq {
            let mut heap: MaxHeap = PairingHeap::new();
            let mut handles = HashMap::new();
            for (&vj, &value) in columns {
                handles.insert(vj, heap.insert(Reverse(OrderedF64(value)), (vi, vj)));
            }
            let (_, key, val) = heap.peek().unwrap();
            let (key, val) = (*key, *val);
            max_handles.insert(vi, max_heap.insert(key, val));
            dq_heap.insert(vi, heap);
            dq_handles.insert(vi, handles);
        }

        let mut a: Vec<f64> = vec![0.0; vs.len()];
        let mut b: Vec<f64>;
        if g.graph_type().directed {
            b = vec![0.0; vs.len()];
            for i in 0..vs.len() {
                a[i] = g.out_degree_of(vs[i]) as f64 / m;
                b[i] = g.in_degree_of(vs[i]) as f64 / m;
            }
        } else {
            for i in 0..vs.len() {
                a[i] = g.degree_of(vs[i]) as f64 / (2.0 * m);
            }
            b = a.clone();
        }

        while dq.len() != 1 {
            let Some((_, max_key, max_val)) = max_heap.peek() else {
                break;
            };
            if max_key.0.0 <= 0.0 {
                break;
            }
            let (i, j) = *max_val;
            let nbrs_i: BTreeSet<usize> = dq[&i].keys().copied().collect();
            let nbrs_j: BTreeSet<usize> = dq[&j].keys().copied().collect();
            let both: BTreeSet<usize> = nbrs_i.intersection(&nbrs_j).copied().collect();
            let all_nbrs: Vec<usize> = {
                let first: Vec<usize> = nbrs_i.iter().copied().collect();
                let rest: Vec<usize> = nbrs_j
                    .iter()
                    .copied()
                    .filter(|x| !nbrs_i.contains(x))
                    .collect();
                let ordered = match &self.java_hash {
                    Some(hash) => java_hash_set_order(first, rest, |&x| hash(vs[x])),
                    None => {
                        let mut all: Vec<usize> = first.into_iter().chain(rest).collect();
                        all.sort_unstable();
                        all
                    }
                };
                ordered.into_iter().filter(|&x| x != i && x != j).collect()
            };
            for &k in &all_nbrs {
                let new_dq_jk = if both.contains(&k) {
                    dq[&i][&k] + dq[&j][&k]
                } else if nbrs_i.contains(&k) {
                    dq[&i][&k] - (a[j] * b[k] + a[k] * b[j])
                } else {
                    dq[&j][&k] - (a[i] * b[k] + a[k] * b[i])
                };
                dq.get_mut(&j).unwrap().insert(k, new_dq_jk);
                dq.get_mut(&k).unwrap().insert(j, new_dq_jk);
                let new_key = Reverse(OrderedF64(new_dq_jk));
                let j_handle = dq_handles[&k].get(&j).copied();
                match j_handle {
                    None => {
                        let heap = dq_heap.get_mut(&k).unwrap();
                        let new_handle = heap.insert(new_key, (k, j));
                        dq_handles.get_mut(&k).unwrap().insert(j, new_handle);
                        if heap.peek().map(|(h, _, _)| h) == Some(new_handle) {
                            max_heap.delete(max_handles[&k]).ok();
                            max_handles.insert(k, max_heap.insert(new_key, (k, j)));
                        }
                    }
                    Some(jh) => {
                        let heap = dq_heap.get_mut(&k).unwrap();
                        let is_j_max = heap.peek().map(|(h, _, _)| h) == Some(jh);
                        heap.delete(jh).ok();
                        let new_handle = heap.insert(new_key, (k, j));
                        dq_handles.get_mut(&k).unwrap().insert(j, new_handle);
                        if heap.peek().map(|(h, _, _)| h) == Some(new_handle) {
                            max_heap.delete(max_handles[&k]).ok();
                            max_handles.insert(k, max_heap.insert(new_key, (k, j)));
                        } else if is_j_max {
                            max_heap.delete(max_handles[&k]).ok();
                            let (_, key, val) = heap.peek().unwrap();
                            let (key, val) = (*key, *val);
                            max_handles.insert(k, max_heap.insert(key, val));
                        }
                    }
                }
                if let Some(ih) = dq_handles[&k].get(&i).copied() {
                    let previous_max_k = max_handles[&k];
                    let heap = dq_heap.get_mut(&k).unwrap();
                    let i_value = *heap.value(ih).unwrap();
                    let is_i_max = *max_heap.value(previous_max_k).unwrap() == i_value;
                    heap.delete(ih).ok();
                    dq_handles.get_mut(&k).unwrap().remove(&i);
                    if is_i_max {
                        max_heap.delete(previous_max_k).ok();
                        let (_, key, val) = heap.peek().unwrap();
                        let (key, val) = (*key, *val);
                        max_handles.insert(k, max_heap.insert(key, val));
                    }
                }
            }
            for columns in dq.values_mut() {
                columns.remove(&i);
            }
            dq.remove(&i);
            dq_heap.remove(&i);
            dq_handles.remove(&i);
            if let Some(h) = max_handles.remove(&i) {
                max_heap.delete(h).ok();
            }
            let mut new_j_heap: MaxHeap = PairingHeap::new();
            let mut new_j_handles = HashMap::new();
            for (&col, &value) in &dq[&j] {
                new_j_handles.insert(col, new_j_heap.insert(Reverse(OrderedF64(value)), (j, col)));
            }
            let first = new_j_heap.peek().map(|(_, key, val)| (*key, *val));
            dq_heap.insert(j, new_j_heap);
            dq_handles.insert(j, new_j_handles);
            if let Some(h) = max_handles.remove(&j) {
                max_heap.delete(h).ok();
            }
            if let Some((key, val)) = first {
                max_handles.insert(j, max_heap.insert(key, val));
            }
            let moved = communities.remove(&i).unwrap();
            communities.get_mut(&j).unwrap().extend(moved);
            a[j] += a[i];
            a[i] = 0.0;
            if g.graph_type().directed {
                b[j] += b[i];
                b[i] = 0.0;
            } else {
                // undirected: JGraphT uses one map for both `a` and `b`
                b[j] = a[j];
                b[i] = a[i];
            }
        }
        Ok(Clustering::new(
            communities
                .into_values()
                .map(|c| c.into_iter().map(|i| vs[i].clone()).collect())
                .collect(),
        ))
    }
}

/// A label propagation clustering algorithm.
///
/// The algorithm is a near linear time algorithm capable of discovering communities in large graphs.
/// It is described in detail in the following
/// [paper](http://dx.doi.org/10.1103/PhysRevE.76.036106):
///
/// - Raghavan, U. N., Albert, R., and Kumara, S. (2007). Near linear time algorithm to detect
/// community structures in large-scale networks. Physical review E, 76(3), 036106.
///
/// As the paper title suggests the running time is close to linear. The algorithm runs in
/// iterations, each of which runs in $O(n + m)$ where $n$ is the number of vertices and $m$ is the
/// number of edges. The authors found experimentally that in most cases, 95% of the nodes or more
/// are classified correctly by the end of iteration 5. See the paper for more details.
///
/// The algorithm is randomized, meaning that two runs on the same graph may return different
/// results. If the user requires deterministic behavior, the random number generator can be provided
/// by the constructor.
///
/// # Rust notes
///
/// Asynchronous label propagation clustering as in JGraphT (`LabelPropagationClustering`): vertices
/// in random order adopt the most frequent neighbour label, ties broken at random. The random
/// draws reproduce JGraphT's for a [`JavaRandom`](crate::util::java::JavaRandom) with the same seed.
pub struct LabelPropagationClustering<'g, G: Graph, R> {
    graph: &'g G,
    max_iterations: usize,
    rng: std::cell::RefCell<R>,
}

impl<'g, G: Graph, R: RandomSource> LabelPropagationClustering<'g, G, R> {
    /// `max_iterations == 0` means no limit.
    pub fn new(graph: &'g G, max_iterations: usize, rng: R) -> Result<Self> {
        props::require_undirected(graph)?;
        Ok(LabelPropagationClustering {
            graph,
            max_iterations,
            rng: std::cell::RefCell::new(rng),
        })
    }
}

impl<G: IncidenceGraph, R: RandomSource> ClusteringAlgorithm<G::Vertex>
    for LabelPropagationClustering<'_, G, R>
{
    fn clustering(&self) -> Result<Clustering<G::Vertex>> {
        let g = self.graph;
        let mut rng = self.rng.borrow_mut();
        let vs: Vec<&G::Vertex> = g.vertices().collect();
        let index: HashMap<&G::Vertex, usize> =
            vs.iter().enumerate().map(|(i, v)| (*v, i)).collect();
        let nbrs: Vec<Vec<usize>> = vs
            .iter()
            .map(|v| {
                g.edges_of(v)
                    .map(|e| index[g.opposite_vertex(e, v).unwrap()])
                    .collect()
            })
            .collect();
        let mut labels: Vec<usize> = (0..vs.len()).collect();

        // (label -> count) in insertion order, plus the largest count among *other* labels
        let counts_and_max = |v: usize, labels: &[usize]| -> (Vec<(usize, usize)>, usize) {
            let mut counts: Vec<(usize, usize)> = Vec::new();
            let v_label = labels[v];
            let mut max_count = 0;
            for &u in &nbrs[v] {
                let u_label = labels[u];
                let new_count = match counts.iter_mut().find(|(l, _)| *l == u_label) {
                    Some((_, c)) => {
                        *c += 1;
                        *c
                    }
                    None => {
                        counts.push((u_label, 1));
                        1
                    }
                };
                if new_count > max_count && u_label != v_label {
                    max_count = new_count;
                }
            }
            (counts, max_count)
        };
        let should_stop = |labels: &[usize]| -> bool {
            (0..vs.len()).all(|v| {
                let (counts, max_count) = counts_and_max(v, labels);
                let v_count = counts
                    .iter()
                    .find(|(l, _)| *l == labels[v])
                    .map_or(0, |c| c.1);
                max_count <= v_count
            })
        };

        let mut iteration = 0;
        loop {
            if self.max_iterations > 0 && iteration > self.max_iterations {
                break;
            }
            let mut any_change = false;
            let mut all: Vec<usize> = (0..vs.len()).collect();
            java_shuffle(&mut all, &mut *rng);
            for &v in &all {
                if g.degree_of(vs[v]) == 0 {
                    continue;
                }
                let (counts, other_max) = counts_and_max(v, &labels);
                let old = labels[v];
                let v_count = counts.iter().find(|(l, _)| *l == old).map_or(0, |c| c.1);
                let max_count = other_max.max(v_count);
                // Java iterates a HashMap<String, Integer> keyed by the label's decimal string
                let candidates: Vec<usize> = counts
                    .iter()
                    .filter(|(_, c)| *c == max_count)
                    .map(|(l, _)| *l)
                    .collect();
                let ordered_all = java_hash_order(counts.iter().map(|(l, _)| *l).collect(), |l| {
                    java_string_hash(&l.to_string())
                });
                let max_labels: Vec<usize> = ordered_all
                    .into_iter()
                    .filter(|l| candidates.contains(l))
                    .collect();
                let new_label = max_labels[rng.next_int_bound(max_labels.len())];
                if new_label != old {
                    labels[v] = new_label;
                    any_change = true;
                }
            }
            if !any_change || should_stop(&labels) {
                break;
            }
            iteration += 1;
        }

        // connected groups of equal labels, numbered in vertex order
        let mut final_labels: Vec<Option<usize>> = vec![None; vs.len()];
        let mut next_label = 0;
        for v in 0..vs.len() {
            if final_labels[v].is_some() {
                continue;
            }
            let current = next_label;
            next_label += 1;
            final_labels[v] = Some(current);
            let mut frontier = std::collections::VecDeque::from([v]);
            while let Some(u) = frontier.pop_front() {
                for &w in &nbrs[u] {
                    if labels[w] != labels[u] || final_labels[w].is_some() {
                        continue;
                    }
                    final_labels[w] = Some(current);
                    frontier.push_back(w);
                }
            }
        }
        let mut cluster_map: IndexMap<usize, IndexSet<G::Vertex>> = IndexMap::new();
        for v in 0..vs.len() {
            cluster_map
                .entry(final_labels[v].unwrap())
                .or_default()
                .insert(vs[v].clone());
        }
        Ok(Clustering::new(cluster_map.into_values().collect()))
    }
}
