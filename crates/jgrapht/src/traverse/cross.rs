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

//! The cross-component traversal engine and the BFS / DFS / closest-first strategies on top of
//! it (JGraphT's `CrossComponentIterator` family).

use super::events::{TraversalListener, TraversalListeners};
use crate::graph::OutgoingGraph;
use crate::graph::{Graph, GraphExt};
use crate::util::OrderedF64;
use crate::{GraphError, Result};
use rheaps::tree::PairingHeap;
use rheaps::{AddressableHeap, DecreaseKeyHeap};
use std::borrow::Borrow;
use std::collections::{HashMap, VecDeque};
use std::vec::IntoIter;

/// State shared between the engine and its strategy.
pub struct Core<'g, G: OutgoingGraph + 'g, D> {
    /// The graph being traversed.
    pub g: &'g G,
    /// Per-vertex data of every vertex the traversal has reached; the strategy decides what it is.
    pub seen: HashMap<G::Vertex, D>,
    /// Listeners that are told about every traversal event.
    pub listeners: TraversalListeners<'g, G::Vertex, G::Edge>,
}

impl<'g, G: OutgoingGraph + 'g, D> Core<'g, G, D> {
    /// Reports that `v` is finished to the traversal listeners.
    pub fn finish_vertex(&mut self, v: G::Vertex) {
        self.listeners.vertex_finished(&v);
    }
}

