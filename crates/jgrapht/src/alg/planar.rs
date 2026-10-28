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

//! Boyer–Myrvold planarity testing with embedding and Kuratowski subdivision extraction
//! (`BoyerMyrvoldPlanarityInspector`).
//!
//! The original works on a web of linked objects; here nodes and edges live in two arenas and
//! refer to each other by index. Directions of edges are ignored.

use super::interfaces::Embedding;
use crate::graph::GraphExt;
use crate::graph::IncidenceGraph;
use crate::view::AsSubgraph;
use indexmap::IndexMap;
use std::borrow::Borrow;
use std::collections::{HashMap, HashSet};

/// The outcome of a planarity test together with its certificate.
pub enum PlanarityResult<'g, G: IncidenceGraph> {
    /// The graph is planar; this is a combinatorial embedding of it.
    Planar(Embedding<G::Vertex, G::Edge>),
    /// The graph is not planar; this subgraph is a subdivision of K5 or K3,3.
    NonPlanar(AsSubgraph<&'g G>),
}

impl<'g, G: IncidenceGraph> PlanarityResult<'g, G> {
    /// Whether the graph is planar.
    pub fn is_planar(&self) -> bool {
        matches!(self, Self::Planar(_))
    }

    /// The embedding, if the graph is planar.
    pub fn embedding(&self) -> Option<&Embedding<G::Vertex, G::Edge>> {
        match self {
            Self::Planar(embedding) => Some(embedding),
            Self::NonPlanar(_) => None,
        }
    }

    /// The Kuratowski subdivision, if the graph is not planar.
    pub fn kuratowski_subdivision(&self) -> Option<&AsSubgraph<&'g G>> {
        match self {
            Self::NonPlanar(subdivision) => Some(subdivision),
            Self::Planar(_) => None,
        }
    }
}

/// A circular doubly linked list living in a [`Pool`]; constant time splicing and removal by
/// handle, as with Java's `DoublyLinkedList`.
#[derive(Clone, Copy)]
struct List {
    head: usize,
    size: usize,
}

impl List {
    const EMPTY: List = List {
        head: usize::MAX,
        size: 0,
    };
    fn is_empty(&self) -> bool {
        self.size == 0
    }
}

/// Storage for the nodes of all lists: a link has a value (an edge or node index) and neighbours.
#[derive(Default)]
struct Pool {
    prev: Vec<usize>,
    next: Vec<usize>,
    val: Vec<usize>,
}

impl Pool {
    fn new_link(&mut self, val: usize) -> usize {
        self.val.push(val);
        self.prev.push(NIL);
        self.next.push(NIL);
        self.val.len() - 1
    }

    /// Appends a value; returns its handle.
    fn push_back(&mut self, l: &mut List, val: usize) -> usize {
        let n = self.new_link(val);
        if l.size == 0 {
            self.prev[n] = n;
            self.next[n] = n;
            l.head = n;
        } else {
            let h = l.head;
            let t = self.prev[h];
            self.next[t] = n;
            self.prev[n] = t;
            self.next[n] = h;
            self.prev[h] = n;
        }
        l.size += 1;
        n
    }

    fn push_front(&mut self, l: &mut List, val: usize) -> usize {
        let n = self.push_back(l, val);
        l.head = n;
        n
    }

    fn front(&self, l: &List) -> usize {
        self.val[l.head]
    }

    fn back(&self, l: &List) -> usize {
        self.val[self.prev[l.head]]
    }

    /// Removes the link `n` from `l`.
    fn remove(&mut self, l: &mut List, n: usize) {
        if l.size == 1 {
            l.head = NIL;
        } else {
            let (p, nx) = (self.prev[n], self.next[n]);
            self.next[p] = nx;
            self.prev[nx] = p;
            if l.head == n {
                l.head = nx;
            }
        }
        l.size -= 1;
        self.prev[n] = NIL;
        self.next[n] = NIL;
    }

    /// Moves all of `b` behind `a`, or in front of it when `at_front`.
    fn splice(&mut self, a: &mut List, b: &mut List, at_front: bool) {
        if b.size == 0 {
            return;
        }
        if a.size == 0 {
            *a = *b;
        } else {
            let (ah, bh) = (a.head, b.head);
            let (at, bt) = (self.prev[ah], self.prev[bh]);
            self.next[at] = bh;
            self.prev[bh] = at;
            self.next[bt] = ah;
            self.prev[ah] = bt;
            a.size += b.size;
            if at_front {
                a.head = bh;
            }
        }
        *b = List::EMPTY;
    }

    /// Reverses the list (linear, like Java's `invert`).
    fn reverse(&mut self, l: &mut List) {
        if l.size < 2 {
            return;
        }
        let new_head = self.prev[l.head];
        let mut current = l.head;
        loop {
            std::mem::swap(&mut self.next[current], &mut self.prev[current]);
            current = self.prev[current];
            if current == l.head {
                break;
            }
        }
        l.head = new_head;
    }

    fn to_vec(&self, l: &List) -> Vec<usize> {
        let mut out = Vec::with_capacity(l.size);
        if l.size > 0 {
            let mut c = l.head;
            loop {
                out.push(self.val[c]);
                c = self.next[c];
                if c == l.head {
                    break;
                }
            }
        }
        out
    }

    fn retain(&mut self, l: &mut List, keep: impl Fn(usize) -> bool) {
        let mut c = l.head;
        for _ in 0..l.size {
            let next = self.next[c];
            if !keep(self.val[c]) {
                self.remove(l, c);
            }
            c = next;
        }
    }
}

type NodeId = usize;
type EdgeId = usize;
const NIL: usize = usize::MAX;

struct PEdge<E> {
    graph_edge: Option<E>,
    source: NodeId,
    target: NodeId,
    sign: i32,
    embedded: bool,
    short_circuit: bool,
}

struct PNode<V> {
    graph_vertex: Option<V>,
    root_vertex: bool,
    dfs_index: usize,
    height: usize,
    lowpoint: usize,
    least_ancestor: usize,
    visited: usize,
    back_edge_flag: usize,
    boundary_height: i32,
    marked: bool,
    parent_edge: EdgeId,
    edge_to_embed: EdgeId,
    initial_component_root: NodeId,
    outer: [NodeId; 2],
    separated: List,
    pertinent: List,
    list_node: usize,
    tree_edges: Vec<EdgeId>,
    down_edges: Vec<EdgeId>,
    back_edges: Vec<EdgeId>,
    embedded: List,
}

#[derive(Clone, Copy)]
struct MergeInfo {
    parent: NodeId,
    parent_next: NodeId,
    child: NodeId,
    child_prev: NodeId,
    v_in: usize,
    v_out: usize,
}

/// Walks the outer face of a biconnected component (`OuterFaceCirculator`).
#[derive(Clone, Copy)]
struct Circ {
    current: NodeId,
    prev: NodeId,
}

enum Finish {
    NegativeBoundary,
    Node(NodeId),
}

