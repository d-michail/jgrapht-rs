use super::{
    GeneratorResult, GraphGenerator, add_edge, invalid, put, require_directed_or_undirected,
};
use crate::graph::Graph;
use crate::{GraphError, GraphMut, GraphMutExt, Result};
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
    pub n: usize,
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
        let before: HashSet<G::Vertex> = target.vertices().cloned().collect();
        Ring::run(self.n - 1, target)?;
        let rim: Vec<G::Vertex> = target
            .vertices()
            .filter(|v| !before.contains(*v))
            .cloned()
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
        let nodes: Vec<G::Vertex> = target.vertices().cloned().collect();
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
    pub fn between<G: GraphMut>(target: &mut G, a: &[G::Vertex], b: &[G::Vertex]) -> Result<()> {
        if a.is_empty() || b.is_empty() {
            return invalid("partitions must be non-empty");
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
    pub rows: usize,
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
            return invalid(format!(
                "illegal number of rows ({rows}). there must be at least two."
            ));
        }
        if cols < 2 {
            return invalid(format!(
                "illegal number of columns ({cols}). there must be at least two."
            ));
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
    pub n: usize,
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
            return invalid("n must be larger or equal than 3");
        }
        if k < 1 || k as f64 > ((n - 1) as f64 / 2.0).floor() {
            return invalid("k must be in the range [1, floor((n-1)/2.0)]");
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
pub enum WindmillMode {
    /// `m` copies of the complete graph `K_n` sharing one vertex.
    Windmill,
    /// `m` copies of the cycle `C_n` sharing one vertex.
    DutchWindmill,
}

/// Windmill and Dutch windmill graphs; the first vertex is the shared centre.
pub struct Windmill {
    pub mode: WindmillMode,
    pub m: usize,
    pub n: usize,
}
impl GraphGenerator for Windmill {
    fn generate_graph<G: GraphMut>(
        &mut self,
        target: &mut G,
        _: Option<&mut GeneratorResult<G::Vertex>>,
    ) -> Result<()> {
        if self.m < 2 {
            return invalid("m must be larger or equal than 2");
        }
        if self.n < 3 {
            return invalid("n must be larger or equal than 3");
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
    pub fn new<G: Graph<Vertex = V>>(graph: &G, generate_self_loops: bool) -> Result<Self> {
        require_directed_or_undirected(&graph.graph_type())?;
        let vertices: Vec<V> = graph.vertices().cloned().collect();
        let index: std::collections::HashMap<&V, usize> =
            vertices.iter().enumerate().map(|(i, v)| (v, i)).collect();
        let directed = graph.graph_type().directed;
        let mut present = HashSet::new();
        for e in graph.edges() {
            let (s, t) = (
                index[graph.edge_source(e).unwrap()],
                index[graph.edge_target(e).unwrap()],
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
    pub fn new() -> Self {
        Self::default()
    }
    pub fn vertices(mut self, vertices: Vec<V>) -> Self {
        self.vertices = Some(vertices);
        self
    }
    pub fn weights(mut self, weights: Vec<Vec<f64>>) -> Self {
        self.weights = Some(weights);
        self
    }
    pub fn generate_into<G: GraphMut<Vertex = V>>(&self, target: &mut G) -> Result<()> {
        let weights = self.weights.as_ref().ok_or_else(|| {
            GraphError::InvalidArgument(
                "Graph may not be constructed without weight-matrix specified".into(),
            )
        })?;
        let vertices = self.vertices.as_ref().ok_or_else(|| {
            GraphError::InvalidArgument(
                "Graph may not be constructed without vertex-set specified".into(),
            )
        })?;
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
    pub fn new() -> Self {
        Self::default()
    }
    pub fn first(mut self, first: Vec<V>) -> Self {
        self.first = Some(first);
        self
    }
    pub fn second(mut self, second: Vec<V>) -> Self {
        self.second = Some(second);
        self
    }
    pub fn weights(mut self, weights: Vec<Vec<f64>>) -> Self {
        self.weights = Some(weights);
        self
    }
    pub fn generate_into<G: GraphMut<Vertex = V>>(&self, target: &mut G) -> Result<()> {
        let weights = self.weights.as_ref().ok_or_else(|| {
            GraphError::InvalidArgument(
                "Graph may not be constructed without weight-matrix specified".into(),
            )
        })?;
        let (Some(first), Some(second)) = (&self.first, &self.second) else {
            return invalid(
                "Graph may not be constructed without either of vertex-set partitions specified",
            );
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
