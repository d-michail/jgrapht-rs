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

//! Lowest common ancestors in rooted trees/forests (binary lifting).

use crate::graph::IncidenceGraph;
use crate::graph::OutgoingGraph;
use crate::graph::{Graph, GraphExt};
use std::collections::{HashMap, VecDeque};
use std::hash::Hash;

/// LCA structure over the tree reachable from `root` (edges followed in their direction;
/// undirected graphs are treated as trees rooted at `root`).
pub struct Lca<V> {
    index: HashMap<V, usize>,
    vertices: Vec<V>,
    depth: Vec<usize>,
    up: Vec<Vec<usize>>,
}

impl<V: Clone + Eq + Hash> Lca<V> {
    pub fn new<G: Graph<Vertex = V> + OutgoingGraph>(g: &G, root: &V) -> Option<Self> {
        let root = g.vertices().find(|v| *v == root)?;
        let mut index = HashMap::from([(root.clone(), 0)]);
        let mut vertices = vec![root.clone()];
        let (mut depth, mut parent) = (vec![0], vec![0]);
        let mut q = VecDeque::from([root]);
        while let Some(v) = q.pop_front() {
            let i = index[v];
            for n in g.out_neighbors_of(v) {
                if !index.contains_key(n) {
                    index.insert(n.clone(), vertices.len());
                    vertices.push(n.clone());
                    depth.push(depth[i] + 1);
                    parent.push(i);
                    q.push_back(n);
                }
            }
        }
        let levels = (usize::BITS - vertices.len().leading_zeros()) as usize;
        let mut up = vec![parent];
        for k in 1..levels.max(1) {
            let prev = &up[k - 1];
            up.push((0..vertices.len()).map(|v| prev[prev[v]]).collect());
        }
        Some(Lca {
            index,
            vertices,
            depth,
            up,
        })
    }

    /// Lowest common ancestor; `None` if either vertex is not in the tree.
    pub fn lca(&self, a: &V, b: &V) -> Option<&V> {
        let (mut a, mut b) = (*self.index.get(a)?, *self.index.get(b)?);
        if self.depth[a] < self.depth[b] {
            std::mem::swap(&mut a, &mut b);
        }
        let diff = self.depth[a] - self.depth[b];
        for (k, level) in self.up.iter().enumerate() {
            if diff >> k & 1 == 1 {
                a = level[a];
            }
        }
        if a != b {
            for level in self.up.iter().rev() {
                if level[a] != level[b] {
                    a = level[a];
                    b = level[b];
                }
            }
            a = self.up[0][a];
        }
        Some(&self.vertices[a])
    }

    pub fn depth(&self, v: &V) -> Option<usize> {
        self.index.get(v).map(|&i| self.depth[i])
    }
}

// ---------------------------------------------------------------------------------------------
// JGraphT's LCA finders

use super::decomposition::HeavyPathDecomposition;
use super::interfaces::LowestCommonAncestorAlgorithm;
use super::util::UnionFind;
use crate::graph::DirectedGraph;
use crate::view::EdgeReversed;
use crate::{GraphError, Result};
use std::collections::HashSet;

fn invalid<T>(msg: impl Into<String>) -> Result<T> {
    Err(GraphError::InvalidArgument(msg.into()))
}

/// Rooted forest explored from `roots` along outgoing edges (`ancestors[0]` parents, DFS times,
/// Euler tour).
struct Forest<V> {
    index: HashMap<V, usize>,
    vertices: Vec<V>,
    parent: Vec<Option<usize>>,
    /// 0 for vertices not reached from any root, otherwise 1, 2, ...
    component: Vec<usize>,
    time_in: Vec<i64>,
    time_out: Vec<i64>,
    /// `(vertex, depth)` along the Euler tour of every tree
    tour: Vec<(usize, usize)>,
}

impl<V: Clone + Eq + Hash> Forest<V> {
    fn new<G: Graph<Vertex = V> + OutgoingGraph>(g: &G, roots: &[V]) -> Result<Self> {
        if roots.is_empty() {
            return Err(GraphError::InvalidParameter { name: "roots", requirement: crate::ParameterRequirement::NonEmpty });
        }
        let vertices: Vec<V> = g.vertices().cloned().collect();
        let index: HashMap<V, usize> = vertices
            .iter()
            .cloned()
            .enumerate()
            .map(|(i, v)| (v, i))
            .collect();
        if roots.iter().any(|r| !index.contains_key(r)) {
            return Err(GraphError::VertexNotFound);
        }
        let n = vertices.len();
        let mut f = Forest {
            index,
            vertices,
            parent: vec![None; n],
            component: vec![0; n],
            time_in: (0..n).map(|i| -(i as i64 + 1)).collect(),
            time_out: (0..n).map(|i| -(i as i64 + 1)).collect(),
            tour: Vec::new(),
        };
        let mut clock = 0i64;
        let mut number_component = 0;
        for root in roots {
            let r = f.index[root];
            if f.component[r] != 0 {
                return invalid("multiple roots in the same tree");
            }
            number_component += 1;
            f.dfs(g, r, number_component, &mut clock);
        }
        Ok(f)
    }

