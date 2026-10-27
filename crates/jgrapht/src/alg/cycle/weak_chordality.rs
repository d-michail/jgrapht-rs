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

use crate::graph::IncidenceGraph;
use crate::graph::{Graph, GraphExt};
use crate::util::java_int_hash_set::JavaIntHashSet;
use crate::walk::GraphWalk;
use indexmap::IndexSet;
use std::collections::{HashSet, VecDeque};

/// An undirected multigraph over vertex indices: the graph or its complement.
struct Adj {
    n: usize,
    /// `(source, target)` per edge, in edge order
    edges: Vec<(usize, usize)>,
    /// the vertex at the other end of every incident edge, per vertex, in `edgesOf` order
    neighbors: Vec<Vec<usize>>,
    adjacent: Vec<HashSet<usize>>,
}

impl Adj {
    fn contains_edge(&self, a: usize, b: usize) -> bool {
        self.adjacent[a].contains(&b)
    }

    fn complement(&self) -> Adj {
        let n = self.n;
        let mut edges = Vec::new();
        let mut neighbors = vec![Vec::new(); n];
        let mut adjacent = vec![HashSet::new(); n];
        for i in 0..n.saturating_sub(1) {
            for j in i + 1..n {
                if !self.contains_edge(i, j) {
                    edges.push((i, j));
                    neighbors[i].push(j);
                    neighbors[j].push(i);
                    adjacent[i].insert(j);
                    adjacent[j].insert(i);
                }
            }
        }
        Adj {
            n,
            edges,
            neighbors,
            adjacent,
        }
    }
}

type Separator = Vec<(usize, i32)>;
type SeparatorWithEdge = (Separator, (usize, usize));

/// Tests whether a graph is [weakly chordal](http://www.graphclasses.org/classes/gc_14.html). Weakly chordal graphs are also known as weakly triangulated graphs. Triangulated in
/// the context of chordality has a different meaning than triangulated in the context of planarity,
/// where it refers to a maximal planar graph, see:
/// [http://mathworld.wolfram.com/TriangulatedGraph.html](http://mathworld.wolfram.com/TriangulatedGraph.html)
///
/// The following definitions of are equivalent:
///
/// - A graph is weakly chordal (weakly triangulated) if neither it nor its complement contains a
/// [chordless cycles](http://mathworld.wolfram.com/ChordlessCycle.html) with five or more
/// vertices.
/// - A 2-pair in a graph is a pair of non-adjacent vertices $x$, $y$ such that every chordless
/// path has exactly two edges. A graph is weakly chordal if every connected
/// [induced subgraph](https://en.wikipedia.org/wiki/Induced_subgraph) $H$ that is not a
/// complete graph, contains a 2-pair.
///
/// Chordal and weakly chordal graphs are
/// [perfect](http://mathworld.wolfram.com/PerfectGraph.html).
/// For more details, refer to: Hayward, R.B. Weakly triangulated graphs, Journal of Combinatorial
/// Theory, Series B, vol 39, Issue 3, pp 200-208, 1985.
///
/// The implementation in this class is based on: Lars Severin Skeide (2002)
/// [*Recognizing weakly chordal graphs*](http://www.ii.uib.no/~skeide/rec_wcg.pdf).
/// Candidate Scientist Thesis in Informatics. Department of Informatics, University of Bergen,
/// Norway. The terminology used in this implementation is consistent with the one used in this
/// thesis.
///
/// Both the runtime complexity and space complexity of the algorithm implemented in this class is
/// $\mathcal{O}(|E|^2)$.
/// The inspected `graph` is specified at the construction time and cannot be modified. When
/// the graph is modified externally, the behavior of the `WeakChordalityInspector` is
/// undefined.
///
/// In the case the inspected graph in not weakly chordal, this inspector provides a certificate in
/// the form of some [hole](http://graphclasses.org/smallgraphs.html#holes) or
/// [anti-hole](http://graphclasses.org/smallgraphs.html#antiholes). The running time of
/// finding a hole is $\mathcal{O}(|V| + |E|)$ and of finding an anti-hole - $\mathcal{O}(|E|^2)$ in
/// the worst case.
///
/// # Rust notes
///
/// `WeakChordalityInspector`: tests whether neither the graph nor its complement has an induced
/// cycle of length at least five (Nikolopoulos and Palios), and finds such a hole or antihole.
/// Directed graphs are treated as undirected.
pub struct WeakChordalityInspector<'g, G: Graph> {
    graph: &'g G,
    adj: Adj,
    vertices: Vec<G::Vertex>,
    weakly_chordal: Option<bool>,
    certificate: Option<GraphWalk<G::Vertex, G::Edge>>,
}

