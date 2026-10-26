//! The cross-component traversal engine and the BFS / DFS / closest-first strategies on top of
//! it (JGraphT's `CrossComponentIterator` family).

use super::events::{TraversalListener, TraversalListeners};
use crate::graph::{Graph, GraphExt};
use crate::util::OrderedF64;
use crate::util::heap::JavaPairingHeap as PairingHeap;
use crate::{GraphError, Result};
use rheaps::{AddressableHeap, DecreaseKeyHeap};
use std::collections::{HashMap, VecDeque};
use std::vec::IntoIter;

/// State shared between the engine and its strategy.
pub struct Core<'g, G: Graph + 'g, D> {
    pub g: &'g G,
    pub seen: HashMap<&'g G::Vertex, D>,
    pub listeners: TraversalListeners<'g, G::Vertex, G::Edge>,
}

impl<'g, G: Graph + 'g, D> Core<'g, G, D> {
    pub fn finish_vertex(&mut self, v: &'g G::Vertex) {
        self.listeners.vertex_finished(v);
    }
}

/// How a concrete traversal orders vertices (the abstract methods of `CrossComponentIterator`).
pub trait Strategy<'g, G: Graph + 'g>: Sized {
    type Data;
    /// True if the current component has nothing left to visit (may fire finish events).
    fn is_exhausted(&mut self, c: &mut Core<'g, G, Self::Data>) -> bool;
    fn encounter(
        &mut self,
        c: &mut Core<'g, G, Self::Data>,
        v: &'g G::Vertex,
        e: Option<&'g G::Edge>,
    );
    fn encounter_again(
        &mut self,
        c: &mut Core<'g, G, Self::Data>,
        v: &'g G::Vertex,
        e: &'g G::Edge,
    );
    fn provide_next(&mut self, c: &mut Core<'g, G, Self::Data>) -> &'g G::Vertex;
}

type EdgeSelector<'g, G> =
    Box<dyn Fn(&'g G, &'g <G as Graph>::Vertex) -> Vec<&'g <G as Graph>::Edge> + 'g>;

#[derive(Clone, Copy, PartialEq, Eq)]
enum State {
    BeforeComponent,
    WithinComponent,
    AfterComponent,
}

/// Traverses a graph component by component. With cross-component traversal on (the default when
/// no start vertex is given) it restarts at every unseen vertex of the graph; otherwise only at
/// the given start vertices.
pub struct CrossComponentIterator<'g, G: Graph + 'g, S: Strategy<'g, G>> {
    core: Core<'g, G, S::Data>,
    strategy: S,
    cross: bool,
    entire: Option<IntoIter<&'g G::Vertex>>,
    starts: Option<IntoIter<&'g G::Vertex>>,
    start_vertex: Option<&'g G::Vertex>,
    state: State,
    selector: Option<EdgeSelector<'g, G>>,
}

impl<'g, G: Graph + 'g, S: Strategy<'g, G>> CrossComponentIterator<'g, G, S> {
    /// `starts == None` means "whole graph" (cross-component traversal).
    pub fn with_strategy(
        g: &'g G,
        starts: Option<Vec<&'g G::Vertex>>,
        strategy: S,
    ) -> Result<Self> {
        let mut it = CrossComponentIterator {
            core: Core {
                g,
                seen: HashMap::new(),
                listeners: TraversalListeners::default(),
            },
            strategy,
            cross: starts.is_none(),
            entire: None,
            starts: None,
            start_vertex: None,
            state: State::BeforeComponent,
            selector: None,
        };
        let first = match starts {
            None => it.entire_iterator().next(),
            Some(vs) => {
                let mut iter = vs.into_iter();
                let first = iter.next();
                it.starts = Some(iter);
                first
            }
        };
        if let Some(v) = first {
            if !g.contains_vertex(v) {
                return Err(GraphError::InvalidArgument(
                    "graph must contain the start vertex".into(),
                ));
            }
            // resolve to the graph's own reference so lifetimes tie to the graph
            it.start_vertex = Some(v);
        }
        Ok(it)
    }

