//! Vertex-ordering iterators with listener support: topological order, lexicographic BFS,
//! maximum cardinality search and degeneracy ordering. They always cover the whole graph.
//! Events: `vertex_traversed` fires when the next vertex is determined (`has_next`) and
//! `vertex_finished` when it is handed out (`next`).

use super::events::{TraversalListener, TraversalListeners};
use crate::graph::{DirectedGraph, Graph, GraphExt, UndirectedGraph};
use crate::util::java::JavaPriorityQueue;
use crate::{GraphError, Result};
use indexmap::{IndexMap, IndexSet};
use std::collections::{HashMap, HashSet, VecDeque};

/// Produces the next vertex of an ordering.
pub trait Advance<'g, V> {
    fn advance(&mut self) -> Option<&'g V>;
}

/// Adds one-vertex lookahead and traversal events to an [`Advance`] source.
pub struct Ordered<'g, G: Graph + 'g, A> {
    adv: A,
    cur: Option<&'g G::Vertex>,
    listeners: TraversalListeners<'g, G::Vertex, G::Edge>,
}

impl<'g, G: Graph + 'g, A: Advance<'g, G::Vertex>> Ordered<'g, G, A> {
    fn wrap(adv: A) -> Self {
        Ordered {
            adv,
            cur: None,
            listeners: TraversalListeners::default(),
        }
    }
    pub fn add_traversal_listener(
        &mut self,
        l: impl TraversalListener<G::Vertex, G::Edge> + 'g,
    ) -> usize {
        self.listeners.add(l)
    }
    pub fn remove_traversal_listener(&mut self, id: usize) -> bool {
        self.listeners.remove(id)
    }
    /// These iterators always traverse every component.
    pub fn is_cross_component_traversal(&self) -> bool {
        true
    }
    /// Disabling cross-component traversal is not possible.
    pub fn set_cross_component_traversal(&mut self, cross: bool) -> Result<()> {
        if cross {
            Ok(())
        } else {
            Err(GraphError::InvalidArgument(
                "Iterator is always cross-component".into(),
            ))
        }
    }
    /// The underlying ordering state (e.g. [`TopologicalAdvance::is_cyclic`]).
    pub fn inner(&self) -> &A {
        &self.adv
    }
    pub fn has_next(&mut self) -> bool {
        if self.cur.is_some() {
            return true;
        }
        self.cur = self.adv.advance();
        if let Some(v) = self.cur {
            self.listeners.vertex_traversed(v);
        }
        self.cur.is_some()
    }
}

impl<'g, G: Graph + 'g, A: Advance<'g, G::Vertex>> Iterator for Ordered<'g, G, A> {
    type Item = &'g G::Vertex;
    fn next(&mut self) -> Option<&'g G::Vertex> {
        if !self.has_next() {
            return None;
        }
        let v = self.cur.take();
        if let Some(v) = v {
            self.listeners.vertex_finished(v);
        }
        v
    }
}

// ---------------------------------------------------------------------------- topological

enum Queue<'g, V> {
    Fifo(VecDeque<&'g V>),
    Priority(JavaPriorityQueue<&'g V, Box<dyn Fn(&&'g V, &&'g V) -> i32 + 'g>>),
}

impl<'g, V> Queue<'g, V> {
    fn offer(&mut self, v: &'g V) {
        match self {
            Queue::Fifo(q) => q.push_back(v),
            Queue::Priority(q) => q.offer(v),
        }
    }
    fn poll(&mut self) -> Option<&'g V> {
        match self {
            Queue::Fifo(q) => q.pop_front(),
            Queue::Priority(q) => q.poll(),
        }
    }
}

pub struct TopologicalAdvance<'g, G: Graph + 'g> {
    g: &'g G,
    queue: Queue<'g, G::Vertex>,
    in_degree: HashMap<&'g G::Vertex, usize>,
    remaining: usize,
    cyclic: bool,
}

impl<'g, G: Graph + 'g> TopologicalAdvance<'g, G> {
    /// Set once the iterator got stuck on a cycle (JGraphT throws `NotDirectedAcyclicGraphException`).
    pub fn is_cyclic(&self) -> bool {
        self.cyclic
    }
}