impl<'g, G: IncidenceGraph> WeakChordalityInspector<'g, G> {
    /// Creates a weak chordality inspector for the `graph`
    pub fn new(graph: &'g G) -> Self {
        let vertices: Vec<G::Vertex> = graph.vertices().cloned().collect();
        let index: std::collections::HashMap<&G::Vertex, usize> =
            vertices.iter().enumerate().map(|(i, v)| (v, i)).collect();
        let n = vertices.len();
        let mut edges = Vec::new();
        for e in graph.edges() {
            let (s, t) = graph.edge_endpoints(e).expect("edge of the graph");
            edges.push((index[s], index[t]));
        }
        let mut neighbors = vec![Vec::new(); n];
        let mut adjacent = vec![HashSet::new(); n];
        for (i, v) in vertices.iter().enumerate() {
            for e in graph.edges_of(v) {
                let o = index[graph.opposite_vertex(e, v).expect("edge of the graph")];
                neighbors[i].push(o);
                adjacent[i].insert(o);
                // directed edges count in both directions
                adjacent[o].insert(i);
            }
        }
        WeakChordalityInspector {
            graph,
            adj: Adj {
                n,
                edges,
                neighbors,
                adjacent,
            },
            vertices,
            weakly_chordal: None,
            certificate: None,
        }
    }

    /// Check whether the inspected `graph` is weakly chordal. Note: this value is computed
    /// lazily.
    pub fn is_weakly_chordal(&mut self) -> bool {
        self.lazy_compute_weak_chordality()
    }

    /// A hole or antihole (as a closed vertex list) if the graph is not weakly chordal.
    pub fn get_certificate(&mut self) -> Option<&GraphWalk<G::Vertex, G::Edge>> {
        self.lazy_compute_weak_chordality();
        self.certificate.as_ref()
    }

    /// Lazily tests the weak chordality of the `graph` and returns the computed value.
    fn lazy_compute_weak_chordality(&mut self) -> bool {
        if let Some(r) = self.weakly_chordal {
            return r;
        }
        let r = self.compute();
        self.weakly_chordal = Some(r);
        r
    }

    fn compute(&mut self) -> bool {
        let n = self.adj.n;
        let m = self.adj.edges.len();
        let mut global = self.compute_global_separator_list();
        if global.is_empty() {
            return true;
        }
        Self::sort_separators_list(&mut global, n);
        let mut separators_num = 1;
        let mut original = global[0].0.clone();
        let mut co_connected = Self::compute_co_connected_components(&self.adj, &original);
        for (separator, hole_former) in &global {
            if Self::unequal_separators(&original, separator) {
                original = separator.clone();
                separators_num += 1;
                if n + m < separators_num {
                    return false;
                }
                co_connected = Self::compute_co_connected_components(&self.adj, &original);
            }
            if let Some((p0, p1)) = Self::check_labels(n, &co_connected, separator) {
                let (source, target) = *hole_former;
                let (mut source_in_separator, mut target_in_separator) = (p0, p1);
                if !self.adj.contains_edge(source, source_in_separator) {
                    std::mem::swap(&mut source_in_separator, &mut target_in_separator);
                }
                if self
                    .adj
                    .contains_edge(source_in_separator, target_in_separator)
                {
                    self.find_anti_hole(source, target_in_separator);
                } else {
                    let cycle = Self::find_hole_cycle(
                        &self.adj,
                        source_in_separator,
                        source,
                        target,
                        target_in_separator,
                    );
                    self.certificate = Some(self.walk_of(&cycle, true));
                }
                return false;
            }
        }
        true
    }