/// How a concrete traversal orders vertices (the abstract methods of `CrossComponentIterator`).
pub trait Strategy<'g, G: OutgoingGraph + 'g>: Sized {
    /// What the strategy remembers about each vertex it has seen.
    type Data;
    /// True if the current component has nothing left to visit (may fire finish events).
    fn is_exhausted(&mut self, c: &mut Core<'g, G, Self::Data>) -> bool;
    /// A vertex `v` is reached for the first time, through `e` (`None` for a start vertex).
    fn encounter(&mut self, c: &mut Core<'g, G, Self::Data>, v: G::Vertex, e: Option<G::Edge>);
    /// A vertex `v` that was already seen is reached again through `e`.
    fn encounter_again(&mut self, c: &mut Core<'g, G, Self::Data>, v: G::Vertex, e: G::Edge);
    /// Removes and returns the vertex to visit next. Called only when the strategy is not exhausted.
    fn provide_next(&mut self, c: &mut Core<'g, G, Self::Data>) -> G::Vertex;

    /// What the iterator yields: the vertex itself for a strategy that cannot fail, or a
    /// `Result` for one that can (closest-first rejects invalid edge weights while iterating).
    type Output;
    /// Converts one step of the traversal into [`Strategy::Output`]. A strategy whose `Output` is
    /// a bare vertex never receives an `Err`, because it never records an error.
    fn output(result: Result<G::Vertex>) -> Self::Output;
    /// An error recorded while encountering vertices. The engine reports it as the next step of
    /// the iteration and stops.
    fn take_error(&mut self) -> Option<GraphError> {
        None
    }
}

type EdgeSelector<'g, G> = Box<dyn Fn(&G, &<G as Graph>::Vertex) -> Vec<<G as Graph>::Edge> + 'g>;

#[derive(Clone, Copy, PartialEq, Eq)]
enum State {
    Before,
    Within,
    After,
}

/// Traverses a graph component by component. With cross-component traversal on (the default when
/// no start vertex is given) it restarts at every unseen vertex of the graph; otherwise only at
/// the given start vertices.
pub struct CrossComponentIterator<'g, G: OutgoingGraph + 'g, S: Strategy<'g, G>> {
    core: Core<'g, G, S::Data>,
    strategy: S,
    cross: bool,
    entire: Option<IntoIter<G::Vertex>>,
    starts: Option<IntoIter<G::Vertex>>,
    start_vertex: Option<G::Vertex>,
    state: State,
    selector: Option<EdgeSelector<'g, G>>,
    failed: bool,
}

impl<'g, G: OutgoingGraph + 'g + OutgoingGraph, S: Strategy<'g, G>>
    CrossComponentIterator<'g, G, S>
{
    /// `starts == None` means "whole graph" (cross-component traversal).
    ///
    /// # Errors
    ///
    /// Returns [`GraphError::VertexNotFound`] if a vertex argument is not in the graph. Errors of
    /// the graph operations it performs are propagated.
    pub fn with_strategy(g: &'g G, starts: Option<Vec<G::Vertex>>, strategy: S) -> Result<Self> {
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
            state: State::Before,
            selector: None,
            failed: false,
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
            if !g.contains_vertex(&v) {
                return Err(GraphError::VertexNotFound);
            }
            it.start_vertex = Some(v);
        }
        Ok(it)
    }

    fn entire_iterator(&mut self) -> &mut IntoIter<G::Vertex> {
        let g = self.core.g;
        self.entire
            .get_or_insert_with(|| g.vertices_cloned().collect::<Vec<_>>().into_iter())
    }

    /// The graph being traversed.
    pub fn graph(&self) -> &'g G {
        self.core.g
    }
    /// Whether the traversal continues with the other components once the current one is exhausted.
    pub fn is_cross_component_traversal(&self) -> bool {
        self.cross
    }
    /// Sets whether the traversal continues with the other components of the graph.
    pub fn set_cross_component_traversal(&mut self, cross: bool) {
        self.cross = cross;
    }
    /// The traversal strategy (breadth-first, depth-first, ...).
    pub fn strategy(&self) -> &S {
        &self.strategy
    }
    /// Consumes the iterator and returns its strategy.
    pub fn into_strategy(self) -> S {
        self.strategy
    }

    /// The traversal strategy, mutably.
    pub fn strategy_mut(&mut self) -> &mut S {
        &mut self.strategy
    }
    /// The data the strategy keeps for `v`, or `None` if the traversal has not reached `v` yet.
    pub fn seen_data(&self, v: &G::Vertex) -> Option<&S::Data> {
        self.core.seen.get(v)
    }
    /// Registers a listener that is told about the traversal events from now on.
    pub fn add_traversal_listener(
        &mut self,
        l: impl TraversalListener<G::Vertex, G::Edge> + 'g,
    ) -> usize {
        self.core.listeners.add(l)
    }
    /// Removes traversal listener.
    pub fn remove_traversal_listener(&mut self, id: usize) -> bool {
        self.core.listeners.remove(id)
    }
    /// Replaces the choice of edges followed from a vertex (default: its outgoing edges).
    pub fn set_edge_selector(&mut self, f: impl Fn(&G, &G::Vertex) -> Vec<G::Edge> + 'g) {
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
            if self.state == State::Within {
                self.state = State::After;
                self.core.listeners.component_finished();
            }
            loop {
                let next = if self.cross {
                    self.entire_iterator().next()
                } else {
                    self.starts.as_mut().and_then(Iterator::next)
                };
                let Some(v) = next else { return false };
                if !self.core.g.contains_vertex(&v) {
                    continue;
                }
                if !self.core.seen.contains_key(&v) {
                    self.strategy.encounter(&mut self.core, v, None);
                    self.state = State::Before;
                    return true;
                }
            }
        }
        true
    }

    fn add_unseen_children_of(&mut self, v: &G::Vertex) {
        let g = self.core.g;
        let edges: Vec<G::Edge> = match &self.selector {
            Some(f) => f(g, v),
            None => g.out_edges_of(v).cloned().collect(),
        };
        for e in edges {
            self.core.listeners.edge_traversed(&e);
            let Some(opposite) = g.opposite_vertex(&e, v).cloned() else {
                continue;
            };
            if self.core.seen.contains_key(&opposite) {
                self.strategy.encounter_again(&mut self.core, opposite, e);
            } else {
                self.strategy.encounter(&mut self.core, opposite, Some(e));
            }
        }
    }
}

impl<'g, G: OutgoingGraph + 'g, S: Strategy<'g, G>> CrossComponentIterator<'g, G, S> {
    /// The next vertex, or the error that ended the traversal. After an error the iterator is
    /// exhausted.
    ///
    /// Strategies that cannot fail never return `Err`, so [`Iterator::next`] on them yields
    /// vertices directly; this is the fallible form for generic code.
    ///
    /// # Errors
    ///
    /// Errors of the graph operations it performs are propagated.
    pub fn try_next(&mut self) -> Result<Option<G::Vertex>> {
        if self.failed {
            return Ok(None);
        }
        let step = self.step();
        if step.is_err() {
            self.failed = true;
        }
        step
    }

