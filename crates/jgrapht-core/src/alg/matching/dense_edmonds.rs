use super::base::{Matching, MatchingAlgorithm};
use super::greedy::GreedyMaximumCardinalityMatching;
use crate::alg::connectivity::ConnectivityInspector;
use crate::alg::util::{FixedSizeIntegerQueue, UnionFind};
use crate::error::Result;
use crate::graph::{Graph, GraphExt, props};
use crate::view::AsSubgraph;
use indexmap::IndexSet;
use std::collections::{HashMap, HashSet};

const NIL: i64 = -1;

/// Even and odd levels of the alternating forest.
struct Levels {
    even: Vec<i64>,
    odd: Vec<i64>,
    dirty: Vec<usize>,
}

impl Levels {
    fn new(n: usize) -> Self {
        Levels {
            even: vec![NIL; n],
            odd: vec![NIL; n],
            dirty: Vec::new(),
        }
    }

    fn get_even(&self, v: usize) -> usize {
        self.even[v] as usize
    }

    fn set_even(&mut self, v: usize, value: usize) {
        self.even[v] = value as i64;
        self.dirty.push(v);
    }

    fn get_odd(&self, v: usize) -> usize {
        self.odd[v] as usize
    }

    fn set_odd(&mut self, v: usize, value: usize) {
        self.odd[v] = value as i64;
        self.dirty.push(v);
    }

    fn is_even(&self, v: usize) -> bool {
        self.even[v] != NIL
    }

    fn is_odd_or_unreached(&self, v: usize) -> bool {
        self.odd[v] == NIL
    }

    fn is_odd(&self, v: usize) -> bool {
        self.odd[v] != NIL
    }

    fn reset(&mut self) {
        for &v in &self.dirty {
            self.even[v] = NIL;
            self.odd[v] = NIL;
        }
        self.dirty.clear();
    }
}

/// A matching on vertex indices.
struct SimpleMatching {
    mate: Vec<i64>,
}

impl SimpleMatching {
    const UNMATCHED: i64 = -1;

    fn new(n: usize) -> Self {
        SimpleMatching {
            mate: vec![Self::UNMATCHED; n],
        }
    }

    fn is_exposed(&self, v: usize) -> bool {
        self.mate[v] == Self::UNMATCHED
    }

    fn opposite(&self, v: usize) -> usize {
        debug_assert!(!self.is_exposed(v));
        self.mate[v] as usize
    }

    fn match_(&mut self, u: usize, v: usize) {
        self.mate[u] = v as i64;
        self.mate[v] = u as i64;
    }

    /// The exposed vertices in increasing order (the iteration order of Java's `HashSet<Integer>`
    /// of small integers).
    fn exposed(&self) -> Vec<usize> {
        (0..self.mate.len()).filter(|&v| self.is_exposed(v)).collect()
    }
}

/// `DenseEdmondsMaximumCardinalityMatching`: Edmonds' blossom algorithm for a maximum cardinality
/// matching in a general graph, in `O(V^3)` time (Gabow's variant with a union-find structure for
/// the blossoms), best for dense graphs.
pub struct DenseEdmondsMaximumCardinalityMatching<'g, G: Graph> {
    graph: &'g G,
    /// (Heuristic) matching algorithm used to compute an initial feasible solution.
    initializer: Option<Box<dyn MatchingAlgorithm<G::Vertex, G::Edge> + 'g>>,
    /// Ordered list of vertices.
    vertices: Vec<G::Vertex>,
    /// Mapping of a vertex to its unique position in the ordered list of vertices.
    vertex_index_map: HashMap<G::Vertex, usize>,
    /// Adjacency (as positions) of every vertex, in the order of the graph.
    neighbors: Vec<Vec<usize>>,
    matching: SimpleMatching,
    matched_vertices: usize,
    levels: Levels,
    queue: FixedSizeIntegerQueue,
    uf: UnionFind<usize>,
    bridges: HashMap<usize, (usize, usize)>,
    path: Vec<usize>,
    /// Pre-allocated sets to track paths in the trees.
    v_ancestors: HashSet<usize>,
    w_ancestors: HashSet<usize>,
}

