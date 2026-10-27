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

use crate::error::{GraphError, Result};
use crate::graph::props::has_multiple_edges;
use crate::graph::{DirectedGraph, Graph};
use indexmap::IndexSet;
use std::collections::HashSet;

/// `DirectedSimpleCycles`: enumerates the simple cycles of a directed graph.
pub trait DirectedSimpleCycles<V> {
    /// Calls `consumer` with every simple cycle (its vertices in order, without repeating the
    /// first one).
    fn find_simple_cycles_with(&mut self, consumer: &mut dyn FnMut(Vec<V>)) -> Result<()>;

    fn find_simple_cycles(&mut self) -> Result<Vec<Vec<V>>> {
        let mut result = Vec::new();
        self.find_simple_cycles_with(&mut |c| result.push(c))?;
        Ok(result)
    }
}

/// Vertices in graph order and the targets of every vertex's outgoing edges (parallel edges give
/// repeated targets).
fn index_graph<G: Graph>(g: &G) -> (Vec<G::Vertex>, Vec<Vec<usize>>) {
    let vs: Vec<G::Vertex> = g.vertices().cloned().collect();
    let index: std::collections::HashMap<&G::Vertex, usize> =
        vs.iter().enumerate().map(|(i, v)| (v, i)).collect();
    let out = vs
        .iter()
        .map(|v| {
            g.out_edges_of(v).unwrap()
                .map(|e| index[g.edge_target(e).expect("edge of the graph")])
                .collect()
        })
        .collect();
    (vs, out)
}

fn no_graph<T>() -> Result<T> {
    Err(GraphError::InvalidArgument("Null graph.".into()))
}

/// Implements the shared plumbing of an algorithm struct with `graph: Option<&G>` and a
/// `run(out, emit)` method over adjacency lists.
macro_rules! cycles_algorithm {
    ($name:ident) => {
        impl<'g, G: DirectedGraph> Default for $name<'g, G> {
            fn default() -> Self {
                $name { graph: None }
            }
        }

        impl<'g, G: DirectedGraph> $name<'g, G> {
            /// An algorithm without a graph; call [`set_graph`](Self::set_graph) before use.
            pub fn new() -> Self {
                Self::default()
            }

            pub fn with_graph(graph: &'g G) -> Self {
                $name { graph: Some(graph) }
            }

            pub fn graph(&self) -> Option<&'g G> {
                self.graph
            }

            pub fn set_graph(&mut self, graph: &'g G) {
                self.graph = Some(graph);
            }
        }

        impl<'g, G: DirectedGraph> DirectedSimpleCycles<G::Vertex> for $name<'g, G> {
            fn find_simple_cycles_with(
                &mut self,
                consumer: &mut dyn FnMut(Vec<G::Vertex>),
            ) -> Result<()> {
                let Some(graph) = self.graph else {
                    return no_graph();
                };
                let (vs, out) = index_graph(graph);
                Self::run(graph, &out, &mut |cycle: &[usize]| {
                    consumer(cycle.iter().map(|&i| vs[i].clone()).collect())
                });
                Ok(())
            }
        }
    };
}

/// `TiernanSimpleCycles`: the oldest and simplest enumeration, exponential in the worst case.
pub struct TiernanSimpleCycles<'g, G: DirectedGraph> {
    graph: Option<&'g G>,
}
cycles_algorithm!(TiernanSimpleCycles);

impl<'g, G: DirectedGraph> TiernanSimpleCycles<'g, G> {
    fn run(_graph: &G, out: &[Vec<usize>], emit: &mut dyn FnMut(&[usize])) {
        let n = out.len();
        if n == 0 {
            return;
        }
        let adjacent: Vec<HashSet<usize>> =
            out.iter().map(|o| o.iter().copied().collect()).collect();
        let mut path: Vec<usize> = Vec::new();
        let mut path_set = vec![false; n];
        let mut blocked: Vec<HashSet<usize>> = vec![HashSet::new(); n];
        let mut next_start = 1;
        let mut end_of_path = 0;
        path.push(end_of_path);
        path_set[end_of_path] = true;
        loop {
            loop {
                let mut extension_found = false;
                for &nn in &out[end_of_path] {
                    if nn > path[0] && !path_set[nn] && !blocked[end_of_path].contains(&nn) {
                        path.push(nn);
                        path_set[nn] = true;
                        end_of_path = nn;
                        extension_found = true;
                        break;
                    }
                }
                if !extension_found {
                    break;
                }
            }
            let start_of_path = path[0];
            if adjacent[end_of_path].contains(&start_of_path) {
                emit(&path);
            }
            if path.len() > 1 {
                blocked[end_of_path].clear();
                path.pop();
                path_set[end_of_path] = false;
                let temp = end_of_path;
                end_of_path = *path.last().expect("non-empty path");
                blocked[end_of_path].insert(temp);
                continue;
            }
            if next_start < n {
                path.clear();
                path_set.iter_mut().for_each(|b| *b = false);
                end_of_path = next_start;
                next_start += 1;
                path.push(end_of_path);
                path_set[end_of_path] = true;
                blocked.iter_mut().for_each(HashSet::clear);
                continue;
            }
            break;
        }
    }
}