/// An implementation of the Boyer-Myrvold planarity testing algorithm. This class determines whether
/// an input graph is planar or not. If the graph is planar, the algorithm provides a
/// [combinatorial embedding](https://en.wikipedia.org/wiki/Graph_embedding#Combinatorial_embedding) of the graph, which is represented as a clockwise ordering of the edges of the
/// graph. Otherwise, the algorithm provides a
/// [Kuratowski subgraph](https://en.wikipedia.org/wiki/Kuratowski%27s_theorem#Kuratowski_subgraphs) as a certificate. Both embedding of the graph and Kuratowski subdivision are
/// computed lazily, meaning that the call to the `BoyerMyrvoldPlanarityInspector#isPlanar()`
/// does spend time only on the planarity testing. All of the operations of this algorithm (testing,
/// embedding and Kuratowski subgraph extraction) run in linear time.
///
/// A [planar graph](https://en.wikipedia.org/wiki/Planar_graph) is a graph, which can be
/// drawn in two-dimensional space without any of its edges crossing. According to the
/// [Kuratowski theorem](https://en.wikipedia.org/wiki/Kuratowski%27s_theorem), a graph is
/// planar if and only if it doesn't contain a subdivision of the $K_{3,3}$ or $K_{5}$ graphs.
///
/// The Boyer-Myrvold planarity testing algorithm was originally described in: *Boyer, John amp;
/// Myrvold, Wendy. (2004). On the Cutting Edge: Simplified O(n) Planarity by Edge Addition. J. Graph
/// Algorithms Appl.. 8. 241-273. 10.7155/jgaa.00091. *. We refer to this paper for the complete
/// description of the Boyer-Myrvold algorithm
///
/// # Rust notes
///
/// Planarity test, planar embedding and Kuratowski subdivision of a graph.
///
/// [`BoyerMyrvoldPlanarityInspector::planarity`] returns whichever certificate applies as one
/// [`PlanarityResult`], so there is no embedding to ask for on a non-planar graph or subdivision to
/// ask for on a planar one.
///
/// Self-loops and parallel edges do not affect planarity. The test uses only the first edge between
/// two vertices and ignores self-loops, so the embedding and the Kuratowski subdivision contain at
/// most one edge per vertex pair.
pub struct BoyerMyrvoldPlanarityInspector<'g, G: IncidenceGraph> {
    graph: &'g G,
    n: usize,
    arena: Vec<PNode<G::Vertex>>,
    pool: Pool,
    edges: Vec<PEdge<G::Edge>>,
    /// The non-root nodes, ordered by lowpoint after `orient`.
    nodes: Vec<NodeId>,
    dfs_tree_roots: Vec<NodeId>,
    component_roots: Vec<NodeId>,
    stack: Vec<MergeInfo>,
    failed_v: NodeId,
    tested: bool,
    planar: bool,
    embedding: Option<Embedding<G::Vertex, G::Edge>>,
    kuratowski: Option<(Vec<G::Vertex>, Vec<G::Edge>)>,
}

impl<'g, G: IncidenceGraph> BoyerMyrvoldPlanarityInspector<'g, G> {
    /// Creates new instance of the planarity testing algorithm for the `graph`. The input
    /// graph can't be null.
    pub fn new(graph: &'g G) -> Self {
        BoyerMyrvoldPlanarityInspector {
            graph,
            n: graph.vertex_count(),
            arena: Vec::new(),
            pool: Pool::default(),
            edges: Vec::new(),
            nodes: Vec::new(),
            dfs_tree_roots: Vec::new(),
            component_roots: Vec::new(),
            stack: Vec::new(),
            failed_v: NIL,
            tested: false,
            planar: false,
            embedding: None,
            kuratowski: None,
        }
    }

    /// Tests the planarity of the `graph`. Returns true if the input graph is planar, false
    /// otherwise. If this method returns true, the combinatorial embedding of the `graph` is
    /// provided after the call to the `PlanarityTestingAlgorithm#getEmbedding()`. Otherwise, a
    /// Kuratowski subdivision is provided after the call to the
    /// `PlanarityTestingAlgorithm#getKuratowskiSubdivision()`.
    pub fn is_planar(&mut self) -> bool {
        self.lazy_test_planarity()
    }