    fn entire_iterator(&mut self) -> &mut IntoIter<&'g G::Vertex> {
        let g = self.core.g;
        self.entire
            .get_or_insert_with(|| g.vertices().collect::<Vec<_>>().into_iter())
    }

    pub fn graph(&self) -> &'g G {
        self.core.g
    }
    pub fn is_cross_component_traversal(&self) -> bool {
        self.cross
    }
    pub fn set_cross_component_traversal(&mut self, cross: bool) {
        self.cross = cross;
    }
    pub fn strategy(&self) -> &S {
        &self.strategy
    }
    pub fn strategy_mut(&mut self) -> &mut S {
        &mut self.strategy
    }
    pub fn seen_data(&self, v: &G::Vertex) -> Option<&S::Data> {
        self.core.seen.get(v)
    }
    pub fn add_traversal_listener(
        &mut self,
        l: impl TraversalListener<G::Vertex, G::Edge> + 'g,
    ) -> usize {
        self.core.listeners.add(l)
    }
    pub fn remove_traversal_listener(&mut self, id: usize) -> bool {
        self.core.listeners.remove(id)
    }
    /// Replaces the choice of edges followed from a vertex (default: its outgoing edges).
    pub fn set_edge_selector(&mut self, f: impl Fn(&'g G, &'g G::Vertex) -> Vec<&'g G::Edge> + 'g) {
        self.selector = Some(Box::new(f));
    }

    fn encounter_start_vertex(&mut self) {
        if let Some(v) = self.start_vertex.take() {
            self.strategy.encounter(&mut self.core, v, None);
        }
    }

    /// JGraphT's `hasNext()`; it may advance to the next component.
    pub fn has_next(&mut self) -> bool {
        self.encounter_start_vertex();
        if self.strategy.is_exhausted(&mut self.core) {
            if self.state == State::WithinComponent {
                self.state = State::AfterComponent;
                self.core.listeners.component_finished();
            }
            loop {
                let next = if self.cross {
                    self.entire_iterator().next()
                } else {
                    self.starts.as_mut().and_then(Iterator::next)
                };
                let Some(v) = next else { return false };
                if !self.core.g.contains_vertex(v) {
                    continue;
                }
                if !self.core.seen.contains_key(v) {
                    self.strategy.encounter(&mut self.core, v, None);
                    self.state = State::BeforeComponent;
                    return true;
                }
            }
        }
        true
    }

    fn add_unseen_children_of(&mut self, v: &'g G::Vertex) {
        let g = self.core.g;
        let edges: Vec<&'g G::Edge> = match &self.selector {
            Some(f) => f(g, v),
            None => g.out_edges_of(v).collect(),
        };
        for e in edges {
            self.core.listeners.edge_traversed(e);
            let Some(opposite) = g.opposite_vertex(e, v) else {
                continue;
            };
            if self.core.seen.contains_key(opposite) {
                self.strategy.encounter_again(&mut self.core, opposite, e);
            } else {
                self.strategy.encounter(&mut self.core, opposite, Some(e));
            }
        }
    }
}

impl<'g, G: Graph + 'g, S: Strategy<'g, G>> Iterator for CrossComponentIterator<'g, G, S> {
    type Item = &'g G::Vertex;
    fn next(&mut self) -> Option<&'g G::Vertex> {
        self.encounter_start_vertex();
        if !self.has_next() {
            return None;
        }
        if self.state == State::BeforeComponent {
            self.state = State::WithinComponent;
            self.core.listeners.component_started();
        }
        let v = self.strategy.provide_next(&mut self.core);
        self.core.listeners.vertex_traversed(v);
        self.add_unseen_children_of(v);
        Some(v)
    }
}

// ---------------------------------------------------------------------------- breadth-first

/// Per-vertex BFS bookkeeping: the spanning-tree edge and the depth.
#[derive(Debug, Clone, Copy)]
pub struct BfsData<'g, E> {
    pub edge: Option<&'g E>,
    pub depth: usize,
}

pub struct BfsStrategy<'g, G: Graph + 'g> {
    queue: VecDeque<&'g G::Vertex>,
}

