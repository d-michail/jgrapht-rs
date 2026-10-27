use super::base::{Matching, MatchingAlgorithm};
use crate::error::{GraphError, Result};
use crate::graph::{Graph, GraphExt, props};
use dashu_float::DBig;
use indexmap::IndexSet;
use rheaps::tree::FibonacciHeap;
use rheaps::AddressableHeap;
use std::collections::{HashMap, HashSet};

type Heap<V> = FibonacciHeap<DBig, V>;
type Handle<V> = <Heap<V> as AddressableHeap<DBig, V>>::Handle;

/// `BigDecimal.valueOf(double)` for edge weights: the decimal digits of the shortest
/// representation, at unlimited precision so that sums and differences are exact.
fn big_decimal(value: f64) -> Result<DBig> {
    if !value.is_finite() {
        return Err(GraphError::InvalidArgument(format!(
            "Infinite or NaN: {value}"
        )));
    }
    Ok(format!("{value}")
        .parse::<DBig>()
        .map_err(|_| GraphError::InvalidArgument(format!("Not a number: {value}")))?
        .with_precision(0)
        .value())
}
fn zero() -> DBig {
    DBig::ZERO.with_precision(0).value()
}

/// `MaximumWeightBipartiteMatching`: a maximum weight matching in a bipartite graph, by
/// successive shortest augmenting paths with vertex potentials (Mehlhorn and Näher, LEDA book,
/// section 7.8). Running time `O(n (n log n + m))` with a Fibonacci heap; all arithmetic is exact
/// (`BigDecimal` in Java, decimal floats of unlimited precision here).
pub struct MaximumWeightBipartiteMatching<'g, G: Graph> {
    graph: &'g G,
    partition1: IndexSet<G::Vertex>,
    partition2: IndexSet<G::Vertex>,
    // vertex potentials
    pot: Option<HashMap<G::Vertex, DBig>>,
    // the matched edge of a vertex, also used to check if a vertex is free
    matched_edge: HashMap<G::Vertex, G::Edge>,
    // shortest path related data structures
    heap: Heap<G::Vertex>,
    node_in_heap: HashMap<G::Vertex, Handle<G::Vertex>>,
    pred: HashMap<G::Vertex, Option<G::Edge>>,
    dist: HashMap<G::Vertex, DBig>,
    // the actual result
    matching: IndexSet<G::Edge>,
    matching_weight: DBig,
}

impl<'g, G: Graph> MaximumWeightBipartiteMatching<'g, G> {
    /// Fails if the graph is not undirected.
    pub fn new(
        graph: &'g G,
        partition1: IndexSet<G::Vertex>,
        partition2: IndexSet<G::Vertex>,
    ) -> Result<Self> {
        props::require_undirected(graph)?;
        Ok(MaximumWeightBipartiteMatching {
            graph,
            partition1,
            partition2,
            pot: None,
            matched_edge: HashMap::new(),
            heap: Heap::new(),
            node_in_heap: HashMap::new(),
            pred: HashMap::new(),
            dist: HashMap::new(),
            matching: IndexSet::new(),
            matching_weight: zero(),
        })
    }

    /// The vertex potentials of the last computation (empty before the first one).
    pub fn get_potentials(&self) -> HashMap<G::Vertex, DBig> {
        self.pot.clone().unwrap_or_default()
    }

    /// The exact weight of the last matching.
    pub fn get_matching_weight(&self) -> &DBig {
        &self.matching_weight
    }

    fn weight(&self, e: &G::Edge) -> Result<DBig> {
        big_decimal(self.graph.edge_weight(e).unwrap_or(1.0))
    }

    fn pot(&self, v: &G::Vertex) -> &DBig {
        &self.pot.as_ref().expect("initialized")[v]
    }