    fn step(&mut self) -> Result<Option<G::Vertex>> {
        self.encounter_start_vertex();
        if !self.has_next() {
            return Ok(None);
        }
        if self.state == State::Before {
            self.state = State::Within;
            self.core.listeners.component_started();
        }
        let v = self.strategy.provide_next(&mut self.core);
        self.core.listeners.vertex_traversed(&v);
        self.add_unseen_children_of(&v);
        match self.strategy.take_error() {
            Some(e) => Err(e),
            None => Ok(Some(v)),
        }
    }
}

impl<'g, G: OutgoingGraph + 'g, S: Strategy<'g, G>> Iterator for CrossComponentIterator<'g, G, S> {
    type Item = S::Output;
    fn next(&mut self) -> Option<S::Output> {
        self.try_next().transpose().map(S::output)
    }
}

// ---------------------------------------------------------------------------- breadth-first

/// Per-vertex BFS bookkeeping: the spanning-tree edge and the depth.
#[derive(Debug, Clone)]
pub struct BfsData<E> {
    /// The edge through which the vertex was first reached; `None` for a start vertex.
    pub edge: Option<E>,
    /// Distance from the start vertex in edges.
    pub depth: usize,
}

pub struct BfsStrategy<'g, G: OutgoingGraph + 'g> {
    queue: VecDeque<G::Vertex>,
    _g: std::marker::PhantomData<&'g G>,
}

impl<'g, G: OutgoingGraph + 'g> Strategy<'g, G> for BfsStrategy<'g, G> {
    type Data = BfsData<G::Edge>;
    fn is_exhausted(&mut self, _: &mut Core<'g, G, Self::Data>) -> bool {
        self.queue.is_empty()
    }
    fn encounter(&mut self, c: &mut Core<'g, G, Self::Data>, v: G::Vertex, e: Option<G::Edge>) {
        let depth = match &e {
            None => 0,
            Some(e) => c.seen[c.g.opposite_vertex(e, &v).expect("edge touches vertex")].depth + 1,
        };
        c.seen.insert(v.clone(), BfsData { edge: e, depth });
        self.queue.push_back(v);
    }
    fn encounter_again(&mut self, _: &mut Core<'g, G, Self::Data>, _: G::Vertex, _: G::Edge) {}
    fn provide_next(&mut self, _: &mut Core<'g, G, Self::Data>) -> G::Vertex {
        self.queue.pop_front().expect("non-empty queue")
    }
    type Output = G::Vertex;
    fn output(result: Result<G::Vertex>) -> Self::Output {
        result.expect("breadth-first traversal cannot fail")
    }
}

/// Breadth-first traversal; see [`CrossComponentIterator`].
pub type BreadthFirstIterator<'g, G> = CrossComponentIterator<'g, G, BfsStrategy<'g, G>>;

impl<'g, G: OutgoingGraph + 'g> BreadthFirstIterator<'g, G> {
    /// Visits the whole graph, component by component.
    pub fn new(g: &'g G) -> Self {
        Self::with_strategy(
            g,
            None,
            BfsStrategy {
                queue: VecDeque::new(),
                _g: std::marker::PhantomData,
            },
        )
        .expect("no start vertex to validate")
    }
    /// Starts at `start`; call `set_cross_component_traversal(true)` to continue in other components.
    ///
    /// # Errors
    ///
    /// Errors of the graph operations it performs are propagated.
    pub fn from_start(g: &'g G, start: &G::Vertex) -> Result<Self> {
        Self::from_starts(g, [start])
    }
    /// Computes the from starts.
    ///
    /// # Errors
    ///
    /// Errors of the graph operations it performs are propagated.
    pub fn from_starts(
        g: &'g G,
        starts: impl IntoIterator<Item = impl Borrow<G::Vertex>>,
    ) -> Result<Self> {
        Self::with_strategy(
            g,
            Some(starts.into_iter().map(|v| v.borrow().clone()).collect()),
            BfsStrategy {
                queue: VecDeque::new(),
                _g: std::marker::PhantomData,
            },
        )
    }
    /// Parent of `v` in the BFS tree (`None` for roots and unseen vertices).
    pub fn parent(&self, v: &G::Vertex) -> Option<G::Vertex> {
        let e = self.seen_data(v)?.edge.as_ref()?;
        self.graph().opposite_vertex(e, v).cloned()
    }
    /// The edge through which `v` was first reached, or `None` for a start vertex or an unseen vertex.
    pub fn spanning_tree_edge(&self, v: &G::Vertex) -> Option<G::Edge> {
        self.seen_data(v)?.edge.clone()
    }
    /// Number of edges between `v` and its start vertex, or `None` if `v` has not been reached.
    pub fn depth(&self, v: &G::Vertex) -> Option<usize> {
        self.seen_data(v).map(|d| d.depth)
    }
}

