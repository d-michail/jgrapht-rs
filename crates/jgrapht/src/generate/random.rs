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

use super::structured::Complete;
use super::{GeneratorResult, GraphGenerator, add_edge, invalid};
use crate::graph::Graph;
use crate::graph::IncidenceGraph;
use crate::util::java::{RandomSource, java_shuffle};
use crate::util::sampler::AliasMethodSampler;
use crate::{GraphError, GraphMut, GraphType, Result};
use indexmap::IndexSet;
use std::collections::{HashMap, HashSet};

fn check_probability(p: f64) -> Result<f64> {
    if (0.0..=1.0).contains(&p) {
        Ok(p)
    } else {
        invalid("not valid probability of edge existence")
    }
}

fn new_vertices<G: GraphMut>(target: &mut G, n: usize) -> Result<Vec<G::Vertex>> {
    let previous = target.vertex_count();
    let mut vs = Vec::with_capacity(n);
    for _ in 0..n {
        vs.push(target.add_new_vertex()?);
    }
    if target.vertex_count() != previous + n {
        return invalid(format!(
            "Vertex factory did not produce {n} distinct vertices."
        ));
    }
    Ok(vs)
}

/// Erdős–Rényi `G(n, p)`.
pub struct GnpRandomGraphGenerator<R> {
    n: usize,
    p: f64,
    rng: R,
    create_loops: bool,
}
impl<R: RandomSource> GnpRandomGraphGenerator<R> {
    /// Creates a new instance.
    ///
    /// # Errors
    ///
    /// Returns a [`GraphError`] propagated from the underlying graph operations or from validating the input.
    pub fn new(n: usize, p: f64, rng: R, create_loops: bool) -> Result<Self> {
        Ok(GnpRandomGraphGenerator {
            n,
            p: check_probability(p)?,
            rng,
            create_loops,
        })
    }
}
impl<R: RandomSource> GraphGenerator for GnpRandomGraphGenerator<R> {
    fn generate_graph<G: GraphMut>(
        &mut self,
        target: &mut G,
        _: Option<&mut GeneratorResult<G::Vertex>>,
    ) -> Result<()> {
        if self.n == 0 {
            return Ok(());
        }
        if self.create_loops && !target.graph_type().self_loops {
            return invalid("Provided graph does not support self-loops");
        }
        let vertices = new_vertices(target, self.n)?;
        let directed = target.graph_type().directed;
        for i in 0..self.n {
            for j in i..self.n {
                if i == j && !self.create_loops {
                    continue;
                }
                let mut ends: Option<(usize, usize)> = None;
                if self.rng.next_double() < self.p {
                    ends = Some((i, j));
                    add_edge(target, &vertices[i], &vertices[j])?;
                }
                if directed && self.rng.next_double() < self.p {
                    let (s, t) = ends.unwrap_or((i, j));
                    add_edge(target, &vertices[t], &vertices[s])?;
                }
            }
        }
        Ok(())
    }
}

/// Erdős–Rényi `G(n, m)`.
pub struct GnmRandomGraphGenerator<R> {
    n: usize,
    m: usize,
    rng: R,
    loops: bool,
    multiple_edges: bool,
}
impl<R: RandomSource> GnmRandomGraphGenerator<R> {
    /// Creates a new instance.
    ///
    /// # Errors
    ///
    /// Returns a [`GraphError`] propagated from the underlying graph operations or from validating the input.
    pub fn new(n: usize, m: usize, rng: R, loops: bool, multiple_edges: bool) -> Result<Self> {
        Ok(GnmRandomGraphGenerator {
            n,
            m,
            rng,
            loops,
            multiple_edges,
        })
    }

    /// Largest edge count a graph with `n` vertices of the given kind can have (`i32::MAX` = unbounded).
    pub fn maximum_allowed_edges(
        n: usize,
        directed: bool,
        create_loops: bool,
        create_multiple_edges: bool,
    ) -> usize {
        if n == 0 {
            return 0;
        }
        let unbounded = i32::MAX as usize;
        let n_ = n as i64;
        let mut max = if directed {
            n_.checked_mul(n_ - 1)
        } else if n % 2 == 0 {
            (n_ / 2).checked_mul(n_ - 1)
        } else {
            n_.checked_mul((n_ - 1) / 2)
        };
        let fits = |x: Option<i64>| x.filter(|v| *v <= i32::MAX as i64);
        max = fits(max);
        let Some(mut m) = max else { return unbounded };
        if create_loops {
            if create_multiple_edges {
                return unbounded;
            }
            let extra = if directed { 2 * n_ } else { n_ };
            match fits(m.checked_add(extra)) {
                Some(v) => m = v,
                None => return unbounded,
            }
        } else if create_multiple_edges && n > 1 {
            return unbounded;
        }
        m as usize
    }
}
impl<R: RandomSource> GraphGenerator for GnmRandomGraphGenerator<R> {
    fn generate_graph<G: GraphMut>(
        &mut self,
        target: &mut G,
        _: Option<&mut GeneratorResult<G::Vertex>>,
    ) -> Result<()> {
        if self.n == 0 {
            return Ok(());
        }
        let ty = target.graph_type();
        if self.loops && !ty.self_loops {
            return invalid("Provided graph does not support self-loops");
        }
        if self.multiple_edges && !ty.multiple_edges {
            return invalid(
                "Provided graph does not support multiple edges between the same vertices",
            );
        }
        if self.m
            > Self::maximum_allowed_edges(self.n, ty.directed, self.loops, self.multiple_edges)
        {
            return invalid(format!(
                "number of edges is not valid for the graph type; invalid number of edges={} for graph with {} vertices",
                self.m, self.n
            ));
        }
        let vertices = new_vertices(target, self.n)?;
        let mut count = 0;
        while count < self.m {
            let s_index = self.rng.next_int_bound(self.n);
            let t_index = self.rng.next_int_bound(self.n);
            let add = if s_index == t_index {
                self.loops
            } else if self.multiple_edges {
                true
            } else {
                !target.contains_edge_between(&vertices[s_index], &vertices[t_index])
            };
            if add {
                if let Ok(Some(_)) = target.add_edge(&vertices[s_index], &vertices[t_index]) {
                    count += 1;
                }
            }
        }
        Ok(())
    }
}

