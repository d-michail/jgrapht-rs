//! Densest subgraph (density = edges / vertices, direction and multiplicity ignored).

use super::flow::{MaxFlowAlgorithm, max_flow, minimum_st_cut};
use super::util::Indexed;
use crate::graph::GraphMutExt;
use crate::graph::{Graph, GraphMut};
use crate::{DiGraph, Result};
use std::collections::HashSet;

fn simple_edges<G: Graph>(ix: &Indexed<'_, G>) -> Vec<(usize, usize)> {
    let set: HashSet<(usize, usize)> = ix
        .ends
        .iter()
        .filter(|&&(a, b)| a != b)
        .map(|&(a, b)| (a.min(b), a.max(b)))
        .collect();
    let mut v: Vec<_> = set.into_iter().collect();
    v.sort_unstable();
    v
}

fn density_of(edges: &[(usize, usize)], set: &[bool]) -> f64 {
    let n = set.iter().filter(|&&x| x).count();
    if n == 0 {
        return 0.0;
    }
    edges.iter().filter(|&&(a, b)| set[a] && set[b]).count() as f64 / n as f64
}

/// Charikar's greedy peeling: a 1/2-approximation of the densest subgraph.
pub fn greedy_densest_subgraph<G: Graph>(g: &G) -> (Vec<G::Vertex>, f64) {
    let ix = Indexed::new(g);
    let n = ix.n();
    let edges = simple_edges(&ix);
    let mut adj = vec![HashSet::new(); n];
    for &(a, b) in &edges {
        adj[a].insert(b);
        adj[b].insert(a);
    }
    let mut alive = vec![true; n];
    let (mut m, mut count) = (edges.len(), n);
    let (mut best, mut best_set) = (if n > 0 { m as f64 / n as f64 } else { 0.0 }, alive.clone());
    while count > 1 {
        let v = (0..n)
            .filter(|&v| alive[v])
            .min_by_key(|&v| adj[v].iter().filter(|&&u| alive[u]).count())
            .unwrap();
        m -= adj[v].iter().filter(|&&u| alive[u]).count();
        alive[v] = false;
        count -= 1;
        let d = m as f64 / count as f64;
        if d > best {
            best = d;
            best_set = alive.clone();
        }
    }
    (
        (0..n)
            .filter(|&v| best_set[v])
            .map(|v| ix.vertex(v))
            .collect(),
        best,
    )
}

/// Exact maximum density subgraph (Goldberg's reduction to a sequence of minimum cuts).
pub fn goldberg_densest_subgraph<G: Graph>(g: &G) -> Result<(Vec<G::Vertex>, f64)> {
    let ix = Indexed::new(g);
    let n = ix.n();
    let edges = simple_edges(&ix);
    let m = edges.len() as f64;
    if edges.is_empty() {
        return Ok((ix.vs.iter().take(1).map(|v| (*v).clone()).collect(), 0.0));
    }
    let mut deg = vec![0.0; n];
    for &(a, b) in &edges {
        deg[a] += 1.0;
        deg[b] += 1.0;
    }
    let (mut lo, mut hi) = (0.0f64, m);
    let mut best = vec![true; n];
    let precision = 1.0 / (n as f64 * (n as f64 - 1.0)).max(1.0);
    while hi - lo >= precision {
        let guess = (lo + hi) / 2.0;
        let (s, t) = (n, n + 1);
        let mut net = DiGraph::<usize>::simple().weighted();
        net.add_vertices(0..n + 2);
        for v in 0..n {
            net.add_weighted_edge(&s, &v, m)?;
            net.add_weighted_edge(&v, &t, m + 2.0 * guess - deg[v])?;
        }
        for &(a, b) in &edges {
            net.add_weighted_edge(&a, &b, 1.0)?;
            net.add_weighted_edge(&b, &a, 1.0)?;
        }
        let f = max_flow(&net, &s, &t, MaxFlowAlgorithm::Dinic)?;
        let side: Vec<bool> = {
            let mut x = vec![false; n];
            f.source_side
                .iter()
                .filter(|&&v| v < n)
                .for_each(|&v| x[v] = true);
            x
        };
        if side.iter().any(|&b| b) {
            lo = guess;
            best = side;
        } else {
            hi = guess;
        }
    }
    let d = density_of(&edges, &best);
    Ok((
        (0..n).filter(|&v| best[v]).map(|v| ix.vertex(v)).collect(),
        d,
    ))
}

/// The directed multigraph Goldberg's algorithm builds its flow networks in.
pub type DensityNetwork<V> = crate::BaseGraph<V, crate::DefaultEdge, crate::graph::Directed>;

/// Computes the source side of a minimum `s`-`t` cut of a [`DensityNetwork`] (edge weights are
/// the capacities). This plays the role of JGraphT's `MinimumSTCutAlgorithm` factory argument.
pub type MinCutFn<V> = Box<dyn Fn(&DensityNetwork<V>, &V, &V) -> Result<HashSet<V>>>;