impl<'g, G: Graph + 'g> Strategy<'g, G> for BfsStrategy<'g, G> {
    type Data = BfsData<'g, G::Edge>;
    fn is_exhausted(&mut self, _: &mut Core<'g, G, Self::Data>) -> bool {
        self.queue.is_empty()
    }
    fn encounter(
        &mut self,
        c: &mut Core<'g, G, Self::Data>,
        v: &'g G::Vertex,
        e: Option<&'g G::Edge>,
    ) {
        let depth = match e {
            None => 0,
            Some(e) => c.seen[c.g.opposite_vertex(e, v).expect("edge touches vertex")].depth + 1,
        };
        c.seen.insert(v, BfsData { edge: e, depth });
        self.queue.push_back(v);
    }
    fn encounter_again(
        &mut self,
        _: &mut Core<'g, G, Self::Data>,
        _: &'g G::Vertex,
        _: &'g G::Edge,
    ) {
    }
    fn provide_next(&mut self, _: &mut Core<'g, G, Self::Data>) -> &'g G::Vertex {
        self.queue.pop_front().expect("non-empty queue")
    }
}

pub type BreadthFirstIterator<'g, G> = CrossComponentIterator<'g, G, BfsStrategy<'g, G>>;

impl<'g, G: Graph + 'g> BreadthFirstIterator<'g, G> {
    /// Visits the whole graph, component by component.
    pub fn new(g: &'g G) -> Self {
        Self::with_strategy(
            g,
            None,
            BfsStrategy {
                queue: VecDeque::new(),
            },
        )
        .expect("no start vertex to validate")
    }
    /// Starts at `start`; call `set_cross_component_traversal(true)` to continue in other components.
    pub fn from_start(g: &'g G, start: &'g G::Vertex) -> Result<Self> {
        Self::from_starts(g, [start])
    }
    pub fn from_starts(g: &'g G, starts: impl IntoIterator<Item = &'g G::Vertex>) -> Result<Self> {
        Self::with_strategy(
            g,
            Some(starts.into_iter().collect()),
            BfsStrategy {
                queue: VecDeque::new(),
            },
        )
    }
    /// Parent of `v` in the BFS tree (`None` for roots and unseen vertices).
    pub fn parent(&self, v: &G::Vertex) -> Option<&'g G::Vertex> {
        let e = self.seen_data(v)?.edge?;
        self.graph().opposite_vertex(e, v)
    }
    pub fn spanning_tree_edge(&self, v: &G::Vertex) -> Option<&'g G::Edge> {
        self.seen_data(v)?.edge
    }
    pub fn depth(&self, v: &G::Vertex) -> Option<usize> {
        self.seen_data(v).map(|d| d.depth)
    }
}

// ---------------------------------------------------------------------------- depth-first

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum VisitColor {
    White,
    Gray,
    Black,
}

enum StackEntry<'g, V> {
    Vertex(&'g V),
    Sentinel,
}

pub struct DfsStrategy<'g, G: Graph + 'g> {
    stack: VecDeque<StackEntry<'g, G::Vertex>>,
}

impl<'g, G: Graph + 'g> DfsStrategy<'g, G> {
    fn record_finish(&mut self, c: &mut Core<'g, G, VisitColor>) {
        if let Some(StackEntry::Vertex(v)) = self.stack.pop_back() {
            c.seen.insert(v, VisitColor::Black);
            c.finish_vertex(v);
        }
    }
}