/// Barabási–Albert preferential attachment: complete graph on `m0` nodes, then nodes up to `n`,
/// each attached to `m` existing nodes.
pub struct BarabasiAlbertGraphGenerator<R> {
    m0: usize,
    m: usize,
    n: usize,
    rng: R,
}
impl<R: RandomSource> BarabasiAlbertGraphGenerator<R> {
    /// Creates a new instance.
    ///
    /// # Errors
    ///
    /// Returns a [`GraphError`] propagated from the underlying graph operations or from validating the input.
    pub fn new(m0: usize, m: usize, n: usize, rng: R) -> Result<Self> {
        if m0 < 1 {
            return invalid(format!("invalid initial nodes ({m0} < 1)"));
        }
        if m == 0 {
            return invalid("invalid edges per node (0 <= 0");
        }
        if m > m0 {
            return invalid(format!("invalid edges per node ({m} > {m0})"));
        }
        if n < m0 {
            return invalid("total number of nodes must be at least equal to the initial set");
        }
        Ok(BarabasiAlbertGraphGenerator { m0, m, n, rng })
    }
}
impl<R: RandomSource> GraphGenerator for BarabasiAlbertGraphGenerator<R> {
    fn generate_graph<G: GraphMut + IncidenceGraph>(
        &mut self,
        target: &mut G,
        result: Option<&mut GeneratorResult<G::Vertex>>,
    ) -> Result<()> {
        let old: HashSet<G::Vertex> = target.vertices().cloned().collect();
        Complete(self.m0).generate_graph(target, result)?;
        let new_nodes: Vec<G::Vertex> = target
            .vertices()
            .filter(|v| !old.contains(*v))
            .cloned()
            .collect();
        let mut nodes: Vec<G::Vertex> = Vec::with_capacity(self.n * self.m);
        nodes.extend(new_nodes.iter().cloned());
        for _ in 0..self.m0.saturating_sub(2) {
            nodes.extend(new_nodes.iter().cloned());
        }
        for i in self.m0..self.n {
            let v = target.add_new_vertex()?;
            let mut new_endpoints = Vec::new();
            let mut added = 0;
            while added < self.m {
                let u = nodes[self.rng.next_int_bound(nodes.len())].clone();
                if !target.contains_edge_between(&v, &u) {
                    add_edge(target, &v, &u)?;
                    added += 1;
                    new_endpoints.push(v.clone());
                    if i > 1 {
                        new_endpoints.push(u);
                    }
                }
            }
            nodes.extend(new_endpoints);
        }
        Ok(())
    }
}

/// Forest of `t` Barabási–Albert trees on `n` nodes (undirected, empty target).
pub struct BarabasiAlbertForestGenerator<R> {
    t: usize,
    n: usize,
    rng: R,
}
impl<R: RandomSource> BarabasiAlbertForestGenerator<R> {
    /// Creates a new instance.
    ///
    /// # Errors
    ///
    /// Returns a [`GraphError`] propagated from the underlying graph operations or from validating the input.
    pub fn new(t: usize, n: usize, rng: R) -> Result<Self> {
        if t < 1 {
            return invalid(format!("invalid number of trees ({t} < 1)"));
        }
        if n < t {
            return invalid("total number of nodes must be at least equal to the number of trees");
        }
        Ok(BarabasiAlbertForestGenerator { t, n, rng })
    }
}
impl<R: RandomSource> GraphGenerator for BarabasiAlbertForestGenerator<R> {
    fn generate_graph<G: GraphMut>(
        &mut self,
        target: &mut G,
        _: Option<&mut GeneratorResult<G::Vertex>>,
    ) -> Result<()> {
        crate::graph::props::require_undirected(&*target)?;
        if target.vertex_count() != 0 {
            return invalid("target graph is not empty");
        }
        let mut nodes = Vec::new();
        for _ in 0..self.t {
            nodes.push(target.add_new_vertex()?);
        }
        for i in self.t..self.n {
            let v = target.add_new_vertex()?;
            let u = nodes[self.rng.next_int_bound(nodes.len())].clone();
            add_edge(target, &v, &u)?;
            nodes.push(v);
            if i > 1 {
                nodes.push(u);
            }
        }
        Ok(())
    }
}

