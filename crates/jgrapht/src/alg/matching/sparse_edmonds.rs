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

use super::base::{Matching, MatchingAlgorithm};
use super::greedy::GreedyMaximumCardinalityMatching;
use crate::alg::util::FixedSizeIntegerQueue;
use crate::error::Result;
use crate::graph::{Graph, GraphExt, props};
use indexmap::IndexSet;
use std::collections::{HashMap, HashSet, VecDeque};

const NULL: i64 = -1;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Label {
    Even,
    Odd,
    Unlabeled,
}

/// The partition of the vertices into blossoms (`VertexPartition`): union by rank with path
/// compression, where every set has a name that can be chosen.
struct VertexPartition {
    rep: Vec<usize>,
    rank: Vec<u32>,
    parent: Vec<usize>,
}

impl VertexPartition {
    fn new(n: usize) -> Self {
        VertexPartition {
            rep: (0..n).collect(),
            rank: vec![0; n],
            parent: (0..n).collect(),
        }
    }

    fn find_item(&mut self, e: usize) -> usize {
        let mut current = e;
        while self.parent[current] != current {
            current = self.parent[current];
        }
        let root = current;
        let mut current = e;
        while current != root {
            let parent = self.parent[current];
            self.parent[current] = root;
            current = parent;
        }
        root
    }

    fn find(&mut self, e: usize) -> usize {
        let item = self.find_item(e);
        self.rep[item]
    }

    fn union(&mut self, a: usize, b: usize) {
        let ia = self.find_item(a);
        let ib = self.find_item(b);
        if ia == ib {
            return;
        }
        if self.rank[ia] > self.rank[ib] {
            self.parent[ib] = ia;
        } else if self.rank[ia] < self.rank[ib] {
            self.parent[ia] = ib;
        } else {
            self.parent[ib] = ia;
            self.rank[ia] += 1;
        }
    }

    fn name(&mut self, e: usize) {
        let ie = self.find_item(e);
        self.rep[ie] = e;
    }

    fn split(&mut self, to_split: &[usize]) {
        for &i in to_split {
            self.parent[i] = i;
            self.rep[i] = i;
            self.rank[i] = 0;
        }
    }
}

/// The matching state of one run (`SparseEdmondsMaximumCardinalityMatching.Algorithm`).
struct Algorithm<'a, G: Graph> {
    graph: &'a G,
    nodes: usize,
    vertex_index_map: HashMap<G::Vertex, usize>,
    vertex_map: Vec<G::Vertex>,
    /// Neighbours of every vertex as positions, in the order of `edgesOf`.
    neighbors: Vec<Vec<usize>>,
    mate: Vec<i64>,
    label: Vec<Label>,
    pred: Vec<i64>,
    strue: f64,
    path1: Vec<f64>,
    path2: Vec<f64>,
    source_bridge: Vec<i64>,
    target_bridge: Vec<i64>,
    base: VertexPartition,
    queue: FixedSizeIntegerQueue,
    labeled_nodes: Vec<usize>,
}

impl<'a, G: Graph> Algorithm<'a, G> {
    fn new(graph: &'a G) -> Self {
        let nodes = graph.vertex_count();
        let vertex_map: Vec<G::Vertex> = graph.vertices().cloned().collect();
        let vertex_index_map: HashMap<G::Vertex, usize> = vertex_map
            .iter()
            .enumerate()
            .map(|(i, v)| (v.clone(), i))
            .collect();
        let neighbors = vertex_map
            .iter()
            .map(|v| {
                graph
                    .edges_of(v)
                    .unwrap()
                    .map(|e| vertex_index_map[graph.opposite_vertex(e, v).expect("incident edge")])
                    .collect()
            })
            .collect();
        Algorithm {
            graph,
            nodes,
            vertex_index_map,
            vertex_map,
            neighbors,
            mate: vec![NULL; nodes],
            label: vec![Label::Even; nodes],
            pred: vec![NULL; nodes],
            strue: 0.0,
            path1: vec![0.0; nodes],
            path2: vec![0.0; nodes],
            source_bridge: vec![NULL; nodes],
            target_bridge: vec![NULL; nodes],
            base: VertexPartition::new(nodes),
            queue: FixedSizeIntegerQueue::new(nodes.max(1)),
            labeled_nodes: Vec::new(),
        }
    }