impl<'g, G: Graph + 'g> Advance<'g, G::Vertex> for TopologicalAdvance<'g, G> {
    fn advance(&mut self) -> Option<&'g G::Vertex> {
        let result = self.queue.poll();
        match result {
            Some(v) => {
                let g = self.g;
                for e in g.out_edges_of(v).unwrap() {
                    let Some(other) = g.opposite_vertex(e, v) else {
                        continue;
                    };
                    if let Some(d) = self.in_degree.get_mut(other) {
                        if *d > 0 {
                            *d -= 1;
                            if *d == 0 {
                                self.queue.offer(other);
                            }
                        }
                    }
                }
                self.remaining -= 1;
            }
            None if self.remaining > 0 => self.cyclic = true,
            None => {}
        }
        result
    }
}

/// Topological order of a directed acyclic graph (sources first, FIFO among ready vertices, or
/// by a comparator using `java.util.PriorityQueue` semantics). A cycle ends the iteration early
/// and sets [`TopologicalAdvance::is_cyclic`].
pub type TopologicalOrderIterator<'g, G> = Ordered<'g, G, TopologicalAdvance<'g, G>>;

impl<'g, G: DirectedGraph + 'g> TopologicalOrderIterator<'g, G> {
    pub fn new(g: &'g G) -> Result<Self> {
        Self::build(g, Queue::Fifo(VecDeque::new()))
    }
    /// Ready vertices are taken in `cmp` order (negative = first).
    pub fn with_comparator(
        g: &'g G,
        cmp: impl Fn(&G::Vertex, &G::Vertex) -> i32 + 'g,
    ) -> Result<Self> {
        let boxed: Box<dyn Fn(&&'g G::Vertex, &&'g G::Vertex) -> i32 + 'g> =
            Box::new(move |a, b| cmp(a, b));
        Self::build(g, Queue::Priority(JavaPriorityQueue::new(boxed)))
    }
    fn build(g: &'g G, mut queue: Queue<'g, G::Vertex>) -> Result<Self> {
        let mut in_degree = HashMap::new();
        for v in g.vertices() {
            let mut d = 0;
            for e in g.in_edges_of(v).unwrap() {
                if g.opposite_vertex(e, v) == Some(v) {
                    return Err(GraphError::CycleProhibited);
                }
                d += 1;
            }
            in_degree.insert(v, d);
            if d == 0 {
                queue.offer(v);
            }
        }
        let remaining = g.vertex_count();
        Ok(Ordered::wrap(TopologicalAdvance {
            g,
            queue,
            in_degree,
            remaining,
            cyclic: false,
        }))
    }
}

// ---------------------------------------------------------------------------- lexicographic BFS

struct Bucket<'g, V> {
    next: Option<usize>,
    prev: Option<usize>,
    vertices: IndexSet<&'g V>,
}

pub struct LexAdvance<'g, G: Graph + 'g> {
    g: &'g G,
    buckets: Vec<Bucket<'g, G::Vertex>>,
    head: Option<usize>,
    bucket_of: HashMap<&'g G::Vertex, usize>,
}

impl<'g, G: Graph + 'g> LexAdvance<'g, G> {
    fn poll(&mut self) -> Option<&'g G::Vertex> {
        if self.bucket_of.is_empty() {
            return None;
        }
        let h = self.head?;
        let res = self.buckets[h].vertices.shift_remove_index(0)?;
        self.bucket_of.remove(res);
        if self.buckets[h].vertices.is_empty() {
            self.head = self.buckets[h].next;
            if let Some(n) = self.head {
                self.buckets[n].prev = None;
            }
        }
        Some(res)
    }

    fn update_buckets(&mut self, vertices: Vec<&'g G::Vertex>) {
        let mut visited: HashSet<usize> = HashSet::new();
        for v in vertices {
            let b = self.bucket_of[v];
            if visited.contains(&b) {
                let p = self.buckets[b]
                    .prev
                    .expect("a split bucket has a predecessor");
                self.buckets[p].vertices.insert(v);
                self.bucket_of.insert(v, p);
            } else {
                visited.insert(b);
                let new = self.buckets.len();
                let prev = self.buckets[b].prev;
                self.buckets.push(Bucket {
                    next: Some(b),
                    prev,
                    vertices: IndexSet::from([v]),
                });
                if let Some(p) = prev {
                    self.buckets[p].next = Some(new);
                }
                self.buckets[b].prev = Some(new);
                self.bucket_of.insert(v, new);
                if self.head == Some(b) {
                    self.head = Some(new);
                }
            }
            self.buckets[b].vertices.shift_remove(v);
            if self.buckets[b].vertices.is_empty() {
                visited.remove(&b);
                let (next, prev) = (self.buckets[b].next, self.buckets[b].prev);
                if let Some(n) = next {
                    self.buckets[n].prev = prev;
                }
                if let Some(p) = prev {
                    self.buckets[p].next = next;
                }
            }
        }
    }
}