/// Simple scale-free (preferential attachment) generator that keeps the graph connected.
pub struct ScaleFreeGraphGenerator<R> {
    size: usize,
    rng: R,
}
impl<R: RandomSource> ScaleFreeGraphGenerator<R> {
    /// Creates a new instance.
    ///
    /// # Errors
    ///
    /// Returns a [`GraphError`] propagated from the underlying graph operations or from validating the input.
    pub fn new(size: i64, rng: R) -> Result<Self> {
        if size < 0 {
            return invalid(format!("invalid size: {size} (must be non-negative)"));
        }
        Ok(ScaleFreeGraphGenerator {
            size: size as usize,
            rng,
        })
    }
}
impl<R: RandomSource> GraphGenerator for ScaleFreeGraphGenerator<R> {
    fn generate_graph<G: GraphMut>(
        &mut self,
        target: &mut G,
        _: Option<&mut GeneratorResult<G::Vertex>>,
    ) -> Result<()> {
        let mut vertex_list: Vec<G::Vertex> = Vec::new();
        let mut degrees: Vec<usize> = Vec::new();
        let mut degree_sum = 0usize;
        for i in 0..self.size {
            let new_vertex = target.add_new_vertex()?;
            let mut new_degree = 0;
            while new_degree == 0 && i != 0 {
                for j in 0..vertex_list.len() {
                    if degree_sum == 0 || self.rng.next_int_bound(degree_sum) < degrees[j] {
                        degrees[j] += 1;
                        new_degree += 1;
                        degree_sum += 2;
                        if self.rng.next_boolean() {
                            add_edge(target, &vertex_list[j], &new_vertex)?;
                        } else {
                            add_edge(target, &new_vertex, &vertex_list[j])?;
                        }
                    }
                }
            }
            vertex_list.push(new_vertex);
            degrees.push(new_degree);
        }
        Ok(())
    }
}

/// Bollobás et al. directed scale-free graphs.
pub struct DirectedScaleFreeGraphGenerator<R> {
    alpha: f32,
    alpha_plus_beta: f32,
    delta_in: f32,
    delta_out: f32,
    target_edges: i64,
    target_nodes: i64,
    rng: R,
    max_failures: usize,
    allowing_multiple_edges: bool,
    allowing_self_loops: bool,
}
impl<R: RandomSource> DirectedScaleFreeGraphGenerator<R> {
    /// Creates a new instance.
    ///
    /// # Errors
    ///
    /// Returns a [`GraphError`] propagated from the underlying graph operations or from validating the input.
    pub fn new(
        alpha: f32,
        gamma: f32,
        delta_in: f32,
        delta_out: f32,
        target_edges: i64,
        target_nodes: i64,
        rng: R,
    ) -> Result<Self> {
        if alpha < 0.0 || gamma < 0.0 || alpha + gamma > 1.0 {
            return invalid(format!(
                "alpha and gamma values of ({alpha}, {gamma}) are invalid"
            ));
        }
        if delta_in < 0.0 || delta_out < 0.0 {
            return invalid(format!(
                "deltaIn and deltaOut values of ({delta_in}, {delta_out}) are invalid"
            ));
        }
        if target_edges < 0 && target_nodes < 0 {
            return invalid("can not have both targetEdges and targetNodes not set.");
        }
        Ok(DirectedScaleFreeGraphGenerator {
            alpha,
            alpha_plus_beta: 1.0 - gamma,
            delta_in,
            delta_out,
            target_edges,
            target_nodes,
            rng,
            max_failures: 1000,
            allowing_multiple_edges: true,
            allowing_self_loops: true,
        })
    }
    /// Sets the allowing multiple edges.
    pub fn set_allowing_multiple_edges(&mut self, allow: bool) {
        self.allowing_multiple_edges = allow;
    }
    /// Sets the allowing self loops.
    pub fn set_allowing_self_loops(&mut self, allow: bool) {
        self.allowing_self_loops = allow;
    }
    /// Sets the max failures.
    pub fn set_max_failures(&mut self, max: usize) {
        self.max_failures = max;
    }
}
impl<R: RandomSource> DirectedScaleFreeGraphGenerator<R> {
    fn pick<G: GraphMut + IncidenceGraph>(
        &mut self,
        target: &G,
        nodes: &IndexSet<G::Vertex>,
        edge_count: usize,
        incoming: bool,
        bias: f32,
    ) -> Result<Option<G::Vertex>> {
        match nodes.len() {
            0 => return Ok(None),
            1 => return Ok(nodes.iter().next().cloned()),
            _ => {}
        }
        let denominator = edge_count as f32 + nodes.len() as f32 * bias;
        let mut r = self.rng.next_float();
        r *= denominator;
        let mut acc = 0f32;
        let mut ret = None;
        for v in nodes {
            let degree = if incoming {
                target.in_degree_of(v)
            } else {
                target.out_degree_of(v)
            };
            acc += degree as f32 + bias;
            ret = Some(v.clone());
            if acc >= r {
                break;
            }
        }
        Ok(ret)
    }
}
impl<R: RandomSource> GraphGenerator for DirectedScaleFreeGraphGenerator<R> {
    fn generate_graph<G: GraphMut + IncidenceGraph>(
        &mut self,
        target: &mut G,
        _: Option<&mut GeneratorResult<G::Vertex>>,
    ) -> Result<()> {
        let ty = target.graph_type();
        if self.allowing_multiple_edges && !ty.multiple_edges {
            return invalid(
                "Generator allows Multiple Edges while graph does not. Consider changing this generator parameters or the target graph type.",
            );
        }
        if self.allowing_self_loops && !ty.self_loops {
            return invalid(
                "Generator allows Self loops while graph does not. Consider changing this generator parameters or the target graph type.",
            );
        }
        let mut new_nodes: IndexSet<G::Vertex> = IndexSet::new();
        let mut new_edge_count = 0usize;
        if self.target_edges == 0 || (self.target_edges < 0 && self.target_nodes == 0) {
            return Ok(());
        }
        new_nodes.insert(target.add_new_vertex()?);
        let mut failures = 0usize;
        loop {
            let keep_going = if self.target_edges >= 0 {
                (self.target_edges as usize) > new_edge_count
            } else {
                self.target_nodes >= new_nodes.len() as i64
            };
            if !keep_going {
                break;
            }
            if failures >= self.max_failures {
                return Err(GraphError::InvalidArgument(format!(
                    "{failures} consecutive failures is more than maximum allowed number ({}).",
                    self.max_failures
                )));
            }
            let (mut v, mut w): (Option<G::Vertex>, Option<G::Vertex>) = (None, None);
            let (mut new_v, mut new_w) = (false, false);
            let tributaries = self.rng.next_float();
            let at_node_target =
                self.target_edges < 0 && new_nodes.len() as i64 == self.target_nodes;
            if tributaries <= self.alpha {
                if at_node_target {
                    break;
                }
                new_v = true;
                w = self.pick(target, &new_nodes, new_edge_count, true, self.delta_in)?;
            } else if tributaries <= self.alpha_plus_beta {
                v = self.pick(target, &new_nodes, new_edge_count, false, self.delta_out)?;
                w = self.pick(target, &new_nodes, new_edge_count, true, self.delta_in)?;
            } else {
                if at_node_target {
                    break;
                }
                v = self.pick(target, &new_nodes, new_edge_count, false, self.delta_out)?;
                new_w = true;
            }
            if (new_v && w.is_none()) || (new_w && v.is_none()) {
                failures += 1;
                continue;
            }
            if !self.allowing_self_loops && v == w {
                failures += 1;
                continue;
            }
            if !self.allowing_multiple_edges {
                if let (Some(a), Some(b)) = (&v, &w) {
                    if target.contains_edge_between(a, b) {
                        failures += 1;
                        continue;
                    }
                }
            }
            if new_v {
                v = Some(target.add_new_vertex()?);
            }
            if new_w {
                w = Some(target.add_new_vertex()?);
            }
            let (a, b) = (v.unwrap(), w.unwrap());
            add_edge(target, &a, &b)?;
            failures = 0;
            new_nodes.insert(a);
            new_nodes.insert(b);
            new_edge_count += 1;
        }
        Ok(())
    }
}