    /// The cycle as a walk: the edges of the graph for a hole, none for an antihole (whose
    /// consecutive vertices are not adjacent).
    fn walk_of(&self, cycle: &[usize], hole: bool) -> GraphWalk<G::Vertex, G::Edge> {
        let vertices: Vec<G::Vertex> = cycle.iter().map(|&i| self.vertices[i].clone()).collect();
        if hole {
            if let Ok(w) = GraphWalk::from_vertex_list(self.graph, vertices.clone(), 0.0) {
                return w;
            }
        }
        GraphWalk::new(vertices, Vec::new(), 0.0)
    }

    /// Computes the global separator list of the `graph`. More precisely, for every edge $e$
    /// in the $G = (V, E)$ computes list of minimal separators $S_e$ in the neighborhood of $e$ and
    /// then concatenates these lists. Note: the result may contain duplicates
    fn compute_global_separator_list(&self) -> Vec<SeparatorWithEdge> {
        let mut global = Vec::new();
        for &(source, target) in &self.adj.edges {
            if source != target {
                let edge_separators = Self::find_separators(&self.adj, source, target);
                global.extend(self.reformat_separator_list(&edge_separators, source, target));
            }
        }
        global
    }

    /// Reformats the list o `separators` so that is can be conveniently used by this
    /// inspector. More precisely, in every separator from the list of minimal separators in the
    /// neighborhood of the `edge` substitutes all vertices for their indices in the numeration
    /// defined by `vertices`. Pairs every separator with the `edge`.
    fn reformat_separator_list(
        &self,
        separators: &[HashSet<usize>],
        source: usize,
        target: usize,
    ) -> Vec<SeparatorWithEdge> {
        self.reformat_separator_list_in(&self.adj, separators, source, target)
    }

    fn reformat_separator_list_in(
        &self,
        adj: &Adj,
        separators: &[HashSet<usize>],
        source: usize,
        target: usize,
    ) -> Vec<SeparatorWithEdge> {
        let n = self.adj.n;
        let labeling = Self::get_labeling(adj, source, target);
        let mut reformatted: Vec<SeparatorWithEdge> = Vec::with_capacity(separators.len());
        let mut v_in_separator: Vec<Vec<usize>> = vec![Vec::new(); n];
        for (k, computed) in separators.iter().enumerate() {
            reformatted.push((Vec::with_capacity(computed.len()), (source, target)));
            for &vertex in computed {
                v_in_separator[vertex].push(k);
            }
        }
        for vertex in 0..n {
            for &k in &v_in_separator[vertex] {
                reformatted[k]
                    .0
                    .push((vertex, labeling[vertex].unwrap_or(0)));
            }
        }
        reformatted
    }

    /// 1: adjacent to the source, 2: adjacent to the target only, 3: adjacent to both.
    fn get_labeling(adj: &Adj, source: usize, target: usize) -> Vec<Option<i32>> {
        let mut labeling: Vec<Option<i32>> = vec![None; adj.n];
        for &o in &adj.neighbors[source] {
            labeling[o] = Some(1);
        }
        for &o in &adj.neighbors[target] {
            labeling[o] = if labeling[o].is_some() {
                Some(3)
            } else {
                Some(2)
            };
        }
        labeling
    }