    fn dfs<G: Graph<Vertex = V> + OutgoingGraph>(&mut self, g: &G, root: usize, comp: usize, clock: &mut i64) {
        let children_of = |f: &Self, u: usize| -> Vec<usize> {
            let vu = &f.vertices[u];
            g.out_edges_of(vu)
                .filter_map(|e| g.opposite_vertex(e, vu))
                .map(|w| f.index[w])
                .collect()
        };
        self.component[root] = comp;
        *clock += 1;
        self.time_in[root] = *clock;
        let mut depth_of = vec![(root, 0usize)];
        self.tour.push((root, 0));
        let mut stack: Vec<(usize, Vec<usize>, usize)> = vec![(root, children_of(self, root), 0)];
        while let Some((u, children, next)) = stack.last_mut() {
            let u = *u;
            if *next < children.len() {
                let v = children[*next];
                *next += 1;
                if self.component[v] == 0 {
                    self.component[v] = comp;
                    self.parent[v] = Some(u);
                    *clock += 1;
                    self.time_in[v] = *clock;
                    let depth = stack.len();
                    self.tour.push((v, depth));
                    let ch = children_of(self, v);
                    stack.push((v, ch, 0));
                }
            } else {
                *clock += 1;
                self.time_out[u] = *clock;
                stack.pop();
                if let Some(&(p, _, _)) = stack.last() {
                    self.tour.push((p, stack.len() - 1));
                }
            }
        }
        depth_of.clear();
    }

    fn check(&self, v: &V) -> Result<usize> {
        self.index
            .get(v)
            .copied()
            .map_or_else(|| Err(GraphError::VertexNotFound), Ok)
    }

    fn is_ancestor(&self, ancestor: usize, descendant: usize) -> bool {
        self.time_in[ancestor] <= self.time_in[descendant]
            && self.time_out[descendant] <= self.time_out[ancestor]
    }
}

/// LCA by binary lifting, `O(log n)` per query (`BinaryLiftingLCAFinder`).
pub struct BinaryLiftingLcaFinder<V> {
    forest: Forest<V>,
    /// `ancestors[l][v]`: the 2^l-th ancestor, `usize::MAX` if none
    ancestors: Vec<Vec<usize>>,
}

impl<V: Clone + Eq + Hash> BinaryLiftingLcaFinder<V> {
    pub fn new<G: Graph<Vertex = V> + OutgoingGraph>(g: &G, root: V) -> Result<Self> {
        Self::with_roots(g, &[root])
    }

    pub fn with_roots<G: Graph<Vertex = V> + OutgoingGraph>(g: &G, roots: &[V]) -> Result<Self> {
        let forest = Forest::new(g, roots)?;
        let n = forest.vertices.len();
        let levels = ((usize::BITS - n.max(1).leading_zeros()) as usize).max(1);
        let none = usize::MAX;
        let mut ancestors = vec![
            forest
                .parent
                .iter()
                .map(|p| p.unwrap_or(none))
                .collect::<Vec<_>>(),
        ];
        for l in 1..levels {
            let prev = &ancestors[l - 1];
            let next = (0..n)
                .map(|v| if prev[v] == none { none } else { prev[prev[v]] })
                .collect();
            ancestors.push(next);
        }
        Ok(BinaryLiftingLcaFinder { forest, ancestors })
    }
}

impl<V: Clone + Eq + Hash> LowestCommonAncestorAlgorithm<V> for BinaryLiftingLcaFinder<V> {
    fn lca(&self, a: &V, b: &V) -> Result<Option<V>> {
        let f = &self.forest;
        let (mut ia, ib) = (f.check(a)?, f.check(b)?);
        if a == b {
            return Ok(Some(a.clone()));
        }
        if f.component[ia] != f.component[ib] || f.component[ia] == 0 {
            return Ok(None);
        }
        if f.is_ancestor(ia, ib) {
            return Ok(Some(a.clone()));
        }
        if f.is_ancestor(ib, ia) {
            return Ok(Some(b.clone()));
        }
        for level in self.ancestors.iter().rev() {
            let up = level[ia];
            if up != usize::MAX && !f.is_ancestor(up, ib) {
                ia = up;
            }
        }
        Ok(f.parent[ia].map(|p| f.vertices[p].clone()))
    }
}