/// Watts–Strogatz small-world graph (rewiring, or adding edges with `add_instead_of_rewire`).
pub struct WattsStrogatzGraphGenerator<R> {
    n: usize,
    k: usize,
    p: f64,
    add_instead_of_rewire: bool,
    rng: R,
}
impl<R: RandomSource> WattsStrogatzGraphGenerator<R> {
    /// Creates a new instance.
    ///
    /// # Errors
    ///
    /// Returns a [`GraphError`] propagated from the underlying graph operations or from validating the input.
    pub fn new(n: usize, k: usize, p: f64, add_instead_of_rewire: bool, rng: R) -> Result<Self> {
        if n < 3 {
            return invalid("number of vertices must be at least 3");
        }
        if k < 1 {
            return invalid("number of k-nearest neighbors must be positive");
        }
        if k % 2 == 1 {
            return invalid("number of k-nearest neighbors must be even");
        }
        if k > n - 2 + (n % 2) {
            return invalid("invalid k-nearest neighbors");
        }
        if !(0.0..=1.0).contains(&p) {
            return invalid("invalid probability");
        }
        Ok(WattsStrogatzGraphGenerator {
            n,
            k,
            p,
            add_instead_of_rewire,
            rng,
        })
    }
}
impl<R: RandomSource> GraphGenerator for WattsStrogatzGraphGenerator<R> {
    fn generate_graph<G: GraphMut>(
        &mut self,
        target: &mut G,
        _: Option<&mut GeneratorResult<G::Vertex>>,
    ) -> Result<()> {
        let (n, k) = (self.n, self.k);
        let mut ring = Vec::with_capacity(n);
        let mut adj: Vec<Vec<Option<G::Edge>>> = Vec::with_capacity(n);
        for _ in 0..n {
            ring.push(target.add_new_vertex()?);
            adj.push(Vec::with_capacity(k));
        }
        for i in 0..n {
            for j in 1..=k / 2 {
                let e = add_edge(target, &ring[i], &ring[(i + j) % n])?;
                adj[i].push(e);
            }
        }
        for r in 0..k / 2 {
            for i in 0..n {
                if self.rng.next_double() < self.p {
                    let e = adj[i][r].clone();
                    let other = &ring[self.rng.next_int_bound(n)];
                    if *other != ring[i] && !target.contains_edge_between(&ring[i], other) {
                        if !self.add_instead_of_rewire {
                            if let Some(e) = &e {
                                target.remove_edge(e);
                            }
                        }
                        add_edge(target, &ring[i], &other.clone())?;
                    }
                }
            }
        }
        Ok(())
    }
}