/// `TarjanSimpleCycles`: Tarjan's 1973 backtracking enumeration with marking.
pub struct TarjanSimpleCycles<'g, G: DirectedGraph> {
    graph: Option<&'g G>,
}
cycles_algorithm!(TarjanSimpleCycles);

struct TarjanState<'a> {
    out: &'a [Vec<usize>],
    removed: Vec<HashSet<usize>>,
    marked: Vec<bool>,
    marked_stack: Vec<usize>,
    point_stack: Vec<usize>,
}

impl TarjanState<'_> {
    fn backtrack(&mut self, start: usize, vertex: usize, emit: &mut dyn FnMut(&[usize])) -> bool {
        let mut found_cycle = false;
        self.point_stack.push(vertex);
        self.marked[vertex] = true;
        self.marked_stack.push(vertex);
        let out = self.out;
        for &current in &out[vertex] {
            if self.removed[vertex].contains(&current) {
                continue;
            }
            if current < start {
                self.removed[vertex].insert(current);
            } else if current == start {
                found_cycle = true;
                emit(&self.point_stack);
            } else if !self.marked[current] {
                let got_cycle = self.backtrack(start, current, emit);
                found_cycle = found_cycle || got_cycle;
            }
        }
        if found_cycle {
            while *self.marked_stack.last().expect("marked") != vertex {
                let top = self.marked_stack.pop().expect("marked");
                self.marked[top] = false;
            }
            let top = self.marked_stack.pop().expect("marked");
            self.marked[top] = false;
        }
        self.point_stack.pop();
        found_cycle
    }
}

impl<'g, G: DirectedGraph> TarjanSimpleCycles<'g, G> {
    fn run(_graph: &G, out: &[Vec<usize>], emit: &mut dyn FnMut(&[usize])) {
        let n = out.len();
        let mut state = TarjanState {
            out,
            removed: vec![HashSet::new(); n],
            marked: vec![false; n],
            marked_stack: Vec::new(),
            point_stack: Vec::new(),
        };
        for start in 0..n {
            state.backtrack(start, start, emit);
            while let Some(top) = state.marked_stack.pop() {
                state.marked[top] = false;
            }
        }
    }
}

/// Tarjan's strongly connected components on the subgraph of vertices `>= min_index`; returns the
/// components (vertices in discovery order) with a vertex-set that are not trivial.
fn strongly_connected_components(
    out: &[Vec<usize>],
    min_index: usize,
    only_nontrivial: bool,
) -> Vec<Vec<usize>> {
    const NONE: usize = usize::MAX;
    let n = out.len();
    let mut index = vec![NONE; n];
    let mut low = vec![0usize; n];
    let mut on_path = vec![false; n];
    let mut path: Vec<usize> = Vec::new();
    let mut next_index = 0;
    let mut components = Vec::new();
    for root in min_index..n {
        if index[root] != NONE {
            continue;
        }
        // explicit recursion stack: (vertex, next child position)
        let mut call = vec![(root, 0usize)];
        index[root] = next_index;
        low[root] = next_index;
        next_index += 1;
        path.push(root);
        on_path[root] = true;
        while let Some(&mut (v, ref mut child)) = call.last_mut() {
            if *child < out[v].len() {
                let w = out[v][*child];
                *child += 1;
                if w < min_index {
                    continue;
                }
                if index[w] == NONE {
                    index[w] = next_index;
                    low[w] = next_index;
                    next_index += 1;
                    path.push(w);
                    on_path[w] = true;
                    call.push((w, 0));
                } else if on_path[w] {
                    low[v] = low[v].min(index[w]);
                }
            } else {
                call.pop();
                if let Some(&(parent, _)) = call.last() {
                    low[parent] = low[parent].min(low[v]);
                }
                if low[v] == index[v] {
                    let mut component = Vec::new();
                    loop {
                        let w = path.pop().expect("component member");
                        on_path[w] = false;
                        component.push(w);
                        if w == v {
                            break;
                        }
                    }
                    let keep = !only_nontrivial
                        || component.len() > 1
                        || out[component[0]].contains(&component[0]);
                    if keep {
                        components.push(component);
                    }
                }
            }
        }
    }
    components
}