    /// Tests the planarity of the graph and returns the certificate of the answer: a combinatorial
    /// embedding if it is planar, a subdivision of K5 or K3,3 if it is not.
    ///
    /// # Panics
    ///
    /// Panics only if an internal invariant is violated, which indicates a bug in this library.
    pub fn planarity(&mut self) -> PlanarityResult<'g, G> {
        if self.is_planar() {
            self.lazy_compute_embedding();
            PlanarityResult::Planar(self.embedding.clone().expect("computed above"))
        } else {
            self.lazy_extract_kuratowski();
            let (vs, es) = self.kuratowski.clone().expect("extracted above");
            PlanarityResult::NonPlanar(AsSubgraph::with_edges(self.graph, vs, es))
        }
    }

    // ----- node and edge construction -----------------------------------------------------------

    fn new_node(
        &mut self,
        graph_vertex: Option<G::Vertex>,
        dfs_index: usize,
        parent_edge: EdgeId,
        root_vertex: bool,
    ) -> NodeId {
        let mut embedded = List::EMPTY;
        if parent_edge != NIL {
            self.pool.push_back(&mut embedded, parent_edge);
        }
        self.arena.push(PNode {
            graph_vertex,
            root_vertex,
            dfs_index,
            height: 0,
            lowpoint: 0,
            least_ancestor: 0,
            visited: self.n,
            back_edge_flag: self.n,
            boundary_height: 0,
            marked: false,
            parent_edge,
            edge_to_embed: NIL,
            initial_component_root: NIL,
            outer: [NIL, NIL],
            separated: List::EMPTY,
            pertinent: List::EMPTY,
            list_node: NIL,
            tree_edges: Vec::new(),
            down_edges: Vec::new(),
            back_edges: Vec::new(),
            embedded,
        });
        self.arena.len() - 1
    }

    fn new_edge(&mut self, graph_edge: Option<G::Edge>, source: NodeId, target: NodeId) -> EdgeId {
        self.edges.push(PEdge {
            graph_edge,
            source,
            target,
            sign: 1,
            embedded: false,
            short_circuit: false,
        });
        self.edges.len() - 1
    }

    fn new_short_circuit(&mut self, source: NodeId, target: NodeId) -> EdgeId {
        let e = self.new_edge(None, source, target);
        self.edges[e].short_circuit = true;
        self.edges[e].embedded = true;
        e
    }

    /// Creates a new node by converting the `graphVertex` to the internal node representation.
    fn create_new_node(
        &mut self,
        vertex_map: &mut HashMap<G::Vertex, NodeId>,
        graph_vertex: G::Vertex,
        edge: Option<G::Edge>,
        parent: Option<NodeId>,
        dfs_index: usize,
    ) -> NodeId {
        let child;
        match parent {
            None => {
                child = self.new_node(Some(graph_vertex.clone()), dfs_index, NIL, false);
                self.arena[child].outer = [child, child];
                self.dfs_tree_roots.push(child);
            }
            Some(parent) => {
                let tree_edge = self.new_edge(edge, parent, NIL);
                let component_root =
                    self.new_node(None, self.arena[parent].dfs_index, tree_edge, true);
                child = self.new_node(Some(graph_vertex.clone()), dfs_index, tree_edge, false);
                self.arena[child].height = self.arena[parent].height + 1;
                self.arena[child].initial_component_root = component_root;
                self.edges[tree_edge].target = child;
                self.component_roots.push(component_root);
                self.arena[parent].tree_edges.push(tree_edge);
                self.arena[child].outer = [component_root, component_root];
                self.arena[component_root].outer = [child, child];
            }
        }
        self.nodes.push(child);
        vertex_map.insert(graph_vertex, child);
        child
    }

    // ----- DFS orientation ----------------------------------------------------------------------

    /// Orients the input graph according to its dfs traversal by creating a dfs tree. Computes the
    /// least ancestors and lowpoints of the nodes
    fn orient_dfs(
        &mut self,
        vertex_map: &mut HashMap<G::Vertex, NodeId>,
        start: G::Vertex,
        mut current_dfs_index: usize,
    ) -> usize {
        struct Info<V, E> {
            current: V,
            parent: Option<V>,
            parent_edge: Option<E>,
            backtrack: bool,
        }
        let graph = self.graph;
        let mut stack = vec![Info {
            current: start,
            parent: None,
            parent_edge: None,
            backtrack: false,
        }];
        while let Some(info) = stack.pop() {
            if info.backtrack {
                let current = vertex_map[&info.current];
                let dfs = self.arena[current].dfs_index;
                let mut least_ancestor = dfs;
                for &be in &self.arena[current].back_edges {
                    least_ancestor =
                        least_ancestor.min(self.arena[self.edges[be].target].dfs_index);
                }
                let mut lowpoint = dfs;
                for &te in &self.arena[current].tree_edges {
                    lowpoint = lowpoint.min(self.arena[self.edges[te].target].lowpoint);
                }
                self.arena[current].least_ancestor = least_ancestor;
                self.arena[current].lowpoint = lowpoint.min(least_ancestor);
            } else {
                if vertex_map.contains_key(&info.current) {
                    continue;
                }
                stack.push(Info {
                    current: info.current.clone(),
                    parent: info.parent.clone(),
                    parent_edge: info.parent_edge.clone(),
                    backtrack: true,
                });
                let parent_node = info.parent.as_ref().map(|p| vertex_map[p]);
                let current = self.create_new_node(
                    vertex_map,
                    info.current.clone(),
                    info.parent_edge.clone(),
                    parent_node,
                    current_dfs_index,
                );
                current_dfs_index += 1;
                let mut seen_neighbors: HashSet<&G::Vertex> = HashSet::new();
                for e in graph.edges_of(&info.current) {
                    let opposite = graph.opposite_vertex(e, &info.current).unwrap();
                    // Self-loops and parallel edges never affect planarity. The algorithm
                    // assumes a simple graph (a loop would be a back edge from a node to itself),
                    // so only the first edge to each neighbour is used.
                    if opposite == &info.current || !seen_neighbors.insert(opposite) {
                        continue;
                    }
                    if let Some(&opposite_node) = vertex_map.get(opposite) {
                        if info.parent.as_ref() == Some(opposite) {
                            continue;
                        }
                        let back_edge = self.new_edge(Some(e.clone()), current, opposite_node);
                        self.arena[opposite_node].down_edges.push(back_edge);
                        self.arena[current].back_edges.push(back_edge);
                    } else {
                        stack.push(Info {
                            current: opposite.clone(),
                            parent: Some(info.current.clone()),
                            parent_edge: Some(e.clone()),
                            backtrack: false,
                        });
                    }
                }
            }
        }
        current_dfs_index
    }

    /// Iteratively start an orienting dfs from every `graph` vertex that hasn't been visited
    /// yet. After orienting the graph, sorts the nodes by their lowpoints and adds them to the
    /// `separatedDfsChildList`
    fn orient(&mut self) {
        let mut visited: HashMap<G::Vertex, NodeId> = HashMap::new();
        let mut index = 0;
        let graph = self.graph;
        for v in graph.vertices() {
            let v = v.borrow();
            if !visited.contains_key(v) {
                index = self.orient_dfs(&mut visited, v.clone(), index);
            }
        }
        self.sort_vertices();
    }

    /// Bucket-sorts the nodes by lowpoint and builds the separated-children lists in that order.
    fn sort_vertices(&mut self) {
        let mut buckets: Vec<Vec<NodeId>> = vec![Vec::new(); self.n];
        for &node in &self.nodes {
            buckets[self.arena[node].lowpoint].push(node);
        }
        let mut sorted = Vec::with_capacity(self.n);
        for list in buckets {
            for node in list {
                sorted.push(node);
                let pe = self.arena[node].parent_edge;
                if pe != NIL {
                    let parent = self.edges[pe].source;
                    let handle = self.pool.push_back(&mut self.arena[parent].separated, node);
                    self.arena[node].list_node = handle;
                }
            }
        }
        self.nodes = sorted;
    }

    // ----- node predicates and outer face navigation --------------------------------------------

    fn is_visited_wrt(&self, x: NodeId, node: NodeId) -> bool {
        self.arena[node].dfs_index == self.arena[x].visited
    }

    fn is_pertinent_wrt(&self, x: NodeId, node: NodeId) -> bool {
        let n = &self.arena[x];
        n.back_edge_flag == self.arena[node].dfs_index || !n.pertinent.is_empty()
    }

    fn has_back_edge_wrt(&self, x: NodeId, node: NodeId) -> bool {
        self.arena[x].back_edge_flag == self.arena[node].dfs_index
    }

    fn is_externally_active_wrt(&self, x: NodeId, node: NodeId) -> bool {
        let n = &self.arena[x];
        let bound = self.arena[node].dfs_index;
        n.least_ancestor < bound
            || (!n.separated.is_empty()
                && self.arena[self.pool.front(&n.separated)].lowpoint < bound)
    }

    fn is_internally_active_wrt(&self, x: NodeId, node: NodeId) -> bool {
        self.is_pertinent_wrt(x, node) && !self.is_externally_active_wrt(x, node)
    }

    fn is_inactive_wrt(&self, x: NodeId, node: NodeId) -> bool {
        !self.is_externally_active_wrt(x, node) && !self.is_pertinent_wrt(x, node)
    }

    fn is_active_wrt(&self, x: NodeId, node: NodeId) -> bool {
        !self.is_inactive_wrt(x, node)
    }

    fn is_root(&self, x: NodeId) -> bool {
        self.arena[x].root_vertex
    }

    fn parent_of(&self, x: NodeId) -> NodeId {
        match self.arena[x].parent_edge {
            NIL => NIL,
            pe => self.edges[pe].source,
        }
    }

    /// Checks whether this node has a neighbor, which is a root of a biconnected component
    fn has_root_neighbor(&self, x: NodeId) -> bool {
        let o = self.arena[x].outer;
        self.is_root(o[0]) || self.is_root(o[1])
    }

    /// Returns a neighbor of this node which is not equal to the `prev`
    fn next_on_outer_face(&self, x: NodeId, prev: NodeId) -> NodeId {
        let o = self.arena[x].outer;
        if o[0] == prev { o[1] } else { o[0] }
    }

    /// Substitutes the neighbor `node` with a `newNeighbor`
    fn substitute(&mut self, x: NodeId, node: NodeId, new_neighbor: NodeId) {
        let o = &mut self.arena[x].outer;
        if o[0] == node {
            o[0] = new_neighbor;
        } else {
            o[1] = new_neighbor;
        }
    }

    /// Substitutes a neighbor of this node, which is not equal to the `node`, with the
    /// `newNeighbor`
    fn substitute_another(&mut self, x: NodeId, node: NodeId, new_neighbor: NodeId) {
        let o = &mut self.arena[x].outer;
        if o[0] == node {
            o[1] = new_neighbor;
        } else {
            o[0] = new_neighbor;
        }
    }

    /// Returns a circulator, that moves in the direction `direction`. The next node along
    /// the traversal will be `this.outerFaceNeighbor[direction]`.
    fn iterator(&self, x: NodeId, dir: usize) -> Circ {
        Circ {
            current: self.arena[x].outer[dir],
            prev: x,
        }
    }

    fn circ_next(&self, c: &mut Circ) -> NodeId {
        let t = c.current;
        c.current = self.next_on_outer_face(c.current, c.prev);
        c.prev = t;
        c.prev
    }

    fn circ_current(&self, c: &Circ) -> NodeId {
        c.prev
    }

    fn circ_prev(&self, c: &Circ) -> NodeId {
        self.next_on_outer_face(c.prev, c.current)
    }

    fn to_existing(&self, x: NodeId) -> NodeId {
        if self.is_root(x) {
            self.parent_of(x)
        } else {
            x
        }
    }

    /// Returns the opposite node of the `node`
    fn opposite(&self, e: EdgeId, x: NodeId) -> NodeId {
        let edge = &self.edges[e];
        if edge.source == x {
            edge.target
        } else {
            edge.source
        }
    }

    fn circ_edge_to_next(&self, c: &Circ) -> EdgeId {
        let first = self.emb_front(c.prev);
        let target = self.to_existing(c.current);
        let source = self.to_existing(c.prev);
        if self.opposite(first, source) == target {
            first
        } else {
            self.emb_back(c.prev)
        }
    }

    fn emb_front(&self, x: NodeId) -> EdgeId {
        self.pool.front(&self.arena[x].embedded)
    }

    fn emb_back(&self, x: NodeId) -> EdgeId {
        self.pool.back(&self.arena[x].embedded)
    }

    fn emb_vec(&self, x: NodeId) -> Vec<EdgeId> {
        self.pool.to_vec(&self.arena[x].embedded)
    }

    fn separated_vec(&self, x: NodeId) -> Vec<NodeId> {
        self.pool.to_vec(&self.arena[x].separated)
    }

    // ----- embedding primitives -----------------------------------------------------------------

    /// `Node.embedBackEdge`: puts `edge` at the end of `x`'s embedded list that faces `prev`.
    fn node_embed_back_edge(&mut self, x: NodeId, edge: EdgeId, mut prev: NodeId) {
        if self.is_root(prev) {
            prev = self.parent_of(prev);
        }
        let first = self.emb_front(x);
        if self.opposite(first, x) == prev {
            self.pool.push_front(&mut self.arena[x].embedded, edge);
        } else {
            self.pool.push_back(&mut self.arena[x].embedded, edge);
        }
    }

    /// Merges the embedded edges of the child component root into this node's embedded edges.
    /// Note, that the edges in the `edges` list are always in the clockwise order. There
    /// are 3 parameters which determine how the `edges` list is merged: the `vIn`
    /// direction, the `vOut` direction and the orientation of the edges around this node
    /// (clockwise or counterclockwise). The edges in the `edges` list should have the same
    /// orientation. If this list is inverted, the sign of the `parentEdge` is set to $-1$.
    fn merge_child_edges(
        &mut self,
        x: NodeId,
        mut edges: List,
        v_in: usize,
        v_out: usize,
        parent_next: NodeId,
        parent_edge: EdgeId,
    ) {
        let first = self.emb_front(x);
        let along_parent_traversal = self.opposite(first, x) != parent_next;
        let (mut action_append, mut invert) = (false, false);
        if v_in == 0 {
            if v_out == 0 {
                if !along_parent_traversal {
                    invert = true;
                    action_append = true;
                }
            } else if along_parent_traversal {
                invert = true;
            } else {
                action_append = true;
            }
        } else if v_out == 0 {
            if !along_parent_traversal {
                invert = true;
                action_append = true;
            }
        } else if along_parent_traversal {
            invert = true;
        } else {
            action_append = true;
        }
        if invert {
            self.edges[parent_edge].sign = -1;
            self.pool.reverse(&mut edges);
        }
        self.pool
            .splice(&mut self.arena[x].embedded, &mut edges, !action_append);
    }

    /// Merges the last two biconnected components using the info stored on top of the stack. The key
    /// goal of this method is to merge the outer faces of the two components and to merge the
    /// embedded edges of the child component root with the embedded edges of the component parent
    /// node.
    fn merge_biconnected_component(&mut self) {
        let info = self.stack.pop().unwrap();
        let virtual_root = info.child;
        if info.v_in != info.v_out {
            self.arena[virtual_root].outer.swap(0, 1);
        }
        let root = info.parent;
        let virtual_root_child = self.edges[self.arena[virtual_root].parent_edge].target;
        let handle = self.arena[virtual_root].list_node;
        self.pool.remove(&mut self.arena[root].pertinent, handle);
        let handle = self.arena[virtual_root_child].list_node;
        self.pool.remove(&mut self.arena[root].separated, handle);
        let child_edges = std::mem::replace(&mut self.arena[virtual_root].embedded, List::EMPTY);
        let parent_edge = self.arena[virtual_root].parent_edge;
        self.merge_child_edges(
            root,
            child_edges,
            info.v_in,
            info.v_out,
            info.parent_next,
            parent_edge,
        );
        self.substitute_another(root, info.parent_next, info.child_prev);
        self.substitute(info.child_prev, virtual_root, root);
        self.arena[virtual_root].outer = [NIL, NIL];
    }

    /// Embeds the back edge `edge` into the list of embedded edges of the source and the
    /// virtual target of the edge such that the `childPrev` belongs to the new inner face.
    /// This method also takes care of modifying the boundary of the outer face accordingly
    fn embed_back_edge(
        &mut self,
        root: NodeId,
        entry_dir: usize,
        edge: EdgeId,
        child_prev: NodeId,
    ) -> Circ {
        if entry_dir == 0 {
            self.pool.push_back(&mut self.arena[root].embedded, edge);
        } else {
            self.pool.push_front(&mut self.arena[root].embedded, edge);
        }
        let child = self.edges[edge].source;
        self.node_embed_back_edge(child, edge, child_prev);
        self.arena[child].edge_to_embed = NIL;
        self.arena[child].back_edge_flag = self.n;
        self.edges[edge].embedded = true;
        self.substitute(child, child_prev, root);
        self.arena[root].outer[entry_dir] = child;
        let next = self.next_on_outer_face(child, root);
        Circ {
            current: next,
            prev: child,
        }
    }

    /// Embeds a short-circuit edge from the `componentRoot` to the current node of the
    /// `circulator`. Changes the outer face accordingly
    fn embed_short_circuit(&mut self, component_root: NodeId, entry_dir: usize, circ: &Circ) {
        let current = self.circ_current(circ);
        let prev = self.circ_prev(circ);
        let parent = self.parent_of(component_root);
        let short_circuit = self.new_short_circuit(current, parent);
        if entry_dir == 0 {
            self.pool
                .push_back(&mut self.arena[component_root].embedded, short_circuit);
            self.arena[component_root].outer[0] = current;
        } else {
            self.pool
                .push_front(&mut self.arena[component_root].embedded, short_circuit);
            self.arena[component_root].outer[1] = current;
        }
        self.node_embed_back_edge(current, short_circuit, prev);
        self.substitute(current, prev, component_root);
    }

    /// Either finds and returns a circulator to the node on the boundary of the component, which
    /// satisfies the `predicate` or returns a circulator to the `stop` node.
    fn select_on_outer_face(
        &self,
        pred: &dyn Fn(&Self, NodeId) -> bool,
        start: NodeId,
        stop: NodeId,
        dir: usize,
    ) -> Circ {
        let mut c = self.iterator(start, dir);
        let mut current = self.circ_next(&mut c);
        while current != stop && !pred(self, current) {
            current = self.circ_next(&mut c);
        }
        c
    }

    fn active_successor(&self, start: NodeId, v: NodeId, dir: usize) -> Circ {
        self.select_on_outer_face(&|s, n| s.is_active_wrt(n, v), start, start, dir)
    }

    fn externally_active_successor(
        &self,
        start: NodeId,
        stop: NodeId,
        v: NodeId,
        dir: usize,
    ) -> Circ {
        self.select_on_outer_face(&|s, n| s.is_externally_active_wrt(n, v), start, stop, dir)
    }

    fn component_root_of(&self, node: NodeId) -> NodeId {
        let c = self.select_on_outer_face(&|s, n| s.is_root(n), node, node, 0);
        self.circ_current(&c)
    }

    // ----- the planarity test -------------------------------------------------------------------

    /// The walkup procedure from the original paper. Identifies the pertinent subgraph of the graph
    /// by going up the dfs tree from the `start` node to the `end` node using the edge
    /// `edge`
    fn walk_up(&mut self, start: NodeId, end: NodeId, edge: EdgeId) {
        let visited = self.arena[end].dfs_index;
        self.arena[start].back_edge_flag = visited;
        self.arena[start].edge_to_embed = edge;
        let mut x = self.arena[start].outer[0];
        let mut y = self.arena[start].outer[1];
        let (mut x_prev, mut y_prev) = (start, start);
        self.arena[start].visited = visited;
        while x != end && !self.is_visited_wrt(x, end) && !self.is_visited_wrt(y, end) {
            self.arena[x].visited = visited;
            self.arena[y].visited = visited;
            let root = if self.is_root(x) {
                x
            } else if self.is_root(y) {
                y
            } else {
                NIL
            };
            if root != NIL {
                let pe = self.arena[root].parent_edge;
                let root_child = self.edges[pe].target;
                let new_start = self.edges[pe].source;
                if new_start != end {
                    if self.arena[root_child].lowpoint < self.arena[end].dfs_index {
                        let h = self
                            .pool
                            .push_back(&mut self.arena[new_start].pertinent, root);
                        self.arena[root].list_node = h;
                    } else {
                        let h = self
                            .pool
                            .push_front(&mut self.arena[new_start].pertinent, root);
                        self.arena[root].list_node = h;
                    }
                } else {
                    break;
                }
                self.arena[new_start].visited = visited;
                x_prev = new_start;
                y_prev = new_start;
                x = self.arena[new_start].outer[0];
                y = self.arena[new_start].outer[1];
            } else {
                let t = x;
                x = self.next_on_outer_face(x, x_prev);
                x_prev = t;
                let t = y;
                y = self.next_on_outer_face(y, y_prev);
                y_prev = t;
            }
        }
    }

    /// The walkdown procedure from the original paper. Either embeds all of the back edges in the
    /// subtree rooted at the child of the `componentRoot` or identifies the back edges which
    /// can be used to extract a Kuratowski subdivision. Iteratively traverses the tree of the
    /// biconnected component and descends only to the pertinent components. This procedure is also
    /// responsible for embedding short-circuit edges to make the algorithm run in linear time in the
    /// worst case.
    fn walk_down(&mut self, component_root: NodeId) {
        let mut entry_dir = 0;
        while entry_dir < 2 && self.stack.is_empty() {
            let mut current_dir = entry_dir;
            let mut circ = self.iterator(component_root, current_dir);
            let mut current = self.circ_next(&mut circ);
            while current != component_root {
                if self.has_back_edge_wrt(current, component_root) {
                    let child_prev = self.circ_prev(&circ);
                    while !self.stack.is_empty() {
                        self.merge_biconnected_component();
                    }
                    let edge = self.arena[current].edge_to_embed;
                    circ = self.embed_back_edge(component_root, entry_dir, edge, child_prev);
                }
                if !self.arena[current].pertinent.is_empty() {
                    let parent_entry_dir = current_dir;
                    let root = self.pool.front(&self.arena[current].pertinent);
                    let ccw = self.active_successor(root, component_root, 0);
                    let ccw_active = self.circ_current(&ccw);
                    let cw = self.active_successor(root, component_root, 1);
                    let cw_active = self.circ_current(&cw);
                    current_dir = if self.is_internally_active_wrt(ccw_active, component_root) {
                        0
                    } else if self.is_internally_active_wrt(cw_active, component_root) {
                        1
                    } else if self.is_pertinent_wrt(ccw_active, component_root) {
                        0
                    } else {
                        1
                    };
                    let parent_next = self.circ_next(&mut circ);
                    if current_dir == 0 {
                        self.stack.push(MergeInfo {
                            parent: current,
                            parent_next,
                            child: root,
                            child_prev: self.arena[root].outer[1],
                            v_in: parent_entry_dir,
                            v_out: current_dir,
                        });
                        current = ccw_active;
                        circ = ccw;
                        if !self.has_root_neighbor(cw_active) {
                            self.embed_short_circuit(root, 1, &cw);
                        }
                    } else {
                        self.stack.push(MergeInfo {
                            parent: current,
                            parent_next,
                            child: root,
                            child_prev: self.arena[root].outer[0],
                            v_in: parent_entry_dir,
                            v_out: current_dir,
                        });
                        current = cw_active;
                        circ = cw;
                        if !self.has_root_neighbor(ccw_active) {
                            self.embed_short_circuit(root, 0, &ccw);
                        }
                    }
                } else if self.is_inactive_wrt(current, component_root) {
                    current = self.circ_next(&mut circ);
                } else {
                    if !self.has_root_neighbor(current) && self.stack.is_empty() {
                        self.embed_short_circuit(component_root, entry_dir, &circ);
                    }
                    break;
                }
            }
            entry_dir += 1;
        }
    }

    /// Lazily tests the planarity of the graph. The implementation below is close to the code
    /// presented in the original paper
    fn lazy_test_planarity(&mut self) -> bool {
        if !self.tested {
            self.tested = true;
            self.orient();
            for i in (0..self.n).rev() {
                let current = self.nodes[i];
                for de in self.arena[current].down_edges.clone() {
                    let source = self.edges[de].source;
                    self.walk_up(source, current, de);
                }
                for te in self.arena[current].tree_edges.clone() {
                    let child = self.edges[te].target;
                    let root = self.arena[child].initial_component_root;
                    self.walk_down(root);
                }
                for &de in &self.arena[current].down_edges {
                    if !self.edges[de].embedded {
                        self.failed_v = current;
                        self.planar = false;
                        return false;
                    }
                }
            }
            self.planar = true;
        }
        self.planar
    }

    // ----- the embedding ------------------------------------------------------------------------

    /// Recursively cleans up the dfs tree rooted at the `dfsTreeRoot` my removing all the
    /// short-circuit edges and properly orienting other embedded edges
    fn clean_up_dfs(&mut self, dfs_tree_root: NodeId) {
        let mut stack = vec![(dfs_tree_root, 1i32)];
        while let Some((current, sign)) = stack.pop() {
            if sign < 0 {
                self.pool.reverse(&mut self.arena[current].embedded);
            }
            self.remove_short_circuit_edges(current);
            for node in self.separated_vec(current) {
                let pe = self.arena[node].parent_edge;
                self.edges[pe].sign = sign;
            }
            for &te in &self.arena[current].tree_edges {
                stack.push((self.edges[te].target, sign * self.edges[te].sign));
            }
        }
    }

    fn remove_short_circuit_edges(&mut self, x: NodeId) {
        let edges = &self.edges;
        self.pool
            .retain(&mut self.arena[x].embedded, |e| !edges[e].short_circuit);
    }

    /// Lazily computes a combinatorial embedding of the `graph` by removing all the
    /// short-circuit edges and properly orienting the edges around the nodes.
    fn lazy_compute_embedding(&mut self) {
        if self.embedding.is_some() {
            return;
        }
        for r in self.dfs_tree_roots.clone() {
            self.clean_up_dfs(r);
        }
        let mut map: IndexMap<G::Vertex, Vec<G::Edge>> = IndexMap::new();
        for node in self.nodes.clone() {
            for child in self.separated_vec(node) {
                let virtual_root = self.arena[child].initial_component_root;
                let mut moved =
                    std::mem::replace(&mut self.arena[virtual_root].embedded, List::EMPTY);
                self.pool
                    .splice(&mut self.arena[node].embedded, &mut moved, false);
            }
            let list: Vec<G::Edge> = self
                .emb_vec(node)
                .into_iter()
                .filter_map(|e| self.edges[e].graph_edge.clone())
                .collect();
            map.insert(self.arena[node].graph_vertex.clone().unwrap(), list);
        }
        self.embedding = Some(Embedding::new(map));
    }
}