/// Random labelled tree from a (random or given) Prüfer sequence.
pub struct PruferTreeGenerator<R> {
    n: usize,
    rng: Option<R>,
    sequence: Option<Vec<usize>>,
}
impl<R: RandomSource> PruferTreeGenerator<R> {
    /// A random tree with `n` vertices.
    ///
    /// # Errors
    ///
    /// Returns a [`GraphError`] propagated from the underlying graph operations or from validating the input.
    pub fn new(n: usize, rng: R) -> Result<Self> {
        if n == 0 {
            return invalid("n must be greater than 0");
        }
        Ok(PruferTreeGenerator {
            n,
            rng: Some(rng),
            sequence: None,
        })
    }
}
impl PruferTreeGenerator<rand::rngs::StdRng> {
    /// The tree with the given Prüfer sequence (`n = sequence.len() + 2` vertices).
    ///
    /// # Errors
    ///
    /// Returns a [`GraphError`] propagated from the underlying graph operations or from validating the input.
    pub fn from_sequence(sequence: &[usize]) -> Result<Self> {
        let n = sequence.len() + 2;
        if sequence.iter().any(|&x| x >= n) {
            return invalid("invalid pruferSequence");
        }
        Ok(PruferTreeGenerator {
            n,
            rng: None,
            sequence: Some(sequence.to_vec()),
        })
    }
}
impl<R: RandomSource> GraphGenerator for PruferTreeGenerator<R> {
    fn generate_graph<G: GraphMut>(
        &mut self,
        target: &mut G,
        _: Option<&mut GeneratorResult<G::Vertex>>,
    ) -> Result<()> {
        crate::graph::props::require_undirected(&*target)?;
        if target.vertex_count() != 0 {
            return invalid("target graph is not empty");
        }
        let n = self.n;
        let mut vertex_list = Vec::with_capacity(n);
        for _ in 0..n {
            vertex_list.push(target.add_new_vertex()?);
        }
        if n == 1 {
            return Ok(());
        }
        let mut degree = vec![1usize; n];
        let prufer: Vec<usize> = match &self.sequence {
            None => {
                let rng = self.rng.as_mut().expect("random generator");
                let mut seq = vec![0; n - 2];
                for s in seq.iter_mut() {
                    *s = rng.next_int_bound(n);
                    degree[*s] += 1;
                }
                seq
            }
            Some(seq) => {
                for &s in seq {
                    degree[s] += 1;
                }
                seq.clone()
            }
        };
        let mut index = (0..n).find(|&k| degree[k] == 1).expect("a leaf exists");
        let mut x = index;
        let mut orphans: IndexSet<usize> = (0..n).collect();
        for &y in prufer.iter().take(n - 2) {
            orphans.shift_remove(&x);
            add_edge(target, &vertex_list[x], &vertex_list[y])?;
            degree[y] -= 1;
            if y < index && degree[y] == 1 {
                x = y;
            } else {
                for k in index + 1..n {
                    if degree[k] == 1 {
                        index = k;
                        x = k;
                        break;
                    }
                }
            }
        }
        let mut it = orphans.iter();
        let (u, v) = (*it.next().unwrap(), *it.next().unwrap());
        add_edge(target, &vertex_list[u], &vertex_list[v])?;
        Ok(())
    }
}

/// Random `d`-regular graph on `n` vertices (simple targets: pairing model with restarts).
pub struct RandomRegularGraphGenerator<R> {
    n: usize,
    d: usize,
    rng: R,
}
impl<R: RandomSource> RandomRegularGraphGenerator<R> {
    /// Creates a new instance.
    ///
    /// # Errors
    ///
    /// Returns a [`GraphError`] propagated from the underlying graph operations or from validating the input.
    pub fn new(n: usize, d: usize, rng: R) -> Result<Self> {
        if d > n {
            return invalid("degree of nodes must be smaller than or equal to number of nodes");
        }
        if (n * d) % 2 != 0 {
            return invalid("value 'n * d' must be even");
        }
        Ok(RandomRegularGraphGenerator { n, d, rng })
    }

    fn suitable(edges: &HashSet<(usize, usize)>, potential: &HashMap<usize, usize>) -> bool {
        if potential.is_empty() {
            return true;
        }
        let mut keys: Vec<usize> = potential.keys().copied().collect();
        keys.sort_unstable();
        for i in 0..keys.len() {
            for j in 0..i {
                if !edges.contains(&(keys[j], keys[i])) {
                    return true;
                }
            }
        }
        false
    }