impl<'g, G: Graph + 'g> Advance<'g, G::Vertex> for LexAdvance<'g, G> {
    fn advance(&mut self) -> Option<&'g G::Vertex> {
        let v = self.poll()?;
        let g = self.g;
        let mut unvisited: IndexSet<&'g G::Vertex> = IndexSet::new();
        for e in g.edges_of(v).unwrap() {
            if let Some(o) = g.opposite_vertex(e, v) {
                if self.bucket_of.contains_key(o) {
                    unvisited.insert(o);
                }
            }
        }
        self.update_buckets(unvisited.into_iter().collect());
        Some(v)
    }
}

/// Lexicographic breadth-first search of an undirected graph.
pub type LexBreadthFirstIterator<'g, G> = Ordered<'g, G, LexAdvance<'g, G>>;

impl<'g, G: UndirectedGraph + 'g> LexBreadthFirstIterator<'g, G> {
    pub fn new(g: &'g G) -> Self {
        let all: IndexSet<&'g G::Vertex> = g.vertices().collect();
        let bucket_of = all.iter().map(|v| (*v, 0)).collect();
        let head = if all.is_empty() { None } else { Some(0) };
        let buckets = vec![Bucket {
            next: None,
            prev: None,
            vertices: all,
        }];
        Ordered::wrap(LexAdvance {
            g,
            buckets,
            head,
            bucket_of,
        })
    }
}

// ---------------------------------------------------------------------------- max cardinality

pub struct MaxCardAdvance<'g, G: Graph + 'g> {
    g: &'g G,
    max_cardinality: isize,
    remaining: usize,
    buckets: Vec<Option<IndexSet<&'g G::Vertex>>>,
    cardinality: HashMap<&'g G::Vertex, usize>,
}

impl<'g, G: Graph + 'g> MaxCardAdvance<'g, G> {
    fn remove_from_bucket(&mut self, v: &'g G::Vertex) -> Option<usize> {
        let c = self.cardinality.remove(v)?;
        let b = self.buckets[c].as_mut().expect("bucket exists");
        b.shift_remove(v);
        if b.is_empty() {
            self.buckets[c] = None;
        }
        Some(c)
    }
    fn add_to_bucket(&mut self, v: &'g G::Vertex, c: usize) {
        self.cardinality.insert(v, c);
        self.buckets[c].get_or_insert_with(IndexSet::new).insert(v);
    }
    fn update_neighbours(&mut self, v: &'g G::Vertex) {
        let g = self.g;
        let mut processed: HashSet<&'g G::Vertex> = HashSet::new();
        for e in g.edges_of(v).unwrap() {
            let Some(o) = g.opposite_vertex(e, v) else {
                continue;
            };
            if self.cardinality.contains_key(o) && processed.insert(o) {
                let c = self.remove_from_bucket(o).expect("present") + 1;
                self.add_to_bucket(o, c);
            }
        }
        let n = g.vertex_count() as isize;
        if self.max_cardinality < n
            && self.max_cardinality >= 0
            && self
                .buckets
                .get((self.max_cardinality + 1) as usize)
                .is_some_and(Option::is_some)
        {
            self.max_cardinality += 1;
        }
    }
}