// ----- Kuratowski subdivision ---------------------------------------------------------------------

type EdgeSet = HashSet<EdgeId>;

impl<'g, G: IncidenceGraph> BoyerMyrvoldPlanarityInspector<'g, G> {
    fn add_path_edges_from_edge(&self, edges: &mut EdgeSet, start_edge: EdgeId, stop: NodeId) {
        edges.insert(start_edge);
        let mut current = self.edges[start_edge].source;
        while current != stop {
            edges.insert(self.arena[current].parent_edge);
            current = self.parent_of(current);
        }
    }

    /// Adds the edges on the path from the `startEdge` up to the node `stop` to the set
    /// `edges`
    fn add_path_edges(&self, edges: &mut EdgeSet, start: NodeId, stop: NodeId) {
        if start != stop {
            self.add_path_edges_from_edge(edges, self.arena[start].parent_edge, stop);
        }
    }

    /// Searches a back edge which target has a height smaller than `heightMax`
    fn search_edge(
        &self,
        current: NodeId,
        is_needed: &dyn Fn(&Self, EdgeId) -> bool,
    ) -> Option<EdgeId> {
        for node in self.separated_vec(current) {
            if let Some(r) = self.search_subtree_dfs(node, is_needed) {
                return Some(r);
            }
        }
        self.arena[current]
            .back_edges
            .iter()
            .copied()
            .find(|&e| is_needed(self, e))
    }