impl<'g, G: Graph + 'g> Strategy<'g, G> for DfsStrategy<'g, G> {
    type Data = VisitColor;
    fn is_exhausted(&mut self, c: &mut Core<'g, G, VisitColor>) -> bool {
        loop {
            match self.stack.back() {
                None => return true,
                Some(StackEntry::Vertex(_)) => return false,
                Some(StackEntry::Sentinel) => {
                    self.stack.pop_back();
                    self.record_finish(c);
                }
            }
        }
    }
    fn encounter(
        &mut self,
        c: &mut Core<'g, G, VisitColor>,
        v: &'g G::Vertex,
        _: Option<&'g G::Edge>,
    ) {
        c.seen.insert(v, VisitColor::White);
        self.stack.push_back(StackEntry::Vertex(v));
    }
    fn encounter_again(
        &mut self,
        c: &mut Core<'g, G, VisitColor>,
        v: &'g G::Vertex,
        _: &'g G::Edge,
    ) {
        if c.seen[v] != VisitColor::White {
            return;
        }
        // move the pending occurrence to the top of the stack
        if let Some(pos) = self
            .stack
            .iter()
            .rposition(|e| matches!(e, StackEntry::Vertex(x) if *x == v))
        {
            self.stack.remove(pos);
        }
        self.stack.push_back(StackEntry::Vertex(v));
    }
    fn provide_next(&mut self, c: &mut Core<'g, G, VisitColor>) -> &'g G::Vertex {
        let v = loop {
            match self.stack.pop_back().expect("non-empty stack") {
                StackEntry::Sentinel => self.record_finish(c),
                StackEntry::Vertex(v) => break v,
            }
        };
        self.stack.push_back(StackEntry::Vertex(v));
        self.stack.push_back(StackEntry::Sentinel);
        c.seen.insert(v, VisitColor::Gray);
        v
    }
}

pub type DepthFirstIterator<'g, G> = CrossComponentIterator<'g, G, DfsStrategy<'g, G>>;

impl<'g, G: Graph + 'g> DepthFirstIterator<'g, G> {
    pub fn new(g: &'g G) -> Self {
        Self::with_strategy(
            g,
            None,
            DfsStrategy {
                stack: VecDeque::new(),
            },
        )
        .expect("no start vertex to validate")
    }
    pub fn from_start(g: &'g G, start: &'g G::Vertex) -> Result<Self> {
        Self::from_starts(g, [start])
    }
    pub fn from_starts(g: &'g G, starts: impl IntoIterator<Item = &'g G::Vertex>) -> Result<Self> {
        Self::with_strategy(
            g,
            Some(starts.into_iter().collect()),
            DfsStrategy {
                stack: VecDeque::new(),
            },
        )
    }
    pub fn color(&self, v: &G::Vertex) -> Option<VisitColor> {
        self.seen_data(v).copied()
    }
}

// ---------------------------------------------------------------------------- closest-first

pub struct CfData<'g, G: Graph + 'g, Hd> {
    handle: Option<Hd>,
    key: f64,
    edge: Option<&'g G::Edge>,
    frozen: bool,
}

pub struct ClosestFirstStrategy<'g, G: Graph + 'g, H> {
    heap: H,
    radius: f64,
    _g: std::marker::PhantomData<&'g G>,
}

impl<'g, G, H> ClosestFirstStrategy<'g, G, H>
where
    G: Graph + 'g,
    H: AddressableHeap<OrderedF64, &'g G::Vertex>,
{
    fn path_length(
        c: &Core<'g, G, CfData<'g, G, H::Handle>>,
        v: &'g G::Vertex,
        e: &'g G::Edge,
    ) -> f64 {
        let w = c.g.edge_weight(e).unwrap_or(1.0);
        assert!(w >= 0.0, "negative edge weights not allowed");
        let other = c.g.opposite_vertex(e, v).expect("edge touches vertex");
        c.seen[other].key + w
    }
}