// ---------------------------------------------------------------------------- depth-first

#[non_exhaustive]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum VisitColor {
    White,
    Gray,
    Black,
}

enum StackEntry<V> {
    Vertex(V),
    Sentinel,
}

/// Depth-first strategy: vertices wait on a stack, with a sentinel marking where a vertex is finished.
pub struct DfsStrategy<'g, G: OutgoingGraph + 'g> {
    stack: VecDeque<StackEntry<G::Vertex>>,
    _g: std::marker::PhantomData<&'g G>,
}

impl<'g, G: OutgoingGraph + 'g> DfsStrategy<'g, G> {
    /// A strategy with an empty stack.
    pub fn new() -> Self {
        DfsStrategy {
            stack: VecDeque::new(),
            _g: std::marker::PhantomData,
        }
    }

    fn record_finish(&mut self, c: &mut Core<'g, G, VisitColor>) {
        if let Some(StackEntry::Vertex(v)) = self.stack.pop_back() {
            c.seen.insert(v.clone(), VisitColor::Black);
            c.finish_vertex(v);
        }
    }
}

impl<'g, G: OutgoingGraph + 'g> Default for DfsStrategy<'g, G> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'g, G: OutgoingGraph + 'g> Strategy<'g, G> for DfsStrategy<'g, G> {
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
    fn encounter(&mut self, c: &mut Core<'g, G, VisitColor>, v: G::Vertex, _: Option<G::Edge>) {
        c.seen.insert(v.clone(), VisitColor::White);
        self.stack.push_back(StackEntry::Vertex(v));
    }
    fn encounter_again(&mut self, c: &mut Core<'g, G, VisitColor>, v: G::Vertex, _: G::Edge) {
        if c.seen[&v] != VisitColor::White {
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
    fn provide_next(&mut self, c: &mut Core<'g, G, VisitColor>) -> G::Vertex {
        let v = loop {
            match self.stack.pop_back().expect("non-empty stack") {
                StackEntry::Sentinel => self.record_finish(c),
                StackEntry::Vertex(v) => break v,
            }
        };
        self.stack.push_back(StackEntry::Vertex(v.clone()));
        self.stack.push_back(StackEntry::Sentinel);
        c.seen.insert(v.clone(), VisitColor::Gray);
        v
    }
    type Output = G::Vertex;
    fn output(result: Result<G::Vertex>) -> Self::Output {
        result.expect("depth-first traversal cannot fail")
    }
}

/// Depth-first traversal; see [`CrossComponentIterator`].
pub type DepthFirstIterator<'g, G> = CrossComponentIterator<'g, G, DfsStrategy<'g, G>>;

impl<'g, G: OutgoingGraph + 'g> DepthFirstIterator<'g, G> {
    /// Creates a new instance.
    pub fn new(g: &'g G) -> Self {
        Self::with_strategy(
            g,
            None,
            DfsStrategy {
                stack: VecDeque::new(),
                _g: std::marker::PhantomData,
            },
        )
        .expect("no start vertex to validate")
    }
    /// Computes the from start.
    ///
    /// # Errors
    ///
    /// Errors of the graph operations it performs are propagated.
    pub fn from_start(g: &'g G, start: &G::Vertex) -> Result<Self> {
        Self::from_starts(g, [start])
    }
    /// Computes the from starts.
    ///
    /// # Errors
    ///
    /// Errors of the graph operations it performs are propagated.
    pub fn from_starts(
        g: &'g G,
        starts: impl IntoIterator<Item = impl Borrow<G::Vertex>>,
    ) -> Result<Self> {
        Self::with_strategy(
            g,
            Some(starts.into_iter().map(|v| v.borrow().clone()).collect()),
            DfsStrategy {
                stack: VecDeque::new(),
                _g: std::marker::PhantomData,
            },
        )
    }
    /// The visit color of `v`: gray while it is on the stack, black once finished, `None` if unseen.
    pub fn color(&self, v: &G::Vertex) -> Option<VisitColor> {
        self.seen_data(v).copied()
    }
}