    fn search_edge_below(
        &self,
        current: NodeId,
        height_max: usize,
        forbidden: Option<EdgeId>,
    ) -> Option<EdgeId> {
        self.search_edge(current, &|s, e| {
            forbidden != Some(e) && s.arena[s.edges[e].target].height < height_max
        })
    }

    /// Recursively searches all the subtree root at the node `start` to find an edge
    /// satisfying the `predicate`.
    fn search_subtree_dfs(
        &self,
        start: NodeId,
        is_needed: &dyn Fn(&Self, EdgeId) -> bool,
    ) -> Option<EdgeId> {
        let mut stack = vec![start];
        while let Some(current) = stack.pop() {
            for &e in &self.arena[current].back_edges {
                if is_needed(self, e) {
                    return Some(e);
                }
            }
            for &e in &self.arena[current].tree_edges {
                stack.push(self.edges[e].target);
            }
        }
        None
    }

    /// Returns the highest of the two input node, i.e. the node with the greater height
    fn highest(&self, a: NodeId, b: NodeId) -> NodeId {
        if self.arena[a].height > self.arena[b].height {
            a
        } else {
            b
        }
    }

    /// Returns the lowest of the two input node, i.e. the node with the least height.
    fn lowest(&self, a: NodeId, b: NodeId) -> NodeId {
        if self.arena[a].height < self.arena[b].height {
            a
        } else {
            b
        }
    }

