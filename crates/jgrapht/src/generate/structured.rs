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

use super::{GeneratorResult, GraphGenerator, add_edge, put, require_directed_or_undirected};
use crate::graph::Graph;
use crate::graph::GraphExt;
use crate::{GraphError, GraphMut, GraphMutExt, Result};
use std::borrow::Borrow;
use std::collections::HashSet;
use std::hash::Hash;

pub const START_VERTEX: &str = "Start Vertex";
pub const END_VERTEX: &str = "End Vertex";
pub const CENTER_VERTEX: &str = "Center Vertex";
pub const HUB_VERTEX: &str = "Hub Vertex";
pub const CORNER_VERTEX: &str = "Corner Vertex";

/// `size` isolated vertices.
pub struct Empty(pub usize);
impl GraphGenerator for Empty {
    fn generate_graph<G: GraphMut>(
        &mut self,
        target: &mut G,
        _: Option<&mut GeneratorResult<G::Vertex>>,
    ) -> Result<()> {
        for _ in 0..self.0 {
            target.add_new_vertex()?;
        }
        Ok(())
    }
}

/// Path `v0 -> v1 -> ... -> v(n-1)`; the result map holds `"Start Vertex"` and `"End Vertex"`.
pub struct Linear(pub usize);
impl Linear {
    fn run<G: GraphMut>(&self, target: &mut G) -> Result<Option<(G::Vertex, G::Vertex)>> {
        let mut first: Option<G::Vertex> = None;
        let mut last: Option<G::Vertex> = None;
        for _ in 0..self.0 {
            let v = target.add_new_vertex()?;
            match &last {
                None => first = Some(v.clone()),
                Some(prev) => {
                    add_edge(target, prev, &v)?;
                }
            }
            last = Some(v);
        }
        Ok(first.zip(last))
    }
}
impl GraphGenerator for Linear {
    fn generate_graph<G: GraphMut>(
        &mut self,
        target: &mut G,
        mut result: Option<&mut GeneratorResult<G::Vertex>>,
    ) -> Result<()> {
        if let Some((s, e)) = self.run(target)? {
            put(&mut result, START_VERTEX, s);
            put(&mut result, END_VERTEX, e);
        }
        Ok(())
    }
}

/// Cycle `v0 -> v1 -> ... -> v(n-1) -> v0`. (`n == 1` tries to add a self-loop, as in JGraphT.)
pub struct Ring(pub usize);
impl Ring {
    fn run<G: GraphMut>(size: usize, target: &mut G) -> Result<()> {
        if size < 1 {
            return Ok(());
        }
        if let Some((s, e)) = Linear(size).run(target)? {
            add_edge(target, &e, &s)?;
        }
        Ok(())
    }
}
impl GraphGenerator for Ring {
    fn generate_graph<G: GraphMut>(
        &mut self,
        target: &mut G,
        _: Option<&mut GeneratorResult<G::Vertex>>,
    ) -> Result<()> {
        Ring::run(self.0, target)
    }
}

/// Star with `order` vertices: the centre first, then leaves with edges `leaf -> centre`.
pub struct Star(pub usize);
impl GraphGenerator for Star {
    fn generate_graph<G: GraphMut>(
        &mut self,
        target: &mut G,
        mut result: Option<&mut GeneratorResult<G::Vertex>>,
    ) -> Result<()> {
        if self.0 < 1 {
            return Ok(());
        }
        let center = target.add_new_vertex()?;
        put(&mut result, CENTER_VERTEX, center.clone());
        for _ in 0..self.0 - 1 {
            let leaf = target.add_new_vertex()?;
            add_edge(target, &leaf, &center)?;
        }
        Ok(())
    }
}