    fn simple<G: GraphMut>(&mut self, target: &mut G) -> Result<()> {
        let (n, d) = (self.n, self.d);
        let vertices = new_vertices(target, n)?;
        let mut edges: HashSet<(usize, usize)> = HashSet::new();
        // insertion order of accepted edges (JGraphT iterates a HashSet; the set of edges is what matters)
        let mut order: Vec<(usize, usize)> = Vec::new();
        loop {
            let mut stubs: Vec<usize> = (0..n * d).map(|i| i % n).collect();
            let mut restart = false;
            while !stubs.is_empty() {
                let mut potential: HashMap<usize, usize> = HashMap::new();
                let mut potential_order: Vec<usize> = Vec::new();
                java_shuffle(&mut stubs, &mut self.rng);
                let mut i = 0;
                while i + 1 < stubs.len() {
                    let (mut s1, mut s2) = (stubs[i], stubs[i + 1]);
                    if s1 > s2 {
                        std::mem::swap(&mut s1, &mut s2);
                    }
                    if s1 != s2 && !edges.contains(&(s1, s2)) {
                        edges.insert((s1, s2));
                        order.push((s1, s2));
                    } else {
                        for s in [s1, s2] {
                            if !potential.contains_key(&s) {
                                potential_order.push(s);
                            }
                            *potential.entry(s).or_insert(0) += 1;
                        }
                    }
                    i += 2;
                }
                if !Self::suitable(&edges, &potential) {
                    edges.clear();
                    order.clear();
                    restart = true;
                    break;
                }
                stubs.clear();
                for node in potential_order {
                    for _ in 0..potential[&node] {
                        stubs.push(node);
                    }
                }
            }
            if !restart || !edges.is_empty() {
                if !edges.is_empty() {
                    break;
                }
            }
            if !restart && edges.is_empty() {
                continue;
            }
        }
        for (a, b) in order {
            add_edge(target, &vertices[a], &vertices[b])?;
        }
        Ok(())
    }

    fn non_simple<G: GraphMut>(&mut self, target: &mut G) -> Result<()> {
        let (n, d) = (self.n, self.d);
        let mut vertices = Vec::with_capacity(n * d);
        for _ in 0..n {
            let v = target.add_new_vertex()?;
            for _ in 0..d {
                vertices.push(v.clone());
            }
        }
        java_shuffle(&mut vertices, &mut self.rng);
        for i in 0..(n * d) / 2 {
            add_edge(target, &vertices[2 * i], &vertices[2 * i + 1])?;
        }
        Ok(())
    }
}
impl<R: RandomSource> GraphGenerator for RandomRegularGraphGenerator<R> {
    fn generate_graph<G: GraphMut + IncidenceGraph>(
        &mut self,
        target: &mut G,
        _: Option<&mut GeneratorResult<G::Vertex>>,
    ) -> Result<()> {
        let ty: GraphType = target.graph_type();
        if ty.directed || ty.mixed {
            return invalid("target graph must be undirected");
        }
        if ty.is_simple() {
            if self.n == 0 || self.d == 0 {
                super::structured::Empty(self.n).generate_default(target)
            } else if self.d == self.n {
                invalid("target graph must be simple if 'n == d'")
            } else if self.d == self.n - 1 {
                Complete(self.n).generate_default(target)
            } else {
                self.simple(target)
            }
        } else {
            self.non_simple(target)
        }
    }
}

/// Planted partition model: `l` groups of `k` nodes, intra-group edge probability `p`, inter-group `q`.
pub struct PlantedPartitionGraphGenerator<R> {
    l: usize,
    k: usize,
    p: f64,
    q: f64,
    rng: R,
    self_loops: bool,
    fired: bool,
}
impl<R: RandomSource> PlantedPartitionGraphGenerator<R> {
    /// Creates a new instance.
    ///
    /// # Errors
    ///
    /// Returns a [`GraphError`] propagated from the underlying graph operations or from validating the input.
    pub fn new(l: usize, k: usize, p: f64, q: f64, rng: R, self_loops: bool) -> Result<Self> {
        if !(0.0..=1.0).contains(&p) {
            return invalid("invalid probability p");
        }
        if !(0.0..=1.0).contains(&q) {
            return invalid("invalid probability q");
        }
        Ok(PlantedPartitionGraphGenerator {
            l,
            k,
            p,
            q,
            rng,
            self_loops,
            fired: false,
        })
    }

    /// Generates and returns the planted communities, in group order.
    ///
    /// # Errors
    ///
    /// Returns a [`GraphError`] if an argument violates a precondition; errors of the underlying graph operations are propagated.
    pub fn generate_communities<G: GraphMut>(
        &mut self,
        target: &mut G,
    ) -> Result<Vec<Vec<G::Vertex>>> {
        if self.fired {
            return Err(GraphError::InvalidArgument(
                "generateGraph() can be only called once".into(),
            ));
        }
        self.fired = true;
        let mut communities: Vec<Vec<G::Vertex>> = vec![Vec::new(); self.l];
        if self.l == 0 || self.k == 0 {
            return Ok(communities);
        }
        let n = self.k * self.l;
        let mut vertices = Vec::with_capacity(n);
        for i in 0..n {
            let v = target.add_new_vertex()?;
            vertices.push(v.clone());
            communities[i / self.k].push(v);
        }
        if self.self_loops {
            if !target.graph_type().self_loops {
                return invalid("target graph must allow self-loops");
            }
            for v in &vertices {
                if self.rng.next_double() < self.p {
                    add_edge(target, v, v)?;
                }
            }
        }
        let undirected = !target.graph_type().directed;
        for i in 0..n {
            let li = i / self.k;
            for j in i + 1..n {
                let prob = if li == j / self.k { self.p } else { self.q };
                if self.rng.next_double() < prob {
                    add_edge(target, &vertices[i], &vertices[j])?;
                }
                if !undirected && self.rng.next_double() < prob {
                    add_edge(target, &vertices[j], &vertices[i])?;
                }
            }
        }
        Ok(communities)
    }
}
impl<R: RandomSource> GraphGenerator for PlantedPartitionGraphGenerator<R> {
    fn generate_graph<G: GraphMut>(
        &mut self,
        target: &mut G,
        _: Option<&mut GeneratorResult<G::Vertex>>,
    ) -> Result<()> {
        self.generate_communities(target).map(drop)
    }
}