impl<'g, G: Graph + 'g> DenseEdmondsMaximumCardinalityMatching<'g, G> {
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
        Ok(DenseEdmondsMaximumCardinalityMatching {
            graph,
            initializer,
            vertices: Vec::new(),
            vertex_index_map: HashMap::new(),
            neighbors: Vec::new(),
            matching: SimpleMatching::new(0),
            matched_vertices: 0,
            levels: Levels::new(0),
            queue: FixedSizeIntegerQueue::new(1),
            uf: UnionFind::new(std::iter::empty()),
            bridges: HashMap::new(),
            path: Vec::new(),
            v_ancestors: HashSet::new(),
            w_ancestors: HashSet::new(),
        })
    }

    fn init(&mut self) {
        let graph = self.graph;
        self.vertices = graph.vertices().cloned().collect();
        self.vertex_index_map = self
            .vertices
            .iter()
            .enumerate()
            .map(|(i, v)| (v.clone(), i))
            .collect();
        self.neighbors = self
            .vertices
            .iter()
            .map(|v| {
                graph
                    .neighbor_list_of(v)
                    .iter()
                    .map(|w| self.vertex_index_map[w])
                    .collect()
            })
            .collect();
        let n = self.vertices.len();
        self.matching = SimpleMatching::new(n);
        self.matched_vertices = 0;
        self.levels = Levels::new(n);
        self.queue = FixedSizeIntegerQueue::new(n.max(1));
        self.uf = UnionFind::new(0..n);
        self.bridges.clear();
        self.path = vec![0; n];
        self.v_ancestors.clear();
        self.w_ancestors.clear();
    }

    fn warm_start(&mut self) -> Result<()> {
        let graph = self.graph;
        if let Some(initializer) = self.initializer.as_mut() {
            let initial_solution = initializer.get_matching()?;
            for e in initial_solution.edges() {
                let (u, v) = graph.edge_endpoints(e).expect("edge of the graph");
                self.matching
                    .match_(self.vertex_index_map[u], self.vertex_index_map[v]);
            }
            self.matched_vertices = initial_solution.edges().len() * 2;
        }
        Ok(())
    }

    fn find(&mut self, v: usize) -> usize {
        self.uf.find(&v).expect("vertex index")
    }

    fn union(&mut self, a: usize, b: usize) {
        self.uf.union(&a, &b).expect("vertex indices");
    }

    fn in_same_set(&mut self, a: usize, b: usize) -> bool {
        self.find(a) == self.find(b)
    }

    /// Searches for an augmenting path and augments the matching along it if there is one.
    fn augment(&mut self) -> bool {
        self.levels.reset();
        self.uf.reset();
        self.bridges.clear();
        self.queue.clear();
        let mut exposed: std::collections::VecDeque<usize> = self.matching.exposed().into();
        while let Some(root) = exposed.pop_front() {
            self.levels.set_even(root, root);
            self.queue.enqueue(root);
            while let Some(v) = self.queue.poll() {
                // v is an even vertex
                for i in 0..self.neighbors[v].len() {
                    let w = self.neighbors[v][i];
                    let fw = self.find(w);
                    if self.levels.is_even(fw) {
                        // w is an even vertex
                        if !self.in_same_set(v, w) {
                            self.blossom(v, w); // create a new blossom using bridge edge (v,w)
                        }
                    } else if self.levels.is_odd_or_unreached(w) {
                        // w is odd or unreached
                        if self.matching.is_exposed(w) {
                            // w is unreached: we found an augmenting path
                            self.augment_from(v);
                            self.augment_from(w);
                            self.matching.match_(v, w);
                            return true;
                        }
                        self.levels.set_odd(w, v);
                        let u = self.matching.opposite(w); // even vertex
                        self.levels.set_even(u, w);
                        self.queue.enqueue(u); // continue growing the tree from u
                    }
                }
            }
        }
        false
    }

    fn blossom(&mut self, v: usize, w: usize) {
        let base = self.nearest_common_ancestor(v, w);
        self.blossom_supports(v, w, base);
        self.blossom_supports(w, v, base);
        self.union(v, base);
        self.union(w, base);
        let fb = self.find(base);
        let even = self.levels.get_even(base);
        self.levels.set_even(fb, even);
    }

    fn blossom_supports(&mut self, v: usize, w: usize, base: usize) {
        let bridge = (v, w);
        let mut v = self.find(v);
        let mut u = v;
        while v != base {
            self.union(v, u);
            u = self.levels.get_even(v); // odd vertex
            self.bridges.insert(u, bridge);
            self.queue.enqueue(u);
            self.union(v, u);
            let odd = self.levels.get_odd(u);
            v = self.find(odd); // even vertex
        }
    }

    fn nearest_common_ancestor(&mut self, mut v: usize, mut w: usize) -> usize {
        self.v_ancestors.clear();
        let fv = self.find(v);
        self.v_ancestors.insert(fv);
        self.w_ancestors.clear();
        let fw = self.find(w);
        self.w_ancestors.insert(fw);
        loop {
            v = self.parent(v);
            self.v_ancestors.insert(v);
            w = self.parent(w);
            self.w_ancestors.insert(w);
            if self.w_ancestors.contains(&v) {
                return v;
            } else if self.v_ancestors.contains(&w) {
                return w;
            }
        }
    }

    fn parent(&mut self, v: usize) -> usize {
        let v = self.find(v); // even vertex
        let even = self.levels.get_even(v);
        let parent = self.find(even); // odd vertex, or v if v is the root of its tree
        if parent == v {
            return v; // root of tree
        }
        let odd = self.levels.get_odd(parent);
        self.find(odd)
    }

    fn augment_from(&mut self, v: usize) {
        let mut path = std::mem::take(&mut self.path);
        let n = self.build_path(&mut path, 0, v, None);
        let mut i = 2;
        while i < n {
            self.matching.match_(path[i], path[i - 1]);
            i += 2;
        }
        self.path = path;
    }

    fn build_path(&self, path: &mut [usize], mut i: usize, mut start: usize, end: Option<usize>) -> usize {
        loop {
            while self.levels.is_odd(start) {
                let bridge = self.bridges[&start];
                let j = self.build_path(path, i, bridge.0, Some(start));
                path[i..j].reverse();
                i = j;
                start = bridge.1;
            }
            path[i] = start; // even vertex
            i += 1;
            if self.matching.is_exposed(start) {
                return i;
            }
            path[i] = self.matching.opposite(start); // odd vertex
            i += 1;
            if Some(path[i - 1]) == end {
                return i;
            }
            start = self.levels.get_odd(path[i - 1]); // even vertex
        }
    }

    /// Certifies that `matching` is maximum, via the Tutte–Berge formula.
    pub fn is_maximum_matching(&mut self, matching: &Matching<G::Vertex, G::Edge>) -> Result<bool> {
        let graph = self.graph;
        if matching.edges().len() * 2 + 1 >= graph.vertex_count() {
            return Ok(true);
        }
        self.init(); // reset data structures and use the provided matching as a starting point
        for e in matching.edges() {
            let (u, v) = graph.edge_endpoints(e).expect("edge of the graph");
            self.matching
                .match_(self.vertex_index_map[u], self.vertex_index_map[v]);
        }
        if self.augment() {
            return Ok(false);
        }
        let odd_vertices: HashSet<G::Vertex> = (0..self.vertices.len())
            .filter(|&vx| self.levels.is_odd(vx) && !self.bridges.contains_key(&vx))
            .map(|vx| self.vertices[vx].clone())
            .collect();
        let other_vertices: IndexSet<G::Vertex> = graph
            .vertices()
            .filter(|v| !odd_vertices.contains(*v))
            .cloned()
            .collect();
        let subgraph = AsSubgraph::induced(graph, other_vertices.iter().cloned()); // induced subgraph
        let connected_components = ConnectivityInspector::new().connected_sets(&subgraph);
        let nr_odd_cardinality_components =
            connected_components.iter().filter(|s| s.len() % 2 == 1).count();
        Ok(matching.edges().len() as f64
            == (graph.vertex_count() + odd_vertices.len() - nr_odd_cardinality_components) as f64
                / 2.0)
    }
}

impl<'g, G: Graph + 'g> MatchingAlgorithm<G::Vertex, G::Edge>
    for DenseEdmondsMaximumCardinalityMatching<'g, G>
{
    fn get_matching(&mut self) -> Result<Matching<G::Vertex, G::Edge>> {
        let graph = self.graph;
        self.init();
        self.warm_start()?;

        while self.matched_vertices + 1 < graph.vertex_count() && self.augment() {
            self.matched_vertices += 2;
        }

        let mut edges: IndexSet<G::Edge> = IndexSet::new();
        let mut cost = 0.0;
        for vx in 0..self.vertices.len() {
            if self.matching.is_exposed(vx) {
                continue;
            }
            let v = &self.vertices[vx];
            let w = &self.vertices[self.matching.opposite(vx)];
            let edge = graph.get_edge(v, w).expect("matched vertices are adjacent");
            edges.insert(edge.clone());
            cost += 0.5 * graph.edge_weight(edge).unwrap_or(1.0);
        }
        Ok(Matching::new(graph, edges, cost))
    }
}