    /// Iteratively sets a boundary height for the nodes on the outer face branch ending at the node
    /// `w`.
    fn set_boundary_depth(&mut self, component_root: NodeId, w: NodeId, dir: usize, delta: i32) {
        let mut c = self.iterator(component_root, dir);
        let mut current = self.circ_next(&mut c);
        let mut height = delta;
        while current != w {
            self.arena[current].boundary_height = height;
            height += delta;
            current = self.circ_next(&mut c);
        }
    }

    /// Clears the visited variable of all the nodes and component roots
    fn clear_visited(&mut self) {
        for &n in &self.nodes {
            self.arena[n].visited = 0;
        }
        for &n in &self.component_roots {
            self.arena[n].visited = 0;
        }
    }

    /// Generically searches a path from the `current` node to the first node satisfying the
    /// `isFinish` predicate consisting of all the nodes satisfying the `canGo`
    /// predicate. The key property of this method is that it searches the next edge on the path in
    /// the clockwise order starting from the previous edge. The edges of the resulting path are
    /// added to the `edges`.
    fn find_path_dfs(
        &mut self,
        start: NodeId,
        start_prev: EdgeId,
        finish: &Finish,
        edges: &mut Vec<EdgeId>,
    ) -> bool {
        let is_finish = |s: &Self, n: NodeId| match finish {
            Finish::NegativeBoundary => s.arena[n].boundary_height < 0,
            Finish::Node(v) => n == *v,
        };
        // (current, previous edge, backtrack)
        let mut stack = vec![(start, start_prev, false)];
        while let Some((current, prev_edge, backtrack)) = stack.pop() {
            if is_finish(self, current) {
                edges.push(prev_edge);
                edges.remove(0);
                return true;
            }
            if backtrack {
                edges.pop();
            } else {
                if self.arena[current].visited != 0 {
                    continue;
                }
                self.arena[current].visited = 1;
                stack.push((current, prev_edge, true));
                edges.push(prev_edge);
                // reverse circular iteration over the embedded edges, starting at `prev_edge`
                let embedded: Vec<EdgeId> = self.emb_vec(current);
                let pos = embedded
                    .iter()
                    .position(|&e| e == prev_edge)
                    .expect("previous edge is embedded at the node");
                let len = embedded.len();
                for k in 0..len {
                    let current_edge = embedded[(pos + len - k) % len];
                    let opposite = self.opposite(current_edge, current);
                    let can_go = !self.arena[opposite].marked && self.arena[opposite].visited == 0;
                    if !can_go && !is_finish(self, opposite) {
                        continue;
                    }
                    stack.push((opposite, current_edge, false));
                }
            }
        }
        false
    }