/// Kleinberg's navigable small-world grid.
pub struct KleinbergSmallWorldGraphGenerator<R> {
    n: usize,
    p: usize,
    q: usize,
    r: usize,
    rng: R,
}
impl<R: RandomSource> KleinbergSmallWorldGraphGenerator<R> {
    /// Creates a new instance.
    ///
    /// # Errors
    ///
    /// Returns a [`GraphError`] propagated from the underlying graph operations or from validating the input.
    pub fn new(n: i64, p: i64, q: i64, r: i64, rng: R) -> Result<Self> {
        if n < 1 {
            return invalid("parameter n must be positive");
        }
        if p < 1 {
            return invalid("parameter p must be positive");
        }
        if p > 2 * n - 2 {
            return invalid("lattice distance too large");
        }
        if q < 0 {
            return invalid("parameter q must be non-negative");
        }
        if r < 0 {
            return invalid("parameter r must be non-negative");
        }
        Ok(KleinbergSmallWorldGraphGenerator {
            n: n as usize,
            p: p as usize,
            q: q as usize,
            r: r as usize,
            rng,
        })
    }
}
impl<R: RandomSource> GraphGenerator for KleinbergSmallWorldGraphGenerator<R> {
    fn generate_graph<G: GraphMut>(
        &mut self,
        target: &mut G,
        _: Option<&mut GeneratorResult<G::Vertex>>,
    ) -> Result<()> {
        let n = self.n;
        if n == 1 {
            target.add_new_vertex()?;
            return Ok(());
        }
        super::require_directed_or_undirected(&target.graph_type())?;
        let directed = target.graph_type().directed;
        let mut nodes = Vec::with_capacity(n * n);
        for _ in 0..n * n {
            nodes.push(target.add_new_vertex()?);
        }
        let p = self.p as i64;
        for i in 0..n as i64 {
            for j in 0..n as i64 {
                let vi = i * n as i64 + j;
                for di in -p..=p {
                    for dj in -p..=p {
                        let t = (i + di) * n as i64 + (j + dj);
                        if t < 0 || t == vi || t >= (n * n) as i64 {
                            continue;
                        }
                        if di.abs() + dj.abs() <= p && (directed || t > i * n as i64 + j) {
                            add_edge(target, &nodes[vi as usize], &nodes[t as usize])?;
                        }
                    }
                }
            }
        }
        let mut weights = vec![0f64; n * n];
        for i in 0..n {
            for j in 0..n {
                let v = &nodes[i * n + j];
                let mut sum = 0f64;
                for oi in 0..n {
                    for oj in 0..n {
                        if oi != i || oj != j {
                            let w = ((i as i64 - oi as i64).abs() + (j as i64 - oj as i64).abs())
                                as f64;
                            let w = w.powf(-(self.r as f64));
                            weights[oi * n + oj] = w;
                            sum += w;
                        }
                    }
                }
                weights[i * n + j] = 0.0;
                for w in weights.iter_mut() {
                    *w /= sum;
                }
                let sampler = AliasMethodSampler::new(&weights)?;
                for _ in 0..self.q {
                    let u = &nodes[sampler.next(&mut self.rng)];
                    if u != v && !target.contains_edge_between(v, u) {
                        add_edge(target, v, &u.clone())?;
                    }
                }
            }
        }
        Ok(())
    }
}

/// Linearized chord diagram (LCD) preferential attachment; needs a target allowing multi-edges
/// and loops.
pub struct LinearizedChordDiagramGraphGenerator<R> {
    n: usize,
    m: usize,
    rng: R,
}
impl<R: RandomSource> LinearizedChordDiagramGraphGenerator<R> {
    /// Creates a new instance.
    ///
    /// # Errors
    ///
    /// Returns a [`GraphError`] propagated from the underlying graph operations or from validating the input.
    pub fn new(n: usize, m: usize, rng: R) -> Result<Self> {
        if n == 0 {
            return invalid("invalid number of nodes: must be positive");
        }
        if m == 0 {
            return invalid("invalid edges per node (0 <= 0");
        }
        Ok(LinearizedChordDiagramGraphGenerator { n, m, rng })
    }
}
impl<R: RandomSource> GraphGenerator for LinearizedChordDiagramGraphGenerator<R> {
    fn generate_graph<G: GraphMut>(
        &mut self,
        target: &mut G,
        _: Option<&mut GeneratorResult<G::Vertex>>,
    ) -> Result<()> {
        let mut nodes: Vec<G::Vertex> = Vec::with_capacity(2 * self.n * self.m);
        for _ in 0..self.n {
            let vt = target.add_new_vertex()?;
            for _ in 0..self.m {
                nodes.push(vt.clone());
                let vs = nodes[self.rng.next_int_bound(nodes.len())].clone();
                if target.add_edge(&vt, &vs)?.is_none() {
                    return invalid("Graph does not permit parallel-edges.");
                }
                nodes.push(vs);
            }
        }
        Ok(())
    }
}