    /// Relaxes all edges out of `a1`.
    fn relax_edges_out_of(
        &mut self,
        a1: &G::Vertex,
        reached_b: &mut Vec<G::Vertex>,
    ) -> Result<()> {
        let graph = self.graph;
        for e1 in graph.edges_of(a1) {
            if !self.matching.contains(e1) {
                let b1 = graph.opposite_vertex(e1, a1).expect("incident edge");
                let db1 = &(&(&self.dist[a1] + self.pot(a1)) + self.pot(b1)) - &self.weight(e1)?;
                if self.pred[b1].is_none() {
                    self.dist.insert(b1.clone(), db1.clone());
                    self.pred.insert(b1.clone(), Some(e1.clone()));
                    reached_b.push(b1.clone());
                    let node = self.heap.insert(db1, b1.clone());
                    self.node_in_heap.insert(b1.clone(), node);
                } else if db1 < self.dist[b1] {
                    self.dist.insert(b1.clone(), db1.clone());
                    self.pred.insert(b1.clone(), Some(e1.clone()));
                    if let Some(handle) = self.node_in_heap.get(b1) {
                        self.heap
                            .decrease_key(*handle, db1)
                            .map_err(|e| GraphError::InvalidArgument(e.to_string()))?;
                    }
                }
            }
        }
        Ok(())
    }

    fn augment(&mut self, a: &G::Vertex) -> Result<()> {
        let graph = self.graph;
        self.dist.insert(a.clone(), zero());
        let mut best_in_a = a.clone();
        let mut min_a = self.pot(a).clone();
        let delta: DBig;
        let mut reached_a: Vec<G::Vertex> = vec![a.clone()]; // used as a stack
        let mut reached_b: Vec<G::Vertex> = Vec::new(); // used as a stack

        // relax all edges out of a
        self.relax_edges_out_of(a, &mut reached_b)?;

        loop {
            // select from border the vertex b with minimum distance db
            let mut b: Option<G::Vertex> = None;
            let mut db = zero();
            if let Some((_, v)) = self.heap.pop() {
                self.node_in_heap.remove(&v);
                db = self.dist[&v].clone();
                b = Some(v);
            }

            match b {
                Some(ref bv) if db < min_a => {
                    match self.matched_edge.get(bv).cloned() {
                        None => {
                            delta = db;
                            self.augment_path_to(bv.clone())?;
                            break;
                        }
                        Some(e) => {
                            let a1 = graph.opposite_vertex(&e, bv).expect("incident edge").clone();
                            self.pred.insert(a1.clone(), Some(e));
                            reached_a.push(a1.clone());
                            self.dist.insert(a1.clone(), db.clone());
                            if &db + self.pot(&a1) < min_a {
                                best_in_a = a1.clone();
                                min_a = &db + self.pot(&a1);
                            }
                            // relax all edges out of a1
                            self.relax_edges_out_of(&a1, &mut reached_b)?;
                        }
                    }
                }
                _ => {
                    delta = min_a;
                    self.augment_path_to(best_in_a)?;
                    break;
                }
            }
        }

        // augment: potential update and re-initialization
        while let Some(v) = reached_a.pop() {
            self.pred.insert(v.clone(), None);
            let pot_change = &delta - &self.dist[&v];
            if pot_change <= zero() {
                continue;
            }
            let new_pot = self.pot(&v) - &pot_change;
            self.pot.as_mut().expect("initialized").insert(v, new_pot);
        }
        while let Some(v) = reached_b.pop() {
            self.pred.insert(v.clone(), None);
            if let Some(handle) = self.node_in_heap.remove(&v) {
                self.heap
                    .delete(handle)
                    .map_err(|e| GraphError::InvalidArgument(e.to_string()))?;
            }
            let pot_change = &delta - &self.dist[&v];
            if pot_change <= zero() {
                continue;
            }
            let new_pot = self.pot(&v) + &pot_change;
            self.pot.as_mut().expect("initialized").insert(v, new_pot);
        }
        Ok(())
    }