    /// Finds the highest obstructing path in the component rooted at `componentRoot`. See the
    /// original paper for the definition of the obstructing path. This method heavily relies on the
    /// fact that the method
    /// `find_path_dfs`
    /// chooses the edges in the clockwise order.
    fn find_highest_obstructing_path(&mut self, component_root: NodeId, w: NodeId) -> Vec<EdgeId> {
        self.clear_visited();
        let mut result = Vec::new();
        let mut c = self.iterator(component_root, 0);
        let mut current = self.circ_next(&mut c);
        while current != w {
            let first = self.emb_front(current);
            if self.find_path_dfs(current, first, &Finish::NegativeBoundary, &mut result) {
                return result;
            }
            current = self.circ_next(&mut c);
        }
        result
    }

    /// Finishes the Kuratowski subdivision extraction by constructing the desired subgraph
    fn finish(&mut self, subdivision: &EdgeSet) {
        let mut edge_subset = Vec::new();
        let mut vertex_subset: HashSet<G::Vertex> = HashSet::new();
        for &e in subdivision {
            let edge = &self.edges[e];
            if let Some(ge) = &edge.graph_edge {
                edge_subset.push(ge.clone());
            }
            for n in [edge.target, edge.source] {
                if let Some(v) = &self.arena[n].graph_vertex {
                    vertex_subset.insert(v.clone());
                }
            }
        }
        self.kuratowski = Some((vertex_subset.into_iter().collect(), edge_subset));
    }

    /// Adds the edges on the outer face of the component rooted at `componentRoot` to the set
    /// `edges`
    fn add_boundary_edges(&mut self, edges: &mut EdgeSet, component_root: NodeId) {
        let mut c = self.iterator(component_root, 0);
        loop {
            let edge = self.circ_edge_to_next(&c);
            let (s, t) = (self.edges[edge].source, self.edges[edge].target);
            self.arena[s].marked = true;
            self.arena[t].marked = true;
            edges.insert(edge);
            let current = self.circ_next(&mut c);
            if current == component_root {
                break;
            }
        }
    }

    /// Cleans up the dfs trees before the Kuratowski subdivision extraction phase
    fn kuratowski_clean_up(&mut self) {
        for r in self.dfs_tree_roots.clone() {
            self.clean_up_dfs(r);
        }
        for node in self.component_roots.clone() {
            if self.arena[node].outer[0] != NIL {
                self.remove_short_circuit_edges(node);
                self.fix_boundary_order(node);
            }
        }
    }

    /// Orient the connections on the outer face of the component rooted at `componentRoot`
    /// such that they are ordered
    fn fix_boundary_order(&mut self, component_root: NodeId) {
        if self.arena[component_root].embedded.size < 2 {
            return;
        }
        let component_parent = self.parent_of(component_root);
        let edge_to_next = self.emb_back(component_root);
        let edge_to_prev = self.emb_front(component_root);
        let next = self.opposite(edge_to_next, component_parent);
        let prev = self.opposite(edge_to_prev, component_parent);
        self.arena[component_root].outer = [next, prev];
        self.arena[next].outer[1] = component_root;
        self.arena[prev].outer[0] = component_root;
        let mut current = self.arena[component_root].outer[0];
        loop {
            let edge_to_next = self.emb_back(current);
            let edge_to_prev = self.emb_front(current);
            let next = self.opposite(edge_to_next, current);
            let prev = self.opposite(edge_to_prev, current);
            if prev != component_parent {
                self.arena[current].outer[1] = prev;
            }
            if next != component_parent {
                self.arena[current].outer[0] = next;
            }
            current = next;
            if current == component_parent {
                break;
            }
        }
    }

    /// Removes the edges from the outer face from the `start` node to the `end` node in
    /// the direction `dir` from the set `edges`
    fn remove_up(&self, start: NodeId, end: NodeId, dir: usize, edges: &mut EdgeSet) {
        if start == end {
            return;
        }
        let mut c = self.iterator(start, dir);
        loop {
            let edge = self.circ_edge_to_next(&c);
            edges.remove(&edge);
            let next = self.circ_next(&mut c);
            if next == end {
                break;
            }
        }
    }

    /// Effectively is a method for finding node `z` in the notations of the original paper.
    /// The search proceeds in the reverse order of the path from the `backEdge` to the node
    /// `w`
    fn next_on_path(&self, w: NodeId, back_edge: EdgeId) -> NodeId {
        let source = self.edges[back_edge].source;
        if source == w {
            return NIL;
        }
        let mut prev = source;
        let mut current = self.parent_of(source);
        while current != w {
            prev = current;
            current = self.parent_of(current);
        }
        prev
    }

    /// Finds a path from some intermediate nodes on the path represented by the list `path` to
    /// the node `v`. The path to `v` certainly doesn't exist if the list `path`
    /// has size 1, because we're looking for a path from some intermediate node
    fn find_path_to_v(&mut self, path: &[EdgeId], v: NodeId) -> Vec<EdgeId> {
        self.clear_visited();
        let mut i = 0;
        let mut current_edge = path[i];
        let mut current = if self.arena[self.edges[current_edge].source].boundary_height != 0 {
            self.edges[current_edge].target
        } else {
            self.edges[current_edge].source
        };
        let mut result = Vec::new();
        while i + 1 < path.len() {
            if self.find_path_dfs(current, current_edge, &Finish::Node(v), &mut result) {
                return result;
            }
            i += 1;
            current_edge = path[i];
            current = self.opposite(current_edge, current);
        }
        result
    }

    /// Checks whether the first node `a` is strictly higher than nodes `b` and `c`
    fn first_strictly_higher(&self, a: NodeId, b: NodeId, c: NodeId) -> bool {
        self.arena[a].height > self.arena[b].height && self.arena[a].height > self.arena[c].height
    }

    /// Checks whether the biconnected component rooted at `componentRoot` can be used to
    /// extract a Kuratowski subdivision. It can be used in the case there is one externally active
    /// node on each branch of the outer face and there is a pertinent node on the lower part of the
    /// outer face between these two externally active nodes.
    fn check_component_for_failed_edge(&self, component_root: NodeId, v: NodeId) -> Option<EdgeId> {
        let mut first_dir = self.externally_active_successor(component_root, component_root, v, 0);
        let first_dir_node = self.circ_current(&first_dir);
        let second_dir = self.externally_active_successor(component_root, component_root, v, 1);
        let second_dir_node = self.circ_current(&second_dir);
        if first_dir_node != component_root && first_dir_node != second_dir_node {
            let mut current = self.circ_next(&mut first_dir);
            while current != second_dir_node {
                if self.is_pertinent_wrt(current, v) {
                    return self.search_edge(current, &|s, e| {
                        s.edges[e].target == v && !s.edges[e].embedded
                    });
                }
                current = self.circ_next(&mut first_dir);
            }
        }
        None
    }

    /// Finds an unembedded back edge to `v`, which can be used to extract the Kuratowski
    /// subdivision. If the merge stack isn't empty, the last biconnected component processed by the
    /// walkdown can be used to find such an edge, because walkdown descended to that component
    /// (which means that component is pertinent) and couldn't reach a pertinent node. This can only
    /// happen by encountering externally active nodes on both branches of the traversal. Otherwise,
    /// be have look in all the child biconnected components to find an unembedded back edge. We're
    /// guided by the fact that an edge can not be embedded only in the case both traversals of the
    /// walkdown could reach all off the pertinent nodes. This in turn can happen only if both
    /// traversals get stuck on externally active nodes.
    ///
    /// **Note:** not every unembedded back edge can be used to extract a Kuratowski subdivision
    fn find_failed_edge(&self, v: NodeId) -> Option<EdgeId> {
        if self.stack.is_empty() {
            for child in self.separated_vec(v) {
                let component_root = self.arena[child].initial_component_root;
                if let Some(r) = self.check_component_for_failed_edge(component_root, v) {
                    return Some(r);
                }
            }
            None
        } else {
            let info = self.stack.last().unwrap();
            self.check_component_for_failed_edge(info.child, v)
        }
    }