// ---------------------------------------------------------------------------- closest-first

pub struct CfData<G: OutgoingGraph, Hd> {
    handle: Option<Hd>,
    key: f64,
    edge: Option<G::Edge>,
    frozen: bool,
}

pub struct ClosestFirstStrategy<'g, G: OutgoingGraph + 'g, H> {
    heap: H,
    radius: f64,
    /// The first invalid edge weight met while expanding a vertex; reported by the iterator.
    error: Option<GraphError>,
    _g: std::marker::PhantomData<&'g G>,
}

impl<'g, G, H> ClosestFirstStrategy<'g, G, H>
where
    G: OutgoingGraph + 'g,
    H: AddressableHeap<OrderedF64, G::Vertex>,
{
    /// Length of the path to `v` through `e`, or `None` after recording the invalid weight.
    fn path_length(
        &mut self,
        c: &Core<'g, G, CfData<G, H::Handle>>,
        v: &G::Vertex,
        e: &G::Edge,
    ) -> Option<f64> {
        let w = c.g.weight_of(e);
        if w.is_nan() || w < 0.0 {
            self.error.get_or_insert(GraphError::InvalidWeight(
                crate::WeightProblem::NegativeOrNaN,
            ));
            return None;
        }
        let other = c.g.opposite_vertex(e, v)?;
        Some(c.seen[other].key + w)
    }
}