/// The default min-cut used by the Goldberg algorithms (max-flow based).
pub fn default_min_cut<V: Clone + Eq + std::hash::Hash + 'static>() -> MinCutFn<V> {
    Box::new(|net, s, t| {
        // JGraphT's push-relabel never uses an arc with negative capacity (the edge-density
        // variant can produce them), which is the same as capacity 0.
        let negative = net.edges().any(|e| net.edge_weight(e).is_some_and(|w| w < 0.0));
        let cut = if negative {
            let mut clamped = net.clone();
            let edges: Vec<_> = clamped.edges().cloned().collect();
            for e in edges {
                if clamped.edge_weight(&e).is_some_and(|w| w < 0.0) {
                    clamped.set_edge_weight(&e, 0.0)?;
                }
            }
            minimum_st_cut(&clamped, s, t)?
        } else {
            minimum_st_cut(net, s, t)?
        };
        Ok(cut.source_side.into_iter().collect())
    })
}

enum DensityKind<V> {
    /// density = sum of edge weights / number of vertices
    EdgeWeights,
    /// density = (sum of edge weights + sum of vertex weights) / number of vertices
    NodeWeights(Box<dyn Fn(&V) -> f64>),
    /// density = sum of edge weights / sum of vertex weights
    NodeWeightPerEdgeWeight(Box<dyn Fn(&V) -> f64>),
}

/// Goldberg's maximum density subgraph algorithm (`GoldbergMaximumDensitySubgraphAlgorithm` and
/// its node-weight variants): binary search on the density, each step a minimum cut.
///
/// `s` and `t` are two extra vertices (not in the graph) used as source and sink of the network.
pub struct GoldbergMaximumDensitySubgraph<'g, G: Graph>
where
    G::Vertex: 'static,
{
    graph: &'g G,
    kind: DensityKind<G::Vertex>,
    lower: f64,
    upper: f64,
    epsilon: f64,
    guess: f64,
    check_weights: bool,
    network: DensityNetwork<G::Vertex>,
    s: G::Vertex,
    t: G::Vertex,
    min_cut: MinCutFn<G::Vertex>,
    current_vertices: HashSet<G::Vertex>,
    /// `Some` once the densest subgraph is known
    densest: Option<HashSet<G::Vertex>>,
}