    /// Radix sort of the separators by their vertices, last vertex first.
    fn sort_separators_list(separators: &mut Vec<SeparatorWithEdge>, n: usize) {
        let mut main_queue: VecDeque<SeparatorWithEdge> = VecDeque::new();
        let mut max_separator_length = 0;
        for separator in separators.drain(..) {
            max_separator_length = max_separator_length.max(separator.0.len());
            main_queue.push_back(separator);
        }
        let mut queues: Vec<VecDeque<SeparatorWithEdge>> =
            (0..n).map(|_| VecDeque::new()).collect();
        for i in 0..max_separator_length {
            while let Some(separator) = main_queue.pop_front() {
                if i >= separator.0.len() {
                    separators.push(separator);
                } else {
                    let v = separator.0[separator.0.len() - i - 1].0;
                    queues[v].push_back(separator);
                }
            }
            for queue in queues.iter_mut() {
                main_queue.append(queue);
            }
        }
        separators.extend(main_queue);
    }

    /// Compares two separators for inequality. Labeling of the vertices in the separators isn't
    /// considered
    fn unequal_separators(sep1: &Separator, sep2: &Separator) -> bool {
        if sep1.len() == sep2.len() {
            sep1.iter().zip(sep2).any(|(a, b)| a.0 != b.0)
        } else {
            true
        }
    }

    /// The connected components of the complement of the subgraph induced by the separator.
    fn compute_co_connected_components(adj: &Adj, separator: &Separator) -> Vec<Vec<usize>> {
        let n = adj.n;
        let mut co_connected_components = Vec::new();
        if separator.is_empty() {
            return co_connected_components;
        }
        let mut buckets_by_label: Vec<JavaIntHashSet> = (0..separator.len())
            .map(|_| JavaIntHashSet::new())
            .collect();
        buckets_by_label[0] = JavaIntHashSet::with_expected_size(separator.len());
        let mut labels: Vec<i64> = vec![-1; n];
        for &(v, _) in separator {
            buckets_by_label[0].insert(v as i32);
            labels[v] = 0;
        }
        let mut min_label = 0usize;
        // bucket 0 holds the vertices not yet in a component (`unvisited` in the Java code)
        while !buckets_by_label[0].is_empty() {
            let mut co_connected_component: Vec<usize> = Vec::new();
            loop {
                while !buckets_by_label[min_label].is_empty() {
                    let vertex = buckets_by_label[min_label]
                        .first()
                        .expect("non-empty bucket");
                    buckets_by_label[min_label].remove(vertex);
                    let vertex = vertex as usize;
                    co_connected_component.push(vertex);
                    labels[vertex] = -1;
                    for &opposite in &adj.neighbors[vertex] {
                        let opposite_label = labels[opposite];
                        if opposite_label != -1 {
                            let l = opposite_label as usize;
                            buckets_by_label[l].remove(opposite as i32);
                            buckets_by_label[l + 1].insert(opposite as i32);
                            labels[opposite] = opposite_label + 1;
                        }
                    }
                }
                min_label += 1;
                if min_label == co_connected_component.len() {
                    break;
                }
            }
            // reload: the vertices stuck in the next bucket start over
            if min_label != 0 && min_label < buckets_by_label.len() {
                let vertices: Vec<i32> = buckets_by_label[min_label].iter().collect();
                buckets_by_label[min_label].clear();
                for vertex in vertices {
                    labels[vertex as usize] = 0;
                    buckets_by_label[0].insert(vertex);
                }
            }
            co_connected_components.push(co_connected_component);
            min_label = 0;
        }
        co_connected_components
    }

    /// Two vertices of one co-connected component whose labels differ (and are not both 3).
    fn check_labels(
        n: usize,
        co_connected_components: &[Vec<usize>],
        separator: &Separator,
    ) -> Option<(usize, usize)> {
        let mut vertex_labels: Vec<Option<i32>> = vec![None; n];
        for &(v, label) in separator {
            vertex_labels[v] = Some(label);
        }
        for component in co_connected_components {
            let mut label = 0;
            let mut label_vertex = None;
            for &vertex in component {
                let vl = vertex_labels[vertex].unwrap_or(0);
                if vl != 3 {
                    if label != 0 {
                        if label != vl {
                            return Some((label_vertex.expect("label vertex"), vertex));
                        }
                    } else {
                        label = vl;
                        label_vertex = Some(vertex);
                    }
                }
            }
        }
        None
    }