/// Wheel: a ring of `n - 1` rim vertices (created first) and a hub (`"Hub Vertex"`) created last.
pub struct Wheel {
    /// Number of vertices, hub included.
    pub n: usize,
    /// In a directed graph, point the spokes towards the hub instead of away from it.
    pub inward_spokes: bool,
}
impl GraphGenerator for Wheel {
    fn generate_graph<G: GraphMut>(
        &mut self,
        target: &mut G,
        mut result: Option<&mut GeneratorResult<G::Vertex>>,
    ) -> Result<()> {
        if self.n < 1 {
            return Ok(());
        }
        let before: HashSet<G::Vertex> = target.vertices_cloned().collect();
        Ring::run(self.n - 1, target)?;
        let rim: Vec<G::Vertex> = target
            .vertices()
            .filter(|v| {
                let v = &(*v).borrow();
                !before.contains(*v)
            })
            .map(|x| {
                let x = x.borrow();
                x.clone()
            })
            .collect();
        let hub = target.add_new_vertex()?;
        put(&mut result, HUB_VERTEX, hub.clone());
        for r in &rim {
            if self.inward_spokes {
                add_edge(target, r, &hub)?;
            } else {
                add_edge(target, &hub, r)?;
            }
        }
        Ok(())
    }
}

/// Complete graph on `size` new vertices plus all pre-existing ones (`Complete(0)` = existing only).
/// Directed targets get both orientations of every pair.
pub struct Complete(pub usize);
impl GraphGenerator for Complete {
    fn generate_graph<G: GraphMut>(
        &mut self,
        target: &mut G,
        _: Option<&mut GeneratorResult<G::Vertex>>,
    ) -> Result<()> {
        require_directed_or_undirected(&target.graph_type())?;
        let directed = target.graph_type().directed;
        for _ in 0..self.0 {
            target.add_new_vertex()?;
        }
        let nodes: Vec<G::Vertex> = target.vertices_cloned().collect();
        for i in 0..nodes.len() {
            for j in i + 1..nodes.len() {
                add_edge(target, &nodes[i], &nodes[j])?;
                if directed {
                    add_edge(target, &nodes[j], &nodes[i])?;
                }
            }
        }
        Ok(())
    }
}

/// Complete bipartite graph on two new vertex groups of the given sizes.
pub struct CompleteBipartite(pub usize, pub usize);
impl GraphGenerator for CompleteBipartite {
    fn generate_graph<G: GraphMut>(
        &mut self,
        target: &mut G,
        _: Option<&mut GeneratorResult<G::Vertex>>,
    ) -> Result<()> {
        let mut a = Vec::new();
        for _ in 0..self.0 {
            a.push(target.add_new_vertex()?);
        }
        let mut b = Vec::new();
        for _ in 0..self.1 {
            b.push(target.add_new_vertex()?);
        }
        for u in &a {
            for v in &b {
                add_edge(target, u, v)?;
            }
        }
        Ok(())
    }
}

impl CompleteBipartite {
    /// Connects every vertex of `a` to every vertex of `b` (both non-empty, already in the target).
    ///
    /// # Errors
    ///
    /// Returns [`GraphError::InvalidParameter`] if a parameter is out of range. Errors of the graph
    /// operations it performs are propagated.
    pub fn between<G: GraphMut>(target: &mut G, a: &[G::Vertex], b: &[G::Vertex]) -> Result<()> {
        if a.is_empty() || b.is_empty() {
            return Err(GraphError::InvalidParameter {
                name: "partitions",
                requirement: crate::ParameterRequirement::NonEmpty,
            });
        }
        for u in a {
            for v in b {
                add_edge(target, u, v)?;
            }
        }
        Ok(())
    }
}

/// `rows x cols` grid (both at least 2); `"Corner Vertex 1..4"` in the result map. Every adjacent
/// pair gets edges in both directions (the second is skipped in undirected graphs).
pub struct Grid {
    /// Number of rows of vertices.
    pub rows: usize,
    /// Number of columns of vertices.
    pub cols: usize,
}
impl GraphGenerator for Grid {
    fn generate_graph<G: GraphMut>(
        &mut self,
        target: &mut G,
        mut result: Option<&mut GeneratorResult<G::Vertex>>,
    ) -> Result<()> {
        let (rows, cols) = (self.rows, self.cols);
        if rows < 2 {
            return Err(GraphError::InvalidParameter {
                name: "rows",
                requirement: crate::ParameterRequirement::AtLeast(2),
            });
        }
        if cols < 2 {
            return Err(GraphError::InvalidParameter {
                name: "cols",
                requirement: crate::ParameterRequirement::AtLeast(2),
            });
        }
        let mut vertices = Vec::with_capacity(rows * cols);
        let mut corner = 0;
        for i in 0..rows * cols {
            let v = target.add_new_vertex()?;
            vertices.push(v.clone());
            if i == 0 || i == cols - 1 || i == cols * (rows - 1) || i == rows * cols - 1 {
                corner += 1;
                put(&mut result, format!("{CORNER_VERTEX} {corner}"), v);
            }
        }
        for i in 1..=vertices.len() {
            for j in 1..=vertices.len() {
                if (i % cols > 0 && i + 1 == j) || i + cols == j {
                    add_edge(target, &vertices[i - 1], &vertices[j - 1])?;
                    add_edge(target, &vertices[j - 1], &vertices[i - 1])?;
                }
            }
        }
        Ok(())
    }
}