impl<'g, G, H> Strategy<'g, G> for ClosestFirstStrategy<'g, G, H>
where
    G: OutgoingGraph + 'g,
    H: DecreaseKeyHeap<OrderedF64, G::Vertex>,
{
    type Data = CfData<G, H::Handle>;
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
    fn encounter(&mut self, c: &mut Core<'g, G, Self::Data>, v: G::Vertex, e: Option<G::Edge>) {
        let len = match &e {
            None => 0.0,
            Some(e) => match self.path_length(c, &v, e) {
                Some(len) => len,
                None => return,
            },
        };
        let handle = self.heap.insert(OrderedF64(len), v.clone());
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
    fn encounter_again(&mut self, c: &mut Core<'g, G, Self::Data>, v: G::Vertex, e: G::Edge) {
        if c.seen[&v].frozen {
            return;
        }
        let Some(cand) = self.path_length(c, &v, &e) else {
            return;
        };
        let d = c.seen.get_mut(&v).expect("seen");
        if cand < d.key {
            d.edge = Some(e);
            d.key = cand;
            self.heap
                .decrease_key(d.handle.expect("live handle"), OrderedF64(cand))
                .expect("smaller key");
        }
    }
    fn provide_next(&mut self, c: &mut Core<'g, G, Self::Data>) -> G::Vertex {
        let (_, v) = self.heap.pop().expect("non-empty heap");
        let d = c.seen.get_mut(&v).expect("seen");
        d.frozen = true;
        d.handle = None;
        v
    }
    type Output = Result<G::Vertex>;
    fn output(result: Result<G::Vertex>) -> Self::Output {
        result
    }
    fn take_error(&mut self) -> Option<GraphError> {
        self.error.take()
    }
}

/// Closest-first (Dijkstra order) traversal. Its items are `Result`s: a negative or NaN edge weight
/// is yielded once as `Err(GraphError::InvalidWeight(..))` in place of the vertex whose outgoing
/// edge carries it, and the iterator then ends. Collect
/// with `collect::<Result<Vec<_>>>()` or call [`CrossComponentIterator::try_next`].
pub type ClosestFirstIterator<'g, G, H = PairingHeap<OrderedF64, <G as Graph>::Vertex>> =
    CrossComponentIterator<'g, G, ClosestFirstStrategy<'g, G, H>>;

impl<'g, G: OutgoingGraph + 'g> ClosestFirstIterator<'g, G> {
    /// Creates a new instance.
    pub fn new(g: &'g G) -> Self {
        Self::build(g, None, f64::INFINITY, PairingHeap::new())
            .expect("no radius with cross-component traversal")
    }
    /// Computes the from start.
    ///
    /// # Errors
    ///
    /// Errors of the graph operations it performs are propagated.
    pub fn from_start(g: &'g G, start: &G::Vertex) -> Result<Self> {
        Self::build(
            g,
            Some(vec![start.clone()]),
            f64::INFINITY,
            PairingHeap::new(),
        )
    }
    /// Computes the from start with radius.
    ///
    /// # Errors
    ///
    /// Errors of the graph operations it performs are propagated.
    pub fn from_start_with_radius(g: &'g G, start: &G::Vertex, radius: f64) -> Result<Self> {
        Self::build(g, Some(vec![start.clone()]), radius, PairingHeap::new())
    }
    /// Computes the from starts.
    ///
    /// # Errors
    ///
    /// Errors of the graph operations it performs are propagated.
    pub fn from_starts(
        g: &'g G,
        starts: impl IntoIterator<Item = impl Borrow<G::Vertex>>,
    ) -> Result<Self> {
        Self::build(
            g,
            Some(starts.into_iter().map(|v| v.borrow().clone()).collect()),
            f64::INFINITY,
            PairingHeap::new(),
        )
    }
    /// Performs `from starts with radius`.
    ///
    /// # Errors
    ///
    /// Errors of the graph operations it performs are propagated.
    pub fn from_starts_with_radius(
        g: &'g G,
        starts: impl IntoIterator<Item = impl Borrow<G::Vertex>>,
        radius: f64,
    ) -> Result<Self> {
        Self::build(
            g,
            Some(starts.into_iter().map(|v| v.borrow().clone()).collect()),
            radius,
            PairingHeap::new(),
        )
    }
}

impl<'g, G, H> ClosestFirstIterator<'g, G, H>
where
    G: OutgoingGraph + 'g + OutgoingGraph,
    H: DecreaseKeyHeap<OrderedF64, G::Vertex>,
{
    /// Full constructor with an explicit heap. A finite radius is incompatible with
    /// cross-component traversal (`starts == None`).
    ///
    /// # Errors
    ///
    /// Returns [`GraphError::InvalidParameter`] if a parameter is out of range. Errors of the graph
    /// operations it performs are propagated.
    pub fn build(g: &'g G, starts: Option<Vec<G::Vertex>>, radius: f64, heap: H) -> Result<Self> {
        if radius.is_nan() || radius < 0.0 {
            return Err(GraphError::must_be_non_negative("radius"));
        }
        if starts.is_none() && radius != f64::INFINITY {
            return Err(GraphError::InvalidParameter {
                name: "radius",
                requirement: crate::ParameterRequirement::Consistent,
            });
        }
        let rest: Vec<G::Vertex> = starts
            .as_ref()
            .map(|s| s.iter().skip(1).cloned().collect())
            .unwrap_or_default();
        let multi = starts.is_some();
        let strategy = ClosestFirstStrategy {
            heap,
            radius,
            error: None,
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

    /// Distance of `v` from the closest start vertex, or `None` if the traversal has not reached
    /// `v` (yet). For a vertex that is reached but not yet yielded the value is tentative.
    pub fn shortest_path_length(&self, v: &G::Vertex) -> Option<f64> {
        self.seen_data(v).map(|d| d.key)
    }
    /// The edge through which `v` was reached on its shortest path, or `None` for a start vertex or an unseen vertex.
    pub fn spanning_tree_edge(&self, v: &G::Vertex) -> Option<G::Edge> {
        self.seen_data(v).and_then(|d| d.edge.clone())
    }
}