    fn lazy_extract_kuratowski(&mut self) {
        if self.kuratowski.is_some() {
            return;
        }
        self.kuratowski_clean_up();
        let mut subdivision: EdgeSet = HashSet::new();
        let failed_edge = self
            .find_failed_edge(self.failed_v)
            .expect("a failed edge exists");
        let v = self.edges[failed_edge].target;
        let mut w = self.edges[failed_edge].source;
        let (mut x, mut y);
        let mut component_root;
        loop {
            component_root = self.component_root_of(w);
            let cx = self.externally_active_successor(w, component_root, v, 1);
            x = self.circ_current(&cx);
            let cy = self.externally_active_successor(w, component_root, v, 0);
            y = self.circ_current(&cy);
            if self.is_root(x) {
                w = self.parent_of(x);
            } else if self.is_root(y) {
                w = self.parent_of(y);
            } else {
                component_root = self.component_root_of(w);
                break;
            }
        }
        let v_height = self.arena[v].height;
        let x_back_edge = self
            .search_edge_below(x, v_height, None)
            .expect("x has a back edge");
        let y_back_edge = self
            .search_edge_below(y, v_height, None)
            .expect("y has a back edge");
        let (xt, yt) = (
            self.edges[x_back_edge].target,
            self.edges[y_back_edge].target,
        );
        let back_lower = self.lowest(xt, yt);
        let back_higher = self.highest(xt, yt);
        self.add_path_edges_from_edge(&mut subdivision, x_back_edge, x);
        self.add_path_edges_from_edge(&mut subdivision, y_back_edge, y);
        self.add_boundary_edges(&mut subdivision, component_root);

        if self.parent_of(component_root) != v {
            let p = self.parent_of(component_root);
            self.add_path_edges(&mut subdivision, p, back_lower);
            self.add_path_edges_from_edge(&mut subdivision, failed_edge, w);
            return self.finish(&subdivision);
        }

        let z = self.next_on_path(w, failed_edge);
        let mut back_edge = None;
        if z != NIL {
            back_edge = self.search_subtree_dfs(z, &|s, e| {
                s.arena[s.edges[e].target].height < v_height && e != failed_edge
            });
        }
        if let Some(back_edge) = back_edge {
            self.add_path_edges_from_edge(&mut subdivision, back_edge, w);
            self.add_path_edges_from_edge(&mut subdivision, failed_edge, w);
            let bt = self.edges[back_edge].target;
            let highest = self.highest(xt, self.highest(yt, bt));
            let lowest = self.lowest(xt, self.lowest(yt, bt));
            self.add_path_edges(&mut subdivision, highest, lowest);
            return self.finish(&subdivision);
        }

        self.set_boundary_depth(component_root, w, 0, 1);
        self.set_boundary_depth(component_root, w, 1, -1);
        let path = self.find_highest_obstructing_path(component_root, w);
        let first_edge = path[0];
        let last_edge = *path.last().unwrap();
        let first_node = if self.arena[self.edges[first_edge].source].boundary_height > 0 {
            self.edges[first_edge].source
        } else {
            self.edges[first_edge].target
        };
        let last_node = if self.arena[self.edges[last_edge].source].boundary_height < 0 {
            self.edges[last_edge].source
        } else {
            self.edges[last_edge].target
        };
        let (xb, yb) = (self.arena[x].boundary_height, self.arena[y].boundary_height);
        if self.arena[first_node].boundary_height < xb || self.arena[last_node].boundary_height > yb
        {
            if self.arena[last_node].boundary_height > yb {
                let remove_start = if self.arena[first_node].boundary_height < xb {
                    first_node
                } else {
                    x
                };
                self.remove_up(remove_start, component_root, 1, &mut subdivision);
            } else {
                self.remove_up(y, component_root, 0, &mut subdivision);
            }
            self.add_path_edges_from_edge(&mut subdivision, failed_edge, w);
            subdivision.extend(path.iter().copied());
            self.add_path_edges(&mut subdivision, v, back_lower);
            return self.finish(&subdivision);
        }

        for &e in &path {
            let (s, t) = (self.edges[e].source, self.edges[e].target);
            self.arena[s].marked = true;
            self.arena[t].marked = true;
        }
        let path_to_v = self.find_path_to_v(&path, v);
        if !path_to_v.is_empty() {
            self.remove_up(x, component_root, 1, &mut subdivision);
            self.remove_up(y, component_root, 0, &mut subdivision);
            subdivision.extend(path.iter().copied());
            subdivision.extend(path_to_v.iter().copied());
            self.add_path_edges(&mut subdivision, v, back_lower);
            self.add_path_edges_from_edge(&mut subdivision, failed_edge, w);
            return self.finish(&subdivision);
        }

        let externally_active = self
            .search_edge_below(w, v_height, Some(failed_edge))
            .expect("w has an externally active back edge");
        self.add_path_edges_from_edge(&mut subdivision, externally_active, w);
        let et = self.edges[externally_active].target;
        if self.first_strictly_higher(et, xt, yt) {
            let p = self.parent_of(component_root);
            self.add_path_edges(&mut subdivision, p, back_lower);
        } else if self.first_strictly_higher(xt, yt, et) {
            self.remove_up(component_root, x, 0, &mut subdivision);
            self.remove_up(w, last_node, 0, &mut subdivision);
            subdivision.extend(path.iter().copied());
            self.add_path_edges_from_edge(&mut subdivision, failed_edge, w);
            let low = self.lowest(back_lower, et);
            self.add_path_edges(&mut subdivision, v, low);
        } else if self.first_strictly_higher(yt, xt, et) {
            self.remove_up(y, component_root, 0, &mut subdivision);
            self.remove_up(first_node, w, 0, &mut subdivision);
            subdivision.extend(path.iter().copied());
            self.add_path_edges_from_edge(&mut subdivision, failed_edge, w);
            let low = self.lowest(back_lower, et);
            self.add_path_edges(&mut subdivision, v, low);
        } else if self.arena[first_node].boundary_height > xb {
            self.remove_up(w, last_node, 0, &mut subdivision);
            subdivision.extend(path.iter().copied());
            self.add_path_edges_from_edge(&mut subdivision, failed_edge, w);
            let (hi, lo) = (self.highest(back_higher, et), self.lowest(back_lower, et));
            self.add_path_edges(&mut subdivision, hi, lo);
        } else if self.arena[last_node].boundary_height < yb {
            self.remove_up(first_node, w, 0, &mut subdivision);
            subdivision.extend(path.iter().copied());
            self.add_path_edges_from_edge(&mut subdivision, failed_edge, w);
            let (hi, lo) = (self.highest(back_higher, et), self.lowest(back_lower, et));
            self.add_path_edges(&mut subdivision, hi, lo);
        } else {
            subdivision.extend(path.iter().copied());
            let low = self.lowest(back_lower, et);
            self.add_path_edges(&mut subdivision, v, low);
            self.add_path_edges_from_edge(&mut subdivision, failed_edge, w);
        }
        self.finish(&subdivision);
    }
}