/// LCA by Euler tour and a sparse-table range-minimum query, `O(1)` per query
/// (`EulerTourRMQLCAFinder`).
pub struct EulerTourRmqLcaFinder<V> {
    forest: Forest<V>,
    representative: Vec<usize>,
    /// `rmq[k][i]`: tour position of the minimum depth in `[i, i + 2^k)`
    rmq: Vec<Vec<usize>>,
}

impl<V: Clone + Eq + Hash> EulerTourRmqLcaFinder<V> {
    pub fn new<G: Graph<Vertex = V> + OutgoingGraph>(g: &G, root: V) -> Result<Self> {
        Self::with_roots(g, &[root])
    }

    pub fn with_roots<G: Graph<Vertex = V> + OutgoingGraph>(g: &G, roots: &[V]) -> Result<Self> {
        let forest = Forest::new(g, roots)?;
        let size = forest.tour.len();
        let mut representative = vec![usize::MAX; forest.vertices.len()];
        for (i, &(v, _)) in forest.tour.iter().enumerate() {
            if representative[v] == usize::MAX {
                representative[v] = i;
            }
        }
        let level = |i: usize| forest.tour[i].1;
        let mut rmq = vec![(0..size).collect::<Vec<_>>()];
        let mut k = 1;
        while (1usize << k) <= size {
            let half = 1 << (k - 1);
            let prev = &rmq[k - 1];
            let row = (0..=size - (1 << k))
                .map(|j| {
                    if level(prev[j]) < level(prev[j + half]) {
                        prev[j]
                    } else {
                        prev[j + half]
                    }
                })
                .collect();
            rmq.push(row);
            k += 1;
        }
        Ok(EulerTourRmqLcaFinder {
            forest,
            representative,
            rmq,
        })
    }
}

impl<V: Clone + Eq + Hash> LowestCommonAncestorAlgorithm<V> for EulerTourRmqLcaFinder<V> {
    fn lca(&self, a: &V, b: &V) -> Result<Option<V>> {
        let f = &self.forest;
        let (ia, ib) = (f.check(a)?, f.check(b)?);
        if a == b {
            return Ok(Some(a.clone()));
        }
        if f.component[ia] != f.component[ib] || f.component[ia] == 0 {
            return Ok(None);
        }
        let (mut x, mut y) = (self.representative[ia], self.representative[ib]);
        if x > y {
            std::mem::swap(&mut x, &mut y);
        }
        let l = (y - x + 1).ilog2() as usize;
        let pw = 1usize << l;
        let (c1, c2) = (self.rmq[l][x], self.rmq[l][y + 1 - pw]);
        let sol = if f.tour[c1].1 > f.tour[c2].1 { c2 } else { c1 };
        Ok(Some(f.vertices[f.tour[sol].0].clone()))
    }
}

/// LCA with a heavy-path decomposition, `O(log n)` per query (`HeavyPathLCAFinder`).
pub struct HeavyPathLcaFinder<V: Eq + Hash + Clone> {
    vertex_map: HashMap<V, usize>,
    index_list: Vec<V>,
    parent: Vec<Option<usize>>,
    depth: Vec<Option<usize>>,
    component: Vec<Option<usize>>,
    first_node_in_path: Vec<usize>,
    path: Vec<Option<usize>>,
    position_in_path: Vec<Option<usize>>,
}

impl<V: Clone + Eq + Hash> HeavyPathLcaFinder<V> {
    pub fn new<G: Graph<Vertex = V> + IncidenceGraph>(g: &G, root: V) -> Result<Self> {
        Self::with_roots(g, [root])
    }

    pub fn with_roots<G: Graph<Vertex = V> + IncidenceGraph>(
        g: &G,
        roots: impl IntoIterator<Item = V>,
    ) -> Result<Self> {
        let roots: Vec<V> = roots.into_iter().collect();
        if roots.is_empty() {
            return Err(GraphError::InvalidParameter { name: "roots", requirement: crate::ParameterRequirement::NonEmpty });
        }
        let d = HeavyPathDecomposition::with_roots(g, roots)?;
        Ok(HeavyPathLcaFinder {
            vertex_map: d.vertex_map().clone(),
            index_list: d.index_list().to_vec(),
            parent: d.parent_array().to_vec(),
            depth: d.depth_array().to_vec(),
            component: d.component_array().to_vec(),
            first_node_in_path: d.first_node_in_path_array().to_vec(),
            path: d.path_array().to_vec(),
            position_in_path: d.position_in_path_array().to_vec(),
        })
    }
}