/// `JohnsonSimpleCycles`: Johnson's 1975 algorithm, `O((n + e)(c + 1))` for `c` cycles.
pub struct JohnsonSimpleCycles<'g, G: DirectedGraph> {
    graph: &'g G,
}

impl<'g, G: DirectedGraph> JohnsonSimpleCycles<'g, G> {
    /// Fails when the graph has parallel edges.
    pub fn new(graph: &'g G) -> Result<Self> {
        if has_multiple_edges(graph) {
            return Err(GraphError::InvalidArgument(
                "Graph should not have multiple (parallel) edges".into(),
            ));
        }
        Ok(JohnsonSimpleCycles { graph })
    }
}

struct JohnsonState<'a> {
    scg: &'a [Vec<usize>],
    blocked: Vec<bool>,
    b_sets: Vec<IndexSet<usize>>,
    stack: Vec<usize>,
}

impl JohnsonState<'_> {
    fn find_cycles_in_scg(
        &mut self,
        start_index: usize,
        vertex: usize,
        emit: &mut dyn FnMut(&[usize]),
    ) -> bool {
        let mut found_cycle = false;
        self.stack.push(vertex);
        self.blocked[vertex] = true;
        let scg = self.scg;
        for &successor in &scg[vertex] {
            if successor == start_index {
                emit(&self.stack);
                found_cycle = true;
            } else if !self.blocked[successor] {
                let got_cycle = self.find_cycles_in_scg(start_index, successor, emit);
                found_cycle = found_cycle || got_cycle;
            }
        }
        if found_cycle {
            self.unblock(vertex);
        } else {
            for &w in &scg[vertex] {
                self.b_sets[w].insert(vertex);
            }
        }
        self.stack.pop();
        found_cycle
    }

    fn unblock(&mut self, vertex: usize) {
        self.blocked[vertex] = false;
        while let Some(w) = self.b_sets[vertex].shift_remove_index(0) {
            if self.blocked[w] {
                self.unblock(w);
            }
        }
    }
}

impl<'g, G: DirectedGraph> DirectedSimpleCycles<G::Vertex> for JohnsonSimpleCycles<'g, G> {
    fn find_simple_cycles_with(&mut self, consumer: &mut dyn FnMut(Vec<G::Vertex>)) -> Result<()> {
        let (vs, out) = index_graph(self.graph);
        let size = vs.len();
        let mut emit = |cycle: &[usize]| consumer(cycle.iter().map(|&i| vs[i].clone()).collect());
        let mut blocked = vec![false; size];
        let mut b_sets: Vec<IndexSet<usize>> = vec![IndexSet::new(); size];
        let mut start_index = 0;
        while start_index < size {
            // the strongly connected component holding the lowest vertex >= start_index
            let components = strongly_connected_components(&out, start_index, true);
            let mut min_index_found = usize::MAX;
            let mut min_scc: Option<&Vec<usize>> = None;
            for scc in &components {
                for &v in scc {
                    if v < min_index_found {
                        min_index_found = v;
                        min_scc = Some(scc);
                    }
                }
            }
            let Some(min_scc) = min_scc else { break };
            let mut members = min_scc.clone();
            members.sort_unstable();
            let mut scg: Vec<Vec<usize>> = vec![Vec::new(); size];
            for &v in &members {
                for &w in &members {
                    if out[v].contains(&w) {
                        scg[v].push(w);
                    }
                }
            }
            start_index = min_index_found;
            for &v in &scg[start_index] {
                blocked[v] = false;
                b_sets[v].clear();
            }
            let mut state = JohnsonState {
                scg: &scg,
                blocked: std::mem::take(&mut blocked),
                b_sets: std::mem::take(&mut b_sets),
                stack: Vec::new(),
            };
            state.find_cycles_in_scg(start_index, start_index, &mut emit);
            blocked = state.blocked;
            b_sets = state.b_sets;
            start_index += 1;
        }
        Ok(())
    }
}