impl<'g, G: Graph + 'g> Advance<'g, G::Vertex> for MaxCardAdvance<'g, G> {
    fn advance(&mut self) -> Option<&'g G::Vertex> {
        if self.remaining == 0 {
            return None;
        }
        let mc = self.max_cardinality as usize;
        let v = *self.buckets[mc]
            .as_ref()
            .expect("max bucket exists")
            .iter()
            .next()
            .expect("non-empty");
        self.remove_from_bucket(v);
        if self.buckets[mc].is_none() {
            loop {
                self.max_cardinality -= 1;
                if !(self.max_cardinality >= 0
                    && self.buckets[self.max_cardinality as usize].is_none())
                {
                    break;
                }
            }
        }
        self.update_neighbours(v);
        self.remaining -= 1;
        Some(v)
    }
}

/// Maximum cardinality search of an undirected graph: always visit a vertex with the most
/// already-visited neighbours.
pub type MaximumCardinalityIterator<'g, G> = Ordered<'g, G, MaxCardAdvance<'g, G>>;

impl<'g, G: UndirectedGraph + 'g> MaximumCardinalityIterator<'g, G> {
    pub fn new(g: &'g G) -> Self {
        let n = g.vertex_count();
        let mut buckets: Vec<Option<IndexSet<&'g G::Vertex>>> = vec![None; n];
        let mut cardinality = HashMap::new();
        if n > 0 {
            buckets[0] = Some(g.vertices().collect());
            cardinality = g.vertices().map(|v| (v, 0)).collect();
        }
        Ordered::wrap(MaxCardAdvance {
            g,
            max_cardinality: 0,
            remaining: n,
            buckets,
            cardinality,
        })
    }
}

// ---------------------------------------------------------------------------- degeneracy

pub struct DegeneracyAdvance<'g, G: Graph + 'g> {
    g: &'g G,
    buckets: Vec<IndexSet<&'g G::Vertex>>,
    degrees: IndexMap<&'g G::Vertex, usize>,
    min_degree: usize,
}

impl<'g, G: Graph + 'g> Advance<'g, G::Vertex> for DegeneracyAdvance<'g, G> {
    fn advance(&mut self) -> Option<&'g G::Vertex> {
        while self.min_degree < self.buckets.len() && self.buckets[self.min_degree].is_empty() {
            self.min_degree += 1;
        }
        if self.min_degree >= self.buckets.len() {
            return None;
        }
        let v = self.buckets[self.min_degree]
            .shift_remove_index(0)
            .expect("non-empty bucket");
        self.degrees.shift_remove(v);
        let g = self.g;
        for e in g.edges_of(v).unwrap() {
            let Some(u) = g.opposite_vertex(e, v) else {
                continue;
            };
            if u == v {
                continue;
            }
            if let Some(&d) = self.degrees.get(u) {
                if d > self.min_degree {
                    self.buckets[d].shift_remove(u);
                    self.buckets[d - 1].insert(u);
                    self.degrees.insert(u, d - 1);
                }
            }
        }
        Some(v)
    }
}

/// Repeatedly removes a vertex of minimum degree (self-loops ignored, parallel edges counted).
pub type DegeneracyOrderingIterator<'g, G> = Ordered<'g, G, DegeneracyAdvance<'g, G>>;

impl<'g, G: Graph + 'g> DegeneracyOrderingIterator<'g, G> {
    pub fn new(g: &'g G) -> Self {
        let mut degrees: IndexMap<&'g G::Vertex, usize> = IndexMap::new();
        let (mut min_degree, mut max_degree) = (usize::MAX, 0);
        for v in g.vertices() {
            let d = g
                .edges_of(v).unwrap()
                .filter(|e| g.opposite_vertex(e, v) != Some(v))
                .count();
            degrees.insert(v, d);
            min_degree = min_degree.min(d);
            max_degree = max_degree.max(d);
        }
        min_degree = min_degree.min(max_degree);
        let mut buckets: Vec<IndexSet<&'g G::Vertex>> =
            (0..=max_degree).map(|_| IndexSet::new()).collect();
        for v in g.vertices() {
            buckets[degrees[v]].insert(v);
        }
        Ordered::wrap(DegeneracyAdvance {
            g,
            buckets,
            degrees,
            min_degree,
        })
    }
}