    fn augment_path_to(&mut self, mut v: G::Vertex) -> Result<()> {
        let graph = self.graph;
        let mut matched: Vec<G::Edge> = Vec::new();
        let mut free: Vec<G::Edge> = Vec::new();
        let mut e1 = self.pred[&v].clone();
        while let Some(e) = e1 {
            if self.matching.contains(&e) {
                matched.push(e.clone());
            } else {
                free.push(e.clone());
            }
            v = graph.opposite_vertex(&e, &v).expect("incident edge").clone();
            e1 = self.pred[&v].clone();
        }
        for e in matched {
            let w = self.weight(&e)?;
            let (s, t) = graph.edge_endpoints(&e).expect("edge of the graph");
            self.matched_edge.remove(s);
            self.matched_edge.remove(t);
            self.matching_weight = &self.matching_weight - &w;
            self.matching.shift_remove(&e);
        }
        for e in free {
            let w = self.weight(&e)?;
            let (s, t) = graph.edge_endpoints(&e).expect("edge of the graph");
            self.matched_edge.insert(s.clone(), e.clone());
            self.matched_edge.insert(t.clone(), e.clone());
            self.matching_weight = &self.matching_weight + &w;
            self.matching.insert(e);
        }
        Ok(())
    }

    fn simple_heuristic(&mut self) -> Result<()> {
        let graph = self.graph;
        let partition1: Vec<G::Vertex> = self.partition1.iter().cloned().collect();
        for v in &partition1 {
            let mut max_edge: Option<&G::Edge> = None;
            let mut max_weight = zero();
            for e in graph.edges_of(v) {
                let w = self.weight(e)?;
                if w > max_weight {
                    max_weight = w;
                    max_edge = Some(e);
                }
            }
            self.pot
                .as_mut()
                .expect("initialized")
                .insert(v.clone(), max_weight.clone());
            if let Some(max_edge) = max_edge {
                let u = graph.opposite_vertex(max_edge, v).expect("incident edge");
                if !self.matched_edge.contains_key(u) {
                    self.matching.insert(max_edge.clone());
                    self.matching_weight = &self.matching_weight + &max_weight;
                    self.matched_edge.insert(v.clone(), max_edge.clone());
                    self.matched_edge.insert(u.clone(), max_edge.clone());
                }
            }
        }
        Ok(())
    }
}
impl<G: Graph> MatchingAlgorithm<G::Vertex, G::Edge> for MaximumWeightBipartiteMatching<'_, G> {
    fn get_matching(&mut self) -> Result<Matching<G::Vertex, G::Edge>> {
        let graph = self.graph;
        // test input instance
        if !props::is_simple(graph) {
            return Err(GraphError::InvalidArgument(
                "Only simple graphs supported".into(),
            ));
        }
        let p1: HashSet<G::Vertex> = self.partition1.iter().cloned().collect();
        let p2: HashSet<G::Vertex> = self.partition2.iter().cloned().collect();
        if !props::is_bipartite_partition(graph, &p1, &p2) {
            return Err(GraphError::InvalidArgument(
                "Graph partition is not bipartite".into(),
            ));
        }

        // initialize result
        self.matching = IndexSet::new();
        self.matching_weight = zero();

        // empty graph
        if graph.edge_count() == 0 {
            return Ok(Matching::new(graph, self.matching.clone(), 0.0));
        }

        // initialize
        let mut pot = HashMap::new();
        self.dist = HashMap::new();
        self.matched_edge = HashMap::new();
        self.heap = Heap::new();
        self.node_in_heap = HashMap::new();
        self.pred = HashMap::new();
        for v in graph.vertices() {
            pot.insert(v.clone(), zero());
            self.pred.insert(v.clone(), None);
            self.dist.insert(v.clone(), zero());
        }
        self.pot = Some(pot);

        // run simple heuristic
        self.simple_heuristic()?;

        // augment to optimality
        let partition1: Vec<G::Vertex> = self.partition1.iter().cloned().collect();
        for v in &partition1 {
            if !self.matched_edge.contains_key(v) {
                self.augment(v)?;
            }
        }

        let weight = self.matching_weight.to_f64().value();
        Ok(Matching::new(
            graph,
            self.matching.clone(),
            if weight == 0.0 { 0.0 } else { weight },
        ))
    }
}