/// `SzwarcfiterLauerSimpleCycles`: Szwarcfiter and Lauer's 1974 algorithm, `O(n + e)` per cycle.
pub struct SzwarcfiterLauerSimpleCycles<'g, G: DirectedGraph> {
    graph: Option<&'g G>,
}
cycles_algorithm!(SzwarcfiterLauerSimpleCycles);

struct SzwarcfiterLauerState<'a> {
    out: &'a [Vec<usize>],
    b_sets: Vec<IndexSet<usize>>,
    stack: Vec<usize>,
    marked: Vec<bool>,
    removed: Vec<HashSet<usize>>,
    position: Vec<usize>,
    reach: Vec<bool>,
}

impl SzwarcfiterLauerState<'_> {
    fn cycle(&mut self, v: usize, mut q: usize, emit: &mut dyn FnMut(&[usize])) -> bool {
        let mut found_cycle = false;
        self.marked[v] = true;
        self.stack.push(v);
        let t = self.stack.len();
        self.position[v] = t;
        if !self.reach[v] {
            q = t;
        }
        let out = self.out;
        for &w in &out[v] {
            if self.removed[v].contains(&w) {
                continue;
            }
            if !self.marked[w] {
                let got_cycle = self.cycle(w, q, emit);
                if got_cycle {
                    found_cycle = true;
                } else {
                    self.no_cycle(v, w);
                }
            } else if self.position[w] <= q {
                found_cycle = true;
                let from = self
                    .stack
                    .iter()
                    .position(|&x| x == w)
                    .expect("w is on the stack");
                emit(&self.stack[from..]);
            } else {
                self.no_cycle(v, w);
            }
        }
        self.stack.pop();
        if found_cycle {
            self.unmark(v);
        }
        self.reach[v] = true;
        self.position[v] = self.out.len();
        found_cycle
    }

    fn no_cycle(&mut self, x: usize, y: usize) {
        self.b_sets[y].insert(x);
        self.removed[x].insert(y);
    }

    fn unmark(&mut self, x: usize) {
        self.marked[x] = false;
        let bx = std::mem::take(&mut self.b_sets[x]);
        for y in bx {
            self.removed[y].remove(&x);
            if self.marked[y] {
                self.unmark(y);
            }
        }
    }
}

impl<'g, G: DirectedGraph> SzwarcfiterLauerSimpleCycles<'g, G> {
    fn run(_graph: &G, out: &[Vec<usize>], emit: &mut dyn FnMut(&[usize])) {
        let n = out.len();
        let mut in_degree = vec![0usize; n];
        for targets in out {
            for &t in targets {
                in_degree[t] += 1;
            }
        }
        let mut start_vertices = Vec::new();
        for mut scc in strongly_connected_components(out, 0, false) {
            scc.sort_unstable();
            let mut max_in_degree: i64 = -1;
            let mut start_vertex = usize::MAX;
            for v in scc {
                if in_degree[v] as i64 > max_in_degree {
                    max_in_degree = in_degree[v] as i64;
                    start_vertex = v;
                }
            }
            start_vertices.push(start_vertex);
        }
        let mut state = SzwarcfiterLauerState {
            out,
            b_sets: vec![IndexSet::new(); n],
            stack: Vec::new(),
            marked: vec![false; n],
            removed: vec![HashSet::new(); n],
            position: vec![0; n],
            reach: vec![false; n],
        };
        for vertex in start_vertices {
            state.cycle(vertex, 0, emit);
        }
    }
}

/// `HawickJamesSimpleCycles`: Hawick and James's 2008 algorithm; also counts cycles and can limit
/// the cycle length.
pub struct HawickJamesSimpleCycles<'g, G: DirectedGraph> {
    graph: Option<&'g G>,
    path_limit: usize,
    has_limit: bool,
}