/// `dim`-dimensional hypercube; result keys are the binary labels of the vertices.
pub struct Hypercube(pub u32);
impl GraphGenerator for Hypercube {
    fn generate_graph<G: GraphMut>(
        &mut self,
        target: &mut G,
        mut result: Option<&mut GeneratorResult<G::Vertex>>,
    ) -> Result<()> {
        let dim = self.0 as usize;
        let order = 1usize << dim;
        let mut vertices = Vec::with_capacity(order);
        for i in 0..order {
            let v = target.add_new_vertex()?;
            vertices.push(v.clone());
            put(&mut result, format!("{i:0dim$b}"), v);
        }
        for i in 0..order {
            for j in i + 1..order {
                if (0..dim).any(|z| (j ^ i) == 1 << z) {
                    add_edge(target, &vertices[i], &vertices[j])?;
                }
            }
        }
        Ok(())
    }
}

/// Generalized Petersen graph `GP(n, k)`; vertices alternate outer (`"regular"`) and inner
/// (`"star"`) in the result map.
pub struct GeneralizedPetersen {
    /// Number of vertices of each of the two rings: the outer cycle and the inner star polygon.
    pub n: usize,
    /// Step of the inner star polygon: inner vertex `i` is joined to inner vertex `i + k`.
    pub k: usize,
}
impl GraphGenerator for GeneralizedPetersen {
    fn generate_graph<G: GraphMut>(
        &mut self,
        target: &mut G,
        result: Option<&mut GeneratorResult<G::Vertex>>,
    ) -> Result<()> {
        let (n, k) = (self.n, self.k);
        if n < 3 {
            return Err(GraphError::InvalidParameter {
                name: "n",
                requirement: crate::ParameterRequirement::AtLeast(3),
            });
        }
        if k < 1 || k as f64 > ((n - 1) as f64 / 2.0).floor() {
            return Err(GraphError::InvalidParameter {
                name: "k",
                requirement: crate::ParameterRequirement::Consistent,
            });
        }
        let (mut u, mut v) = (Vec::with_capacity(n), Vec::with_capacity(n));
        for _ in 0..n {
            u.push(target.add_new_vertex()?);
            v.push(target.add_new_vertex()?);
        }
        for i in 0..n {
            add_edge(target, &u[i], &u[(i + 1) % n])?;
            add_edge(target, &u[i], &v[i])?;
            add_edge(target, &v[i], &v[(i + k) % n])?;
        }
        if let Some(r) = result {
            r.insert("regular".into(), u);
            r.insert("star".into(), v);
        }
        Ok(())
    }
}

#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// Which kind of windmill graph to build.
pub enum WindmillMode {
    /// `m` copies of the complete graph `K_n` sharing one vertex.
    Windmill,
    /// `m` copies of the cycle `C_n` sharing one vertex.
    DutchWindmill,
}