impl<V: Clone + Eq + Hash> LowestCommonAncestorAlgorithm<V> for HeavyPathLcaFinder<V> {
    fn lca(&self, a: &V, b: &V) -> Result<Option<V>> {
        let Some(&(mut ia)) = self.vertex_map.get(a) else {
            return Err(GraphError::VertexNotFound);
        };
        let Some(&(mut ib)) = self.vertex_map.get(b) else {
            return Err(GraphError::VertexNotFound);
        };
        if a == b {
            return Ok(Some(a.clone()));
        }
        let (ca, cb) = (self.component[ia], self.component[ib]);
        if ca != cb || ca.is_none() {
            return Ok(None);
        }
        let (mut pa, mut pb) = (self.path[ia].unwrap(), self.path[ib].unwrap());
        while pa != pb {
            let (fa, fb) = (self.first_node_in_path[pa], self.first_node_in_path[pb]);
            if self.depth[fa] < self.depth[fb] {
                ib = self.parent[fb].unwrap();
                pb = self.path[ib].unwrap();
            } else {
                ia = self.parent[fa].unwrap();
                pa = self.path[ia].unwrap();
            }
        }
        let pick = if self.position_in_path[ia] < self.position_in_path[ib] {
            ia
        } else {
            ib
        };
        Ok(Some(self.index_list[pick].clone()))
    }
}

/// Tarjan's offline LCA (`TarjanLCAFinder`): one DFS answers a whole batch of queries.
pub struct TarjanLcaFinder<'g, G: Graph> {
    graph: &'g G,
    roots: Vec<G::Vertex>,
}

impl<'g, G: OutgoingGraph> TarjanLcaFinder<'g, G> {
    pub fn new(graph: &'g G, root: G::Vertex) -> Result<Self> {
        Self::with_roots(graph, vec![root])
    }

    pub fn with_roots(graph: &'g G, roots: Vec<G::Vertex>) -> Result<Self> {
        if roots.is_empty() {
            return Err(GraphError::InvalidParameter { name: "roots", requirement: crate::ParameterRequirement::NonEmpty });
        }
        if roots.iter().any(|r| !graph.contains_vertex(r)) {
            return Err(GraphError::VertexNotFound);
        }
        Ok(TarjanLcaFinder { graph, roots })
    }

    fn compute(&self, queries: &[(G::Vertex, G::Vertex)]) -> Result<Vec<Option<G::Vertex>>> {
        let g = self.graph;
        let mut result: Vec<Option<G::Vertex>> = Vec::with_capacity(queries.len());
        let mut query_occurs: HashMap<&G::Vertex, Vec<usize>> = HashMap::new();
        for (i, (a, b)) in queries.iter().enumerate() {
            if !g.contains_vertex(a) || !g.contains_vertex(b) {
                return Err(GraphError::VertexNotFound);
            }
            if a == b {
                result.push(Some(a.clone()));
            } else {
                query_occurs.entry(a).or_default().push(i);
                query_occurs.entry(b).or_default().push(i);
                result.push(None);
            }
        }
        let mut uf: UnionFind<G::Vertex> = UnionFind::new(std::iter::empty());
        let mut ancestors: HashMap<G::Vertex, G::Vertex> = HashMap::new();
        let mut visited: HashSet<G::Vertex> = HashSet::new();
        for root in &self.roots {
            if visited.contains(root) {
                return invalid("multiple roots in the same tree");
            }
            let mut black: HashSet<G::Vertex> = HashSet::new();
            // iterative DFS: (vertex, parent, neighbours, next)
            visited.insert(root.clone());
            uf.add_element(root.clone())?;
            ancestors.insert(root.clone(), root.clone());
            let nbrs = |u: &G::Vertex| -> Vec<G::Vertex> {
                g.out_edges_of(u)
                    .filter_map(|e| g.opposite_vertex(e, u).cloned())
                    .collect()
            };
            let mut stack: Vec<(G::Vertex, Option<G::Vertex>, Vec<G::Vertex>, usize)> =
                vec![(root.clone(), None, nbrs(root), 0)];
            while let Some((u, p, ns, next)) = stack.last_mut() {
                if *next < ns.len() {
                    let v = ns[*next].clone();
                    *next += 1;
                    if Some(&v) != p.as_ref() {
                        let u = u.clone();
                        visited.insert(v.clone());
                        uf.add_element(v.clone())?;
                        ancestors.insert(v.clone(), v.clone());
                        let ch = nbrs(&v);
                        stack.push((v, Some(u), ch, 0));
                    }
                } else {
                    let (u, p, _, _) = stack.pop().unwrap();
                    black.insert(u.clone());
                    if let Some(indexes) = query_occurs.get(&u) {
                        for &index in indexes {
                            let (qa, qb) = &queries[index];
                            let v = if *qa == u { qb } else { qa };
                            if black.contains(v) {
                                let rep = uf.find(v)?;
                                result[index] = ancestors.get(&rep).cloned();
                            }
                        }
                    }
                    // back in the parent: merge the finished child
                    if let Some(parent) = p {
                        uf.union(&parent, &u)?;
                        let rep = uf.find(&parent)?;
                        ancestors.insert(rep, parent);
                    }
                }
            }
        }
        Ok(result)
    }
}