impl<'g, G: DirectedGraph> Default for HawickJamesSimpleCycles<'g, G> {
    fn default() -> Self {
        HawickJamesSimpleCycles {
            graph: None,
            path_limit: 0,
            has_limit: false,
        }
    }
}

struct HawickJamesState<'a> {
    a_k: &'a [Vec<usize>],
    b: Vec<Vec<usize>>,
    blocked: Vec<bool>,
    stack: Vec<usize>,
    start: usize,
    path_limit: usize,
    has_limit: bool,
}

impl HawickJamesState<'_> {
    fn limit_reached(&self, steps: usize) -> bool {
        self.has_limit && steps >= self.path_limit
    }

    fn circuit(&mut self, v: usize, steps: usize, operation: &mut dyn FnMut(&[usize])) -> bool {
        let mut f = false;
        self.stack.push(v);
        self.blocked[v] = true;
        let a_k = self.a_k;
        for &w in &a_k[v] {
            if w < self.start {
                continue;
            }
            if w == self.start {
                operation(&self.stack);
                f = true;
            } else if !self.blocked[w]
                && (self.limit_reached(steps) || self.circuit(w, steps + 1, operation))
            {
                f = true;
            }
        }
        if f {
            self.unblock(v);
        } else {
            for &w in &a_k[v] {
                if w < self.start {
                    continue;
                }
                if !self.b[w].contains(&v) {
                    self.b[w].push(v);
                }
            }
        }
        self.stack.pop();
        f
    }

    fn unblock(&mut self, u: usize) {
        self.blocked[u] = false;
        let mut w_pos: isize = 0;
        while (w_pos as usize) < self.b[u].len() {
            let w = self.b[u][w_pos as usize];
            let size_before_remove = self.b[u].len();
            self.b[u].retain(|&x| x != w);
            w_pos -= (size_before_remove - self.b[u].len()) as isize;
            if self.blocked[w] {
                self.unblock(w);
            }
            w_pos += 1;
        }
    }
}

impl<'g, G: DirectedGraph> HawickJamesSimpleCycles<'g, G> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_graph(graph: &'g G) -> Self {
        HawickJamesSimpleCycles {
            graph: Some(graph),
            ..Self::default()
        }
    }

    pub fn graph(&self) -> Option<&'g G> {
        self.graph
    }

    pub fn set_graph(&mut self, graph: &'g G) {
        self.graph = Some(graph);
    }

    /// Only report cycles of at most `path_limit` vertices... (the Java semantics: the search
    /// stops extending a path after `path_limit - 1` steps).
    pub fn set_path_limit(&mut self, path_limit: usize) {
        self.path_limit = path_limit.saturating_sub(1);
        self.has_limit = true;
    }

    pub fn clear_path_limit(&mut self) {
        self.has_limit = false;
    }

    fn analyze_circuits(&self, out: &[Vec<usize>], operation: &mut dyn FnMut(&[usize])) {
        let n = out.len();
        let mut state = HawickJamesState {
            a_k: out,
            b: vec![Vec::new(); n],
            blocked: vec![false; n],
            stack: Vec::with_capacity(n),
            start: 0,
            path_limit: self.path_limit,
            has_limit: self.has_limit,
        };
        for i in 0..n {
            state.blocked.iter_mut().for_each(|b| *b = false);
            state.b.iter_mut().for_each(Vec::clear);
            state.start = i;
            state.circuit(i, 0, operation);
        }
    }

    /// The number of simple cycles, without materializing them.
    pub fn count_simple_cycles(&self) -> Result<u64> {
        let Some(graph) = self.graph else {
            return no_graph();
        };
        let (_, out) = index_graph(graph);
        let mut count = 0;
        self.analyze_circuits(&out, &mut |_| count += 1);
        Ok(count)
    }
}

impl<'g, G: DirectedGraph> DirectedSimpleCycles<G::Vertex> for HawickJamesSimpleCycles<'g, G> {
    fn find_simple_cycles_with(&mut self, consumer: &mut dyn FnMut(Vec<G::Vertex>)) -> Result<()> {
        let Some(graph) = self.graph else {
            return no_graph();
        };
        let (vs, out) = index_graph(graph);
        self.analyze_circuits(&out, &mut |cycle: &[usize]| {
            consumer(cycle.iter().map(|&i| vs[i].clone()).collect())
        });
        Ok(())
    }
}