/// Windmill and Dutch windmill graphs; the first vertex is the shared centre.
pub struct Windmill {
    /// Whether the blades are complete graphs or cycles.
    pub mode: WindmillMode,
    /// Number of blades.
    pub m: usize,
    /// Number of vertices of each blade (the shared vertex included).
    pub n: usize,
}
impl GraphGenerator for Windmill {
    fn generate_graph<G: GraphMut>(
        &mut self,
        target: &mut G,
        _: Option<&mut GeneratorResult<G::Vertex>>,
    ) -> Result<()> {
        if self.m < 2 {
            return Err(GraphError::InvalidParameter {
                name: "m",
                requirement: crate::ParameterRequirement::AtLeast(2),
            });
        }
        if self.n < 3 {
            return Err(GraphError::InvalidParameter {
                name: "n",
                requirement: crate::ParameterRequirement::AtLeast(3),
            });
        }
        let center = target.add_new_vertex()?;
        for _ in 0..self.m {
            let mut sub = vec![center.clone()];
            for _ in 1..self.n {
                sub.push(target.add_new_vertex()?);
            }
            match self.mode {
                WindmillMode::DutchWindmill => {
                    for r in 0..sub.len() {
                        add_edge(target, &sub[r], &sub[(r + 1) % self.n])?;
                    }
                }
                WindmillMode::Windmill => {
                    for r in 0..sub.len() - 1 {
                        for s in r + 1..sub.len() {
                            add_edge(target, &sub[r], &sub[s])?;
                        }
                    }
                }
            }
        }
        Ok(())
    }
}

/// Complement of a graph: the same vertices, an edge wherever the source has none.
pub struct Complement<V> {
    vertices: Vec<V>,
    directed: bool,
    /// edge existence by vertex index pair `(i, j)` (both orientations for undirected sources)
    present: HashSet<(usize, usize)>,
    self_loops: bool,
}

impl<V: Clone + Eq + Hash> Complement<V> {
    /// Creates a new instance.
    ///
    /// # Errors
    ///
    /// Returns [`GraphError::EdgeNotFound`] if an edge argument is not in the graph. Errors of the
    /// graph operations it performs are propagated.
    ///
    /// # Panics
    ///
    /// Panics only if the graph breaks the integrity contract of [`Graph`] (an edge without endpoints or weight), or if an internal invariant is violated, which indicates a bug in this library.
    pub fn new<G: Graph<Vertex = V>>(graph: &G, generate_self_loops: bool) -> Result<Self> {
        require_directed_or_undirected(&graph.graph_type())?;
        let vertices: Vec<V> = graph.vertices_cloned().collect();
        let index: std::collections::HashMap<&V, usize> =
            vertices.iter().enumerate().map(|(i, v)| (v, i)).collect();
        let directed = graph.graph_type().directed;
        let mut present = HashSet::new();
        for e in graph.edges() {
            let e = e.borrow();
            let (s, t) = (
                index[graph
                    .edge_source(e)
                    .ok_or(crate::GraphError::EdgeNotFound)?],
                index[graph
                    .edge_target(e)
                    .ok_or(crate::GraphError::EdgeNotFound)?],
            );
            present.insert((s, t));
            if !directed {
                present.insert((t, s));
            }
        }
        Ok(Complement {
            vertices,
            directed,
            present,
            self_loops: generate_self_loops,
        })
    }

    /// Adds the source's vertices and the complement edges to `target`.
    ///
    /// # Errors
    ///
    /// Errors of the graph operations it performs are propagated.
    pub fn generate_into<G: GraphMut<Vertex = V>>(&self, target: &mut G) -> Result<()> {
        target.add_vertices(self.vertices.iter().cloned());
        let n = self.vertices.len();
        if self.directed {
            for u in 0..n {
                for v in 0..n {
                    if u != v && !self.present.contains(&(u, v)) {
                        add_edge(target, &self.vertices[u], &self.vertices[v])?;
                    }
                }
            }
        } else {
            for i in 0..n.saturating_sub(1) {
                for j in i + 1..n {
                    if !self.present.contains(&(i, j)) {
                        add_edge(target, &self.vertices[i], &self.vertices[j])?;
                    }
                }
            }
        }
        if self.self_loops && target.graph_type().self_loops {
            for (i, v) in self.vertices.iter().enumerate() {
                if !self.present.contains(&(i, i)) {
                    add_edge(target, v, v)?;
                }
            }
        }
        Ok(())
    }
}

/// Complete weighted graph from a weight matrix (`SimpleWeightedGraphMatrixGenerator`).
pub struct SimpleWeightedGraphMatrixGenerator<V> {
    vertices: Option<Vec<V>>,
    weights: Option<Vec<Vec<f64>>>,
}