impl<G: OutgoingGraph> LowestCommonAncestorAlgorithm<G::Vertex> for TarjanLcaFinder<'_, G> {
    fn lca(&self, a: &G::Vertex, b: &G::Vertex) -> Result<Option<G::Vertex>> {
        Ok(self.compute(&[(a.clone(), b.clone())])?.remove(0))
    }
    fn batch_lca(&self, queries: &[(G::Vertex, G::Vertex)]) -> Result<Vec<Option<G::Vertex>>> {
        self.compute(queries)
    }
}

/// LCA in directed acyclic graphs by intersecting ancestor sets (`NaiveLCAFinder`).
pub struct NaiveLcaFinder<'g, G: DirectedGraph> {
    graph: &'g G,
}

impl<'g, G: DirectedGraph + IncidenceGraph> NaiveLcaFinder<'g, G> {
    pub fn new(graph: &'g G) -> Result<Self> {
        Ok(NaiveLcaFinder { graph })
    }

    fn check(&self, a: &G::Vertex, b: &G::Vertex) -> Result<()> {
        if !self.graph.contains_vertex(a) || !self.graph.contains_vertex(b) {
            return Err(GraphError::VertexNotFound);
        }
        Ok(())
    }

    /// All ancestors of `start` (including itself), in BFS order.
    fn ancestors(&self, start: &G::Vertex) -> Result<indexmap::IndexSet<G::Vertex>> {
        let reversed = EdgeReversed(self.graph);
        let mut seen: indexmap::IndexSet<G::Vertex> = indexmap::IndexSet::from([start.clone()]);
        let mut queue = std::collections::VecDeque::from([start.clone()]);
        while let Some(v) = queue.pop_front() {
            let next: Vec<G::Vertex> = reversed
                .out_edges_of(&v)
                .filter_map(|e| reversed.edge_target(e).cloned())
                .collect();
            for w in next {
                if seen.insert(w.clone()) {
                    queue.push_back(w);
                }
            }
        }
        Ok(seen)
    }
}

impl<G: DirectedGraph + IncidenceGraph> LowestCommonAncestorAlgorithm<G::Vertex> for NaiveLcaFinder<'_, G> {
    fn lca(&self, a: &G::Vertex, b: &G::Vertex) -> Result<Option<G::Vertex>> {
        self.check(a, b)?;
        let set = self.lca_set_ordered(a, b)?;
        Ok(set.into_iter().next())
    }

    fn lca_set(&self, a: &G::Vertex, b: &G::Vertex) -> Result<HashSet<G::Vertex>> {
        self.check(a, b)?;
        Ok(self.lca_set_ordered(a, b)?.into_iter().collect())
    }
}

impl<G: DirectedGraph + IncidenceGraph> NaiveLcaFinder<'_, G> {
    fn lca_set_ordered(&self, a: &G::Vertex, b: &G::Vertex) -> Result<Vec<G::Vertex>> {
        let (a_anc, b_anc) = (self.ancestors(a)?, self.ancestors(b)?);
        let common: indexmap::IndexSet<G::Vertex> = a_anc
            .iter()
            .filter(|v| b_anc.contains(*v))
            .cloned()
            .collect();
        let mut lowest = Vec::new();
        for ancestor in &common {
            let has_common_child = self.graph.out_edges_of(ancestor).any(|edge| {
                self.graph
                    .edge_target(edge)
                    .is_some_and(|target| common.contains(target))
            });
            if !has_common_child {
                lowest.push(ancestor.clone());
            }
        }
        Ok(lowest)
    }
}