    fn run_initializer(
        &mut self,
        initializer: Option<&mut (dyn MatchingAlgorithm<G::Vertex, G::Edge> + '_)>,
    ) -> Result<()> {
        let Some(initializer) = initializer else {
            return Ok(());
        };
        for e in initializer.get_matching()?.edges() {
            let (u, v) = self.graph.edge_endpoints(e).expect("edge of the graph");
            let (u_index, v_index) = (self.vertex_index_map[u], self.vertex_index_map[v]);
            self.mate[u_index] = v_index as i64;
            self.label[u_index] = Label::Unlabeled;
            self.mate[v_index] = u_index as i64;
            self.label[v_index] = Label::Unlabeled;
        }
        Ok(())
    }

    fn find_path(&self, p: &mut VecDeque<usize>, x: usize, y: usize) {
        if x == y {
            p.push_back(x);
            return;
        }
        if self.label[x] == Label::Even {
            p.push_back(x);
            let mx = self.mate[x] as usize;
            p.push_back(mx);
            self.find_path(p, self.pred[mx] as usize, y);
            return;
        }
        p.push_back(x);
        let mut p2 = VecDeque::new();
        self.find_path(
            &mut p2,
            self.source_bridge[x] as usize,
            self.mate[x] as usize,
        );
        while let Some(last) = p2.pop_back() {
            p.push_back(last);
        }
        self.find_path(p, self.target_bridge[x] as usize, y);
    }

    fn shrink_path(&mut self, b: usize, v: usize, w: usize) {
        let mut x = self.base.find(v);
        while x != b {
            self.base.union(x, b);
            x = self.mate[x] as usize;
            self.base.union(x, b);
            self.base.name(b); // make sure b is called the same
            self.queue.enqueue(x);
            self.source_bridge[x] = v as i64;
            self.target_bridge[x] = w as i64;
            x = self.base.find(self.pred[x] as usize);
        }
    }

    fn compute_matching(
        &mut self,
        initializer: Option<&mut (dyn MatchingAlgorithm<G::Vertex, G::Edge> + '_)>,
    ) -> Result<IndexSet<G::Edge>> {
        self.run_initializer(initializer)?;

        for i in 0..self.nodes {
            if self.mate[i] != NULL {
                continue;
            }
            self.queue.clear();
            self.queue.enqueue(i);
            self.labeled_nodes.clear();
            self.labeled_nodes.push(i);
            let mut break_through = false;
            while !break_through {
                let Some(v) = self.queue.poll() else { break }; // grow tree
                for k in 0..self.neighbors[v].len() {
                    let w = self.neighbors[v][k];
                    let (bv, bw) = (self.base.find(v), self.base.find(w));
                    if bv == bw || self.label[bw] == Label::Odd {
                        continue;
                    }
                    if self.label[w] == Label::Unlabeled {
                        // grow tree
                        self.label[w] = Label::Odd;
                        self.labeled_nodes.push(w);
                        self.pred[w] = v as i64;
                        let mw = self.mate[w] as usize;
                        self.label[mw] = Label::Even;
                        self.labeled_nodes.push(mw);
                        self.queue.enqueue(mw);
                    } else {
                        // augment or shrink blossom
                        let mut hv = self.base.find(v);
                        let mut hw = self.base.find(w);
                        self.strue += 1.0;
                        self.path1[hv] = self.strue;
                        self.path2[hw] = self.strue;
                        while (self.path1[hw] != self.strue && self.path2[hv] != self.strue)
                            && (self.mate[hv] != NULL || self.mate[hw] != NULL)
                        {
                            if self.mate[hv] != NULL {
                                hv = self.base.find(self.pred[self.mate[hv] as usize] as usize);
                                self.path1[hv] = self.strue;
                            }
                            if self.mate[hw] != NULL {
                                hw = self.base.find(self.pred[self.mate[hw] as usize] as usize);
                                self.path2[hw] = self.strue;
                            }
                        }
                        if self.path1[hw] == self.strue || self.path2[hv] == self.strue {
                            let b = if self.path1[hw] == self.strue { hw } else { hv }; // base
                            self.shrink_path(b, v, w);
                            self.shrink_path(b, w, v);
                        } else {
                            let mut p = VecDeque::new();
                            self.find_path(&mut p, v, hv);
                            p.push_front(w);
                            while let (Some(a), Some(b)) = (p.pop_front(), p.pop_front()) {
                                self.mate[a] = b as i64;
                                self.mate[b] = a as i64;
                            }
                            self.labeled_nodes.push(w);
                            for &k in &self.labeled_nodes {
                                self.label[k] = Label::Unlabeled;
                            }
                            self.base.split(&self.labeled_nodes);
                            break_through = true;
                            break;
                        }
                    }
                }
            }
        }

        let graph = self.graph;
        let mut matching: IndexSet<G::Edge> = IndexSet::new();
        for e in graph.edges() {
            let (u, v) = graph.edge_endpoints(e).expect("edge of the graph");
            if u == v {
                continue;
            }
            let (u_index, v_index) = (self.vertex_index_map[u], self.vertex_index_map[v]);
            if u_index != v_index && self.mate[u_index] == v_index as i64 {
                matching.insert(e.clone());
                self.mate[u_index] = u_index as i64;
                self.mate[v_index] = v_index as i64;
            }
        }
        Ok(matching)
    }

    fn compute_odd_set_cover(&mut self) -> HashMap<G::Vertex, i64> {
        let nodes = self.nodes;
        let mut osc = vec![-1i64; nodes];
        let mut number_of_unlabeled = 0;
        let mut arb_u_node = 0;
        for v in 0..nodes {
            if self.label[v] == Label::Unlabeled {
                number_of_unlabeled += 1;
                arb_u_node = v;
            }
        }
        if number_of_unlabeled > 0 {
            osc[arb_u_node] = 1;
            let lambda = if number_of_unlabeled == 2 { 0 } else { 2 };
            for v in 0..nodes {
                if self.label[v] == Label::Unlabeled && v != arb_u_node {
                    osc[v] = lambda;
                }
            }
        }
        let mut kappa = if number_of_unlabeled <= 2 { 2 } else { 3 };
        for v in 0..nodes {
            let bv = self.base.find(v);
            if bv != v && osc[bv] == -1 {
                osc[bv] = kappa;
                kappa += 1;
            }
        }
        for v in 0..nodes {
            let bv = self.base.find(v);
            if bv == v && osc[v] == -1 {
                if self.label[v] == Label::Even {
                    osc[v] = 0;
                }
                if self.label[v] == Label::Odd {
                    osc[v] = 1;
                }
            }
            if bv != v {
                osc[v] = osc[bv];
            }
        }
        self.vertex_map
            .iter()
            .enumerate()
            .map(|(v, vertex)| (vertex.clone(), osc[v]))
            .collect()
    }
}

/// Edmonds' blossom algorithm for maximum cardinality matching in general undirected graphs.
///
/// A matching in a graph $G(V,E)$ is a subset of edges $M$ such that no two edges in $M$ have a
/// vertex in common. A matching has at most $\frac{1}{2}|V|$ edges. A node $v$ in $G$ is matched by
/// matching $M$ if $M$ contains an edge incident to $v$. A matching is perfect if all nodes are
/// matched. By definition, a perfect matching consists of exactly $\frac{1}{2}|V|$ edges. This
/// algorithm will return a perfect matching if one exists. If no perfect matching exists, then the
/// largest (non-perfect) matching is returned instead. In the special case that the input graph is
/// bipartite, consider using `HopcroftKarpMaximumCardinalityBipartiteMatching` instead.
///
/// To compute a maximum cardinality matching, at most $n$ augmenting path computations are
/// performed. Each augmenting path computation takes $O(m \alpha(m,n))$ time, where $\alpha(m,n)$ is
/// the inverse of the Ackerman function, $n$ is the number of vertices, and $m$ the number of edges.
/// This results in a total runtime complexity of O(n m alpha(m,n)). In practice, the number of
/// augmenting path computations performed is far smaller than $n$, since an efficient heuristic is
/// used to compute a near-optimal initial solution. The heuristic by default is the
/// `GreedyMaximumCardinalityMatching` but can be changed using the appropriate constructor.
///
/// The runtime complexity of this implementation could be improved to $O(n m)$ when the UnionFind
/// data structure used in this implementation is replaced by the linear-time set union data
/// structure proposed in: Gabow, H.N., Tarjan, R.E. A linear-time algorithm for a special case of
/// disjoint set union. Proc. Fifteenth Annual ACM Symposium on Theory of Computing, 1982, pp.
/// 246-251.
///
/// Edmonds' original algorithm first appeared in Edmonds, J. Paths, trees, and flowers. Canadian
/// Journal of Mathematics 17, 1965, pp. 449-467, and had a runtime complexity of $O(n^4)$.
///
/// This is the algorithm and implementation described in the
/// [LEDA book](https://people.mpi-inf.mpg.de/~mehlhorn/LEDAbook.html). See the LEDA
/// Platform of Combinatorial and Geometric Computing, Cambridge University Press, 1999.
///
/// For future reference - A more efficient algorithm exists: S. Micali and V. Vazirani. An
/// $O(\sqrt{n}m)$ algorithm for finding maximum matching in general graphs. Proc. 21st Ann. Symp. on
/// Foundations of Computer Science, IEEE, 1980, pp. 17–27. This is the most efficient algorithm
/// known for computing maximum cardinality matchings in general graphs. More details on this
/// algorithm can be found in:
///
/// - [Presentation from Vazirani 'Dispelling an Old Myth about an Ancient Algorithm'](http://research.microsoft.com/apps/video/dl.aspx?id=171055)
/// - [Vazirani, V. A Simplification of the MV Matching Algorithm and its Proof, 2013](https://arxiv.org/abs/1210.4594)
///
/// # Rust notes
///
/// `SparseEdmondsMaximumCardinalityMatching`: Edmonds' blossom algorithm for a maximum
/// cardinality matching in a general graph, in `O(V E log V)` (Gabow, with an odd set cover that
/// certifies optimality), best for sparse graphs.
pub struct SparseEdmondsMaximumCardinalityMatching<'g, G: Graph> {
    graph: &'g G,
    initializer: Option<Box<dyn MatchingAlgorithm<G::Vertex, G::Edge> + 'g>>,
    result: Option<Matching<G::Vertex, G::Edge>>,
    odd_set_cover: Option<HashMap<G::Vertex, i64>>,
}

impl<'g, G: Graph + 'g> SparseEdmondsMaximumCardinalityMatching<'g, G> {
    /// Uses `GreedyMaximumCardinalityMatching` (unsorted) as the initializer. Fails if the graph is
    /// not undirected.
    pub fn new(graph: &'g G) -> Result<Self> {
        let initializer = GreedyMaximumCardinalityMatching::new(graph, false)?;
        Self::with_initializer(graph, Some(Box::new(initializer)))
    }