impl<V: Clone + Eq + Hash> Default for SimpleWeightedGraphMatrixGenerator<V> {
    fn default() -> Self {
        SimpleWeightedGraphMatrixGenerator {
            vertices: None,
            weights: None,
        }
    }
}

impl<V: Clone + Eq + Hash> SimpleWeightedGraphMatrixGenerator<V> {
    /// An empty builder; set the vertices and the weights before generating.
    pub fn new() -> Self {
        Self::default()
    }
    /// The vertices of the graph, in the order of the rows of the weight matrix.
    pub fn vertices(mut self, vertices: Vec<V>) -> Self {
        self.vertices = Some(vertices);
        self
    }
    /// Weight matrix: `weights[i][j]` is the weight of the edge between vertex `i` and vertex `j`.
    pub fn weights(mut self, weights: Vec<Vec<f64>>) -> Self {
        self.weights = Some(weights);
        self
    }
    /// Generates into.
    ///
    /// # Errors
    ///
    /// Returns [`GraphError::MissingParameter`] if a required parameter was not supplied. Errors of
    /// the graph operations it performs are propagated.
    pub fn generate_into<G: GraphMut<Vertex = V>>(&self, target: &mut G) -> Result<()> {
        let weights = self
            .weights
            .as_ref()
            .ok_or(GraphError::MissingParameter("weight_matrix"))?;
        let vertices = self
            .vertices
            .as_ref()
            .ok_or(GraphError::MissingParameter("vertex_set"))?;
        for v in vertices {
            target.add_vertex(v.clone());
        }
        for i in 0..vertices.len() {
            for j in 0..vertices.len() {
                if i != j {
                    if let Some(e) = target.add_edge(&vertices[i], &vertices[j])? {
                        target.set_edge_weight(&e, weights[i][j])?;
                    }
                }
            }
        }
        Ok(())
    }
}

/// Complete bipartite weighted graph from a weight matrix.
pub struct SimpleWeightedBipartiteGraphMatrixGenerator<V> {
    first: Option<Vec<V>>,
    second: Option<Vec<V>>,
    weights: Option<Vec<Vec<f64>>>,
}

impl<V: Clone + Eq + Hash> Default for SimpleWeightedBipartiteGraphMatrixGenerator<V> {
    fn default() -> Self {
        SimpleWeightedBipartiteGraphMatrixGenerator {
            first: None,
            second: None,
            weights: None,
        }
    }
}

impl<V: Clone + Eq + Hash> SimpleWeightedBipartiteGraphMatrixGenerator<V> {
    /// An empty builder; set the two vertex sets and the weights before generating.
    pub fn new() -> Self {
        Self::default()
    }
    /// The vertices of the first partition, in the order of the rows of the weight matrix.
    pub fn first(mut self, first: Vec<V>) -> Self {
        self.first = Some(first);
        self
    }
    /// The vertices of the second partition, in the order of the columns of the weight matrix.
    pub fn second(mut self, second: Vec<V>) -> Self {
        self.second = Some(second);
        self
    }
    /// Weight matrix: `weights[i][j]` is the weight of the edge between the `i`-th vertex of the first
    /// partition and the `j`-th vertex of the second.
    pub fn weights(mut self, weights: Vec<Vec<f64>>) -> Self {
        self.weights = Some(weights);
        self
    }
    /// Generates into.
    ///
    /// # Errors
    ///
    /// Returns [`GraphError::MissingParameter`] if a required parameter was not supplied. Errors of
    /// the graph operations it performs are propagated.
    pub fn generate_into<G: GraphMut<Vertex = V>>(&self, target: &mut G) -> Result<()> {
        let weights = self
            .weights
            .as_ref()
            .ok_or(GraphError::MissingParameter("weight_matrix"))?;
        let (Some(first), Some(second)) = (&self.first, &self.second) else {
            return Err(GraphError::MissingParameter("partitions"));
        };
        for v in first.iter().chain(second) {
            target.add_vertex(v.clone());
        }
        for i in 0..first.len() {
            for j in 0..second.len() {
                if let Some(e) = target.add_edge(&first[i], &second[j])? {
                    target.set_edge_weight(&e, weights[i][j])?;
                }
            }
        }
        Ok(())
    }
}