/// Random bipartite graph with `m` edges between two groups of sizes `n1` and `n2`.
pub struct GnmRandomBipartiteGraphGenerator<R> {
    n1: usize,
    n2: usize,
    m: usize,
    rng: R,
}
impl<R: RandomSource> GnmRandomBipartiteGraphGenerator<R> {
    /// Creates a new instance.
    pub fn new(n1: usize, n2: usize, m: usize, rng: R) -> Self {
        GnmRandomBipartiteGraphGenerator { n1, n2, m, rng }
    }
    /// Generates and returns the two partitions `(A, B)` as created (A first).
    ///
    /// # Errors
    ///
    /// Returns a [`GraphError`] propagated from the underlying graph operations or from validating the input.
    pub fn generate_partitions<G: GraphMut>(
        &mut self,
        target: &mut G,
    ) -> Result<(Vec<G::Vertex>, Vec<G::Vertex>)> {
        if self.n1 + self.n2 == 0 {
            return Ok((vec![], vec![]));
        }
        let previous = target.vertex_count();
        let mut a = Vec::with_capacity(self.n1);
        for _ in 0..self.n1 {
            a.push(target.add_new_vertex()?);
        }
        let mut b = Vec::with_capacity(self.n2);
        for _ in 0..self.n2 {
            b.push(target.add_new_vertex()?);
        }
        if target.vertex_count() != previous + self.n1 + self.n2 {
            return invalid(format!(
                "Vertex factory did not produce {} distinct vertices.",
                self.n1 + self.n2
            ));
        }
        let directed = target.graph_type().directed;
        let max = if directed {
            self.n1.saturating_mul(self.n2).saturating_mul(2)
        } else {
            self.n1.saturating_mul(self.n2)
        };
        if self.m > max {
            return invalid(format!(
                "number of edges not valid for bipartite graph with {} and {} vertices",
                self.n1, self.n2
            ));
        }
        let mut count = 0;
        while count < self.m {
            let mut s = a[self.rng.next_int_bound(self.n1)].clone();
            let mut t = b[self.rng.next_int_bound(self.n2)].clone();
            if directed && self.rng.next_boolean() {
                std::mem::swap(&mut s, &mut t);
            }
            if !target.contains_edge_between(&s, &t) {
                if let Ok(Some(_)) = target.add_edge(&s, &t) {
                    count += 1;
                }
            }
        }
        Ok((a, b))
    }
}
impl<R: RandomSource> GraphGenerator for GnmRandomBipartiteGraphGenerator<R> {
    fn generate_graph<G: GraphMut>(
        &mut self,
        target: &mut G,
        _: Option<&mut GeneratorResult<G::Vertex>>,
    ) -> Result<()> {
        self.generate_partitions(target).map(drop)
    }
}

/// Random bipartite graph `G(n1, n2, p)`.
pub struct GnpRandomBipartiteGraphGenerator<R> {
    n1: usize,
    n2: usize,
    p: f64,
    rng: R,
}
impl<R: RandomSource> GnpRandomBipartiteGraphGenerator<R> {
    /// Creates a new instance.
    ///
    /// # Errors
    ///
    /// Returns a [`GraphError`] propagated from the underlying graph operations or from validating the input.
    pub fn new(n1: usize, n2: usize, p: f64, rng: R) -> Result<Self> {
        Ok(GnpRandomBipartiteGraphGenerator {
            n1,
            n2,
            p: check_probability(p)?,
            rng,
        })
    }
    /// Generates partitions.
    ///
    /// # Errors
    ///
    /// Returns a [`GraphError`] propagated from the underlying graph operations or from validating the input.
    pub fn generate_partitions<G: GraphMut>(
        &mut self,
        target: &mut G,
    ) -> Result<(Vec<G::Vertex>, Vec<G::Vertex>)> {
        if self.n1 + self.n2 == 0 {
            return Ok((vec![], vec![]));
        }
        let previous = target.vertex_count();
        let mut a = Vec::with_capacity(self.n1);
        for _ in 0..self.n1 {
            a.push(target.add_new_vertex()?);
        }
        let mut b = Vec::with_capacity(self.n2);
        for _ in 0..self.n2 {
            b.push(target.add_new_vertex()?);
        }
        if target.vertex_count() != previous + self.n1 + self.n2 {
            return invalid(format!(
                "Vertex factory did not produce {} distinct vertices.",
                self.n1 + self.n2
            ));
        }
        let directed = target.graph_type().directed;
        for s in &a {
            for t in &b {
                if self.rng.next_double() < self.p {
                    add_edge(target, s, t)?;
                }
                if directed && self.rng.next_double() < self.p {
                    add_edge(target, t, s)?;
                }
            }
        }
        Ok((a, b))
    }
}
impl<R: RandomSource> GraphGenerator for GnpRandomBipartiteGraphGenerator<R> {
    fn generate_graph<G: GraphMut>(
        &mut self,
        target: &mut G,
        _: Option<&mut GeneratorResult<G::Vertex>>,
    ) -> Result<()> {
        self.generate_partitions(target).map(drop)
    }
}

/// Returns the (smaller, larger) of the two partitions as JGraphT's `getFirstPartition`/`getSecondPartition`.
pub fn ordered_partitions<V>(a: Vec<V>, b: Vec<V>) -> (Vec<V>, Vec<V>) {
    if a.len() <= b.len() { (a, b) } else { (b, a) }
}