impl<'g, G, H> Strategy<'g, G> for ClosestFirstStrategy<'g, G, H>
where
    G: Graph + 'g,
    H: DecreaseKeyHeap<OrderedF64, &'g G::Vertex>,
{
    type Data = CfData<'g, G, H::Handle>;
    fn is_exhausted(&mut self, _: &mut Core<'g, G, Self::Data>) -> bool {
        if self.heap.is_empty() {
            return true;
        }
        let min = self.heap.peek().map(|(_, k, _)| k.0).unwrap();
        if min > self.radius {
            self.heap.clear();
            return true;
        }
        false
    }
    fn encounter(
        &mut self,
        c: &mut Core<'g, G, Self::Data>,
        v: &'g G::Vertex,
        e: Option<&'g G::Edge>,
    ) {
        let len = match e {
            None => 0.0,
            Some(e) => Self::path_length(c, v, e),
        };
        let handle = self.heap.insert(OrderedF64(len), v);
        c.seen.insert(
            v,
            CfData {
                handle: Some(handle),
                key: len,
                edge: e,
                frozen: false,
            },
        );
    }
    fn encounter_again(
        &mut self,
        c: &mut Core<'g, G, Self::Data>,
        v: &'g G::Vertex,
        e: &'g G::Edge,
    ) {
        if c.seen[v].frozen {
            return;
        }
        let cand = Self::path_length(c, v, e);
        let d = c.seen.get_mut(v).expect("seen");
        if cand < d.key {
            d.edge = Some(e);
            d.key = cand;
            self.heap
                .decrease_key(d.handle.expect("live handle"), OrderedF64(cand))
                .expect("smaller key");
        }
    }
    fn provide_next(&mut self, c: &mut Core<'g, G, Self::Data>) -> &'g G::Vertex {
        let (_, v) = self.heap.pop().expect("non-empty heap");
        let d = c.seen.get_mut(v).expect("seen");
        d.frozen = true;
        d.handle = None;
        v
    }
}

pub type ClosestFirstIterator<'g, G, H = PairingHeap<OrderedF64, &'g <G as Graph>::Vertex>> =
    CrossComponentIterator<'g, G, ClosestFirstStrategy<'g, G, H>>;

impl<'g, G: Graph + 'g> ClosestFirstIterator<'g, G> {
    pub fn new(g: &'g G) -> Self {
        Self::build(g, None, f64::INFINITY, PairingHeap::new())
            .expect("no radius with cross-component traversal")
    }
    pub fn from_start(g: &'g G, start: &'g G::Vertex) -> Result<Self> {
        Self::build(g, Some(vec![start]), f64::INFINITY, PairingHeap::new())
    }
    pub fn from_start_with_radius(g: &'g G, start: &'g G::Vertex, radius: f64) -> Result<Self> {
        Self::build(g, Some(vec![start]), radius, PairingHeap::new())
    }
    pub fn from_starts(g: &'g G, starts: impl IntoIterator<Item = &'g G::Vertex>) -> Result<Self> {
        Self::build(
            g,
            Some(starts.into_iter().collect()),
            f64::INFINITY,
            PairingHeap::new(),
        )
    }
    pub fn from_starts_with_radius(
        g: &'g G,
        starts: impl IntoIterator<Item = &'g G::Vertex>,
        radius: f64,
    ) -> Result<Self> {
        Self::build(
            g,
            Some(starts.into_iter().collect()),
            radius,
            PairingHeap::new(),
        )
    }
}

impl<'g, G, H> ClosestFirstIterator<'g, G, H>
where
    G: Graph + 'g,
    H: DecreaseKeyHeap<OrderedF64, &'g G::Vertex>,
{
    /// Full constructor with an explicit heap. A finite radius is incompatible with
    /// cross-component traversal (`starts == None`).
    pub fn build(
        g: &'g G,
        starts: Option<Vec<&'g G::Vertex>>,
        radius: f64,
        heap: H,
    ) -> Result<Self> {
        if starts.is_none() && radius != f64::INFINITY {
            return Err(GraphError::InvalidArgument(
                "radius may not be specified for cross-component traversal".into(),
            ));
        }
        let rest: Vec<&'g G::Vertex> = starts
            .as_ref()
            .map(|s| s.iter().skip(1).copied().collect())
            .unwrap_or_default();
        let multi = starts.is_some();
        let strategy = ClosestFirstStrategy {
            heap,
            radius,
            _g: std::marker::PhantomData,
        };
        let mut it = CrossComponentIterator::with_strategy(g, starts, strategy)?;
        if multi {
            it.has_next();
            for v in rest {
                let (strategy, core) = (&mut it.strategy, &mut it.core);
                strategy.encounter(core, v, None);
            }
        }
        Ok(it)
    }

    /// Distance of `v` from the closest start vertex (`INFINITY` if unseen).
    pub fn shortest_path_length(&self, v: &G::Vertex) -> f64 {
        self.seen_data(v).map_or(f64::INFINITY, |d| d.key)
    }
    pub fn spanning_tree_edge(&self, v: &G::Vertex) -> Option<&'g G::Edge> {
        self.seen_data(v).and_then(|d| d.edge)
    }
}