    /// Fails if the graph is not undirected. `None` starts from the empty matching.
    pub fn with_initializer(
        graph: &'g G,
        initializer: Option<Box<dyn MatchingAlgorithm<G::Vertex, G::Edge> + 'g>>,
    ) -> Result<Self> {
        props::require_undirected(graph)?;
        Ok(SparseEdmondsMaximumCardinalityMatching {
            graph,
            initializer,
            result: None,
            odd_set_cover: None,
        })
    }

    /// The odd set cover that certifies the optimality of the matching.
    pub fn get_odd_set_cover(&mut self) -> Result<&HashMap<G::Vertex, i64>> {
        self.get_matching()?;
        Ok(self.odd_set_cover.as_ref().expect("computed"))
    }

    /// Checks that `matching` is a matching and that `odd_set_cover` proves it maximum.
    pub fn is_optimal_matching(
        graph: &G,
        matching: &IndexSet<G::Edge>,
        odd_set_cover: &HashMap<G::Vertex, i64>,
    ) -> bool {
        let mut matched: HashSet<&G::Vertex> = HashSet::new();
        for e in matching {
            let (s, t) = graph.edge_endpoints(e).expect("edge of the graph");
            if !matched.insert(s) || !matched.insert(t) {
                return false;
            }
        }
        let n = graph.vertex_count().max(2);
        let mut kappa = 1i64;
        let mut count = vec![0usize; n];
        for v in graph.vertices() {
            let osc = odd_set_cover[v];
            if osc < 0 || osc >= n as i64 {
                return false;
            }
            count[osc as usize] += 1;
            if osc > kappa {
                kappa = osc;
            }
        }
        let mut s = count[1];
        for c in count.iter().take(kappa as usize + 1).skip(2) {
            s += c / 2;
        }
        if s != matching.len() {
            return false;
        }
        for e in graph.edges() {
            let (v, w) = graph.edge_endpoints(e).expect("edge of the graph");
            let (oscv, oscw) = (odd_set_cover[v], odd_set_cover[w]);
            if v == w || oscv == 1 || oscw == 1 || (oscv == oscw && oscv >= 2) {
                continue;
            }
            return false;
        }
        true
    }
}

impl<'g, G: Graph + 'g> MatchingAlgorithm<G::Vertex, G::Edge>
    for SparseEdmondsMaximumCardinalityMatching<'g, G>
{
    fn get_matching(&mut self) -> Result<Matching<G::Vertex, G::Edge>> {
        if self.result.is_none() {
            let mut alg = Algorithm::new(self.graph);
            let matching_edges = alg.compute_matching(self.initializer.as_deref_mut())?;
            let cardinality = matching_edges.len();
            self.result = Some(Matching::new(
                self.graph,
                matching_edges,
                cardinality as f64,
            ));
            self.odd_set_cover = Some(alg.compute_odd_set_cover());
        }
        Ok(self.result.clone().expect("computed"))
    }
}