    /// Finds an anti-hole in the inspected `graph`. Vertices `source` and
    /// `targetInSeparator` specify an edge, which belongs to the anti-hole in the complement
    /// of the `graph`. Then the hole in the complement of the graph is computed in the graph's
    /// complement in the same way a hole is computed in the `graph`.
    fn find_anti_hole(&mut self, source: usize, target_in_separator: usize) {
        let complement = self.adj.complement();
        // the complement edge joining `source` and `target_in_separator`, as stored
        let &(cycle_source, cycle_target) = complement
            .edges
            .iter()
            .find(|&&(a, b)| {
                (a == source && b == target_in_separator)
                    || (a == target_in_separator && b == source)
            })
            .expect("source and target in the separator are not adjacent");
        let separators = Self::find_separators(&complement, cycle_source, cycle_target);
        let mut reformatted = self.reformat_separator_list(&separators, cycle_source, cycle_target);
        Self::sort_separators_list(&mut reformatted, self.adj.n);
        if reformatted.is_empty() {
            return;
        }
        let mut original = reformatted[0].0.clone();
        let mut co_connected = Self::compute_co_connected_components(&complement, &original);
        for (separator, _) in &reformatted {
            if Self::unequal_separators(separator, &original) {
                original = separator.clone();
                co_connected = Self::compute_co_connected_components(&complement, separator);
            }
            if let Some((p0, p1)) = Self::check_labels(self.adj.n, &co_connected, separator) {
                let (mut cycle_source_in_separator, mut cycle_target_in_separator) = (p0, p1);
                if !complement.contains_edge(cycle_source_in_separator, cycle_source) {
                    std::mem::swap(
                        &mut cycle_source_in_separator,
                        &mut cycle_target_in_separator,
                    );
                }
                let cycle = Self::find_hole_cycle(
                    &complement,
                    cycle_source_in_separator,
                    cycle_source,
                    cycle_target,
                    cycle_target_in_separator,
                );
                self.certificate = Some(self.walk_of(&cycle, false));
                return;
            }
        }
    }

    fn find_hole_cycle(
        adj: &Adj,
        source_in_separator: usize,
        source: usize,
        target: usize,
        target_in_separator: usize,
    ) -> Vec<usize> {
        let mut visited: HashSet<usize> = HashSet::new();
        visited.insert(target);
        visited.insert(source);
        let cycle = Self::find_cycle(
            &mut visited,
            adj,
            target_in_separator,
            target,
            source,
            source_in_separator,
        );
        Self::minimize_cycle(
            adj,
            &cycle,
            target,
            target_in_separator,
            source,
            source_in_separator,
        )
    }

    /// Starts the iterative depth-first traversal from `sourInSep` vertex. Tries to build a
    /// cycle with the vertices, which aren't adjacent to the `tar` and `sour`. This
    /// condition is used in order to ensure that the cycle contains a hole, to which it is later
    /// minimized.
    fn find_cycle(
        visited: &mut HashSet<usize>,
        adj: &Adj,
        tar_in_sep: usize,
        tar: usize,
        sour: usize,
        sour_in_sep: usize,
    ) -> Vec<usize> {
        let mut cycle = vec![tar_in_sep, tar, sour];
        let mut stack = vec![sour_in_sep];
        while let Some(current_vertex) = stack.pop() {
            if visited.insert(current_vertex) {
                while !adj.contains_edge(*cycle.last().expect("non-empty"), current_vertex) {
                    cycle.pop();
                }
                cycle.push(current_vertex);
                if tar_in_sep == current_vertex {
                    break;
                }
                for &neighbor in &adj.neighbors[current_vertex] {
                    if !visited.contains(&neighbor)
                        && !adj.contains_edge(sour, neighbor)
                        && (!adj.contains_edge(tar, neighbor) || neighbor == tar_in_sep)
                    {
                        stack.push(neighbor);
                    }
                }
            }
        }
        cycle
    }