impl<'g, G: Graph> GoldbergMaximumDensitySubgraph<'g, G>
where
    G::Vertex: 'static,
{
    /// Edge density: sum of edge weights over the number of vertices.
    pub fn new(graph: &'g G, s: G::Vertex, t: G::Vertex, epsilon: f64) -> Result<Self> {
        Self::build(
            graph,
            s,
            t,
            DensityKind::EdgeWeights,
            false,
            epsilon,
            default_min_cut(),
        )
    }

    /// Edge density plus vertex weights `node_weight`.
    pub fn with_node_weights(
        graph: &'g G,
        s: G::Vertex,
        t: G::Vertex,
        epsilon: f64,
        node_weight: impl Fn(&G::Vertex) -> f64 + 'static,
    ) -> Result<Self> {
        Self::build(
            graph,
            s,
            t,
            DensityKind::NodeWeights(Box::new(node_weight)),
            true,
            epsilon,
            default_min_cut(),
        )
    }

    /// Sum of edge weights per unit of vertex weight.
    pub fn with_node_weight_per_edge_weight(
        graph: &'g G,
        s: G::Vertex,
        t: G::Vertex,
        epsilon: f64,
        node_weight: impl Fn(&G::Vertex) -> f64 + 'static,
    ) -> Result<Self> {
        Self::build(
            graph,
            s,
            t,
            DensityKind::NodeWeightPerEdgeWeight(Box::new(node_weight)),
            true,
            epsilon,
            default_min_cut(),
        )
    }

    /// Replaces the minimum cut routine.
    pub fn with_min_cut(mut self, min_cut: MinCutFn<G::Vertex>) -> Self {
        self.min_cut = min_cut;
        self
    }

    fn build(
        graph: &'g G,
        s: G::Vertex,
        t: G::Vertex,
        kind: DensityKind<G::Vertex>,
        check_weights: bool,
        epsilon: f64,
        min_cut: MinCutFn<G::Vertex>,
    ) -> Result<Self> {
        if graph.contains_vertex(&s) || graph.contains_vertex(&t) {
            return Err(crate::GraphError::InvalidArgument(
                "Source or sink vertex already in graph".into(),
            ));
        }
        let mut alg = GoldbergMaximumDensitySubgraph {
            graph,
            kind,
            lower: 0.0,
            upper: 0.0,
            epsilon,
            guess: 0.0,
            check_weights,
            network: DensityNetwork::<G::Vertex>::pseudo().weighted(),
            s,
            t,
            min_cut,
            current_vertices: HashSet::new(),
            densest: None,
        };
        alg.upper = alg.numerator(graph);
        alg.initialize_network()?;
        // an empty denominator means the whole (empty) graph is the solution
        if alg.denominator(graph) == 0.0 {
            alg.densest = Some(graph.vertices().cloned().collect());
        }
        Ok(alg)
    }

    fn weight_of<H: Graph<Vertex = G::Vertex>>(g: &H, e: &H::Edge) -> f64 {
        g.edge_weight(e).unwrap_or(1.0)
    }

    fn numerator<H: Graph<Vertex = G::Vertex>>(&self, g: &H) -> f64 {
        let edges: f64 = g.edges().map(|e| Self::weight_of(g, e)).sum();
        match &self.kind {
            DensityKind::NodeWeights(w) => edges + g.vertices().map(|v| w(v)).sum::<f64>(),
            _ => edges,
        }
    }

    fn denominator<H: Graph<Vertex = G::Vertex>>(&self, g: &H) -> f64 {
        match &self.kind {
            DensityKind::NodeWeightPerEdgeWeight(w) => g.vertices().map(|v| w(v)).sum(),
            _ => g.vertex_count() as f64,
        }
    }

    fn outgoing_weight(&self, v: &G::Vertex) -> f64 {
        self.graph
            .out_edges_of(v)
            .map(|e| Self::weight_of(self.graph, e))
            .sum()
    }

    fn source_weight(&self, _v: &G::Vertex) -> f64 {
        match &self.kind {
            DensityKind::EdgeWeights => self.graph.edge_count() as f64,
            _ => 0.0,
        }
    }

    fn sink_weight(&self, v: &G::Vertex) -> f64 {
        match &self.kind {
            DensityKind::EdgeWeights => {
                self.graph.edge_count() as f64 + 2.0 * self.guess - self.outgoing_weight(v)
            }
            DensityKind::NodeWeights(w) => 2.0 * self.guess - self.outgoing_weight(v) - 2.0 * w(v),
            DensityKind::NodeWeightPerEdgeWeight(w) => {
                2.0 * self.guess * w(v) - self.outgoing_weight(v)
            }
        }
    }

    fn initialize_network(&mut self) -> Result<()> {
        let (s, t) = (self.s.clone(), self.t.clone());
        self.network.add_vertex(s.clone());
        self.network.add_vertex(t.clone());
        for v in self.graph.vertices() {
            self.network.add_vertex(v.clone());
            self.network.add_edge(&s, v)?;
            self.network.add_edge(v, &t)?;
        }
        for e in self.graph.edges() {
            let (a, b) = (
                self.graph.edge_source(e).unwrap().clone(),
                self.graph.edge_target(e).unwrap().clone(),
            );
            let weight = Self::weight_of(self.graph, e);
            for (x, y) in [(&a, &b), (&b, &a)] {
                if let Some(edge) = self.network.add_edge(x, y)? {
                    self.network.set_edge_weight(&edge, weight)?;
                }
            }
        }
        Ok(())
    }

    fn update_network(&mut self) -> Result<()> {
        let vertices: Vec<G::Vertex> = self.graph.vertices().cloned().collect();
        for v in &vertices {
            let (sw, tw) = (self.source_weight(v), self.sink_weight(v));
            let to_sink = self.network.get_edge(v, &self.t).cloned().unwrap();
            self.network.set_edge_weight(&to_sink, tw)?;
            let from_source = self.network.get_edge(&self.s, v).cloned().unwrap();
            self.network.set_edge_weight(&from_source, sw)?;
        }
        if self.check_weights {
            // all capacities must be non-negative: shift them by the minimum
            let min = vertices
                .iter()
                .flat_map(|v| {
                    [
                        self.network.get_edge(v, &self.t).cloned(),
                        self.network.get_edge(&self.s, v).cloned(),
                    ]
                })
                .flatten()
                .map(|e| self.network.edge_weight(&e).unwrap())
                .fold(None, |m: Option<f64>, w| Some(m.map_or(w, |m| m.min(w))))
                .unwrap_or(0.0);
            if min < 0.0 {
                for v in &vertices {
                    for e in [
                        self.network.get_edge(v, &self.t).cloned().unwrap(),
                        self.network.get_edge(&self.s, v).cloned().unwrap(),
                    ] {
                        let w = self.network.edge_weight(&e).unwrap();
                        self.network.set_edge_weight(&e, w - min)?;
                    }
                }
            }
        }
        Ok(())
    }

    /// The densest subgraph (induced).
    pub fn calculate_densest(&mut self) -> Result<crate::AsSubgraph<&'g G>> {
        if self.densest.is_none() {
            while self.upper - self.lower >= self.epsilon {
                self.guess = self.lower + (self.upper - self.lower) / 2.0;
                self.update_network()?;
                let mut source_partition = (self.min_cut)(&self.network, &self.s, &self.t)?;
                source_partition.remove(&self.s);
                if source_partition.is_empty() {
                    self.upper = self.guess;
                } else {
                    self.lower = self.guess;
                    self.current_vertices = source_partition;
                }
            }
            self.densest = Some(std::mem::take(&mut self.current_vertices));
        }
        Ok(crate::AsSubgraph::induced(
            self.graph,
            self.densest.as_ref().unwrap().iter().cloned(),
        ))
    }

    /// Density of the densest subgraph.
    pub fn density(&mut self) -> Result<f64> {
        let sub = self.calculate_densest()?;
        let denominator = self.denominator(&sub);
        Ok(if denominator != 0.0 {
            self.numerator(&sub) / denominator
        } else {
            0.0
        })
    }
}