    /// Minimizes the `cycle` so that it contains a hole in the `graph`. Vertices
    /// `tar`, `tarInSep`, `sour` and `sourInSep` belong to the final result.
    fn minimize_cycle(
        adj: &Adj,
        cycle: &[usize],
        tar: usize,
        tar_in_sep: usize,
        sour: usize,
        sour_in_sep: usize,
    ) -> Vec<usize> {
        let mut minimized_cycle = vec![tar_in_sep, tar, sour];
        let mut forward_vertices: HashSet<usize> = cycle.iter().copied().collect();
        forward_vertices.remove(&tar);
        forward_vertices.remove(&sour);
        forward_vertices.remove(&sour_in_sep);
        let mut i = 3;
        while i + 1 < cycle.len() {
            let current = cycle[i];
            minimized_cycle.push(current);
            forward_vertices.remove(&current);
            let mut current_forward: IndexSet<usize> = IndexSet::new();
            for &neighbor in &adj.neighbors[current] {
                if forward_vertices.contains(&neighbor) {
                    current_forward.insert(neighbor);
                }
            }
            for forward_vertex in current_forward {
                if forward_vertices.contains(&forward_vertex) {
                    loop {
                        forward_vertices.remove(&cycle[i]);
                        i += 1;
                        if !(i < cycle.len() && cycle[i] != forward_vertex) {
                            break;
                        }
                    }
                }
            }
        }
        minimized_cycle.push(tar_in_sep);
        minimized_cycle
    }

    /// The separators of the common non-neighbours of the edge's ends: the neighbours of a
    /// component of the graph without the edge's closed neighbourhood.
    fn find_separators(adj: &Adj, source: usize, target: usize) -> Vec<HashSet<usize>> {
        let mut separators = Vec::new();
        let neighborhood = Self::neighborhood_set_of(adj, source, target);
        let mut dfs_map: Vec<u8> = (0..adj.n)
            .map(|v| if neighborhood.contains(&v) { 1 } else { 0 })
            .collect();
        dfs_map[source] = 2;
        dfs_map[target] = 2;
        for vertex in 0..adj.n {
            if dfs_map[vertex] == 0 {
                let separator = Self::get_separator(adj, vertex, &mut dfs_map);
                if !separator.is_empty() {
                    separators.push(separator);
                }
            }
        }
        separators
    }

    /// Performs iterative depth-first search starting from the `startVertex` in the
    /// `graph`. Adds every encountered red vertex to the resulting separator. Doesn't process
    /// red vertices. Marks all white vertices with black color.
    fn get_separator(adj: &Adj, start_vertex: usize, dfs_map: &mut [u8]) -> HashSet<usize> {
        let mut stack = vec![start_vertex];
        let mut separator = HashSet::new();
        while let Some(current_vertex) = stack.pop() {
            if dfs_map[current_vertex] == 0 {
                dfs_map[current_vertex] = 2;
                for &opposite in &adj.neighbors[current_vertex] {
                    if dfs_map[opposite] == 0 {
                        stack.push(opposite);
                    } else if dfs_map[opposite] == 1 {
                        separator.insert(opposite); // a red vertex belongs to the separator
                    }
                }
            }
        }
        separator
    }

    /// Returns a set of vertices that are neighbors of the source of the specified edge or of the
    /// target of specified edge. The endpoints of the specified edge aren't included in the result.
    fn neighborhood_set_of(adj: &Adj, source: usize, target: usize) -> HashSet<usize> {
        let mut neighborhood: HashSet<usize> = HashSet::new();
        neighborhood.extend(adj.neighbors[source].iter().copied());
        neighborhood.extend(adj.neighbors[target].iter().copied());
        neighborhood.remove(&source);
        neighborhood.remove(&target);
        neighborhood
    }
}
