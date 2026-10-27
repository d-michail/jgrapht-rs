//! Connected components, strong connectivity, biconnectivity and condensation.

use super::interfaces::StrongConnectivityAlgorithm;
use super::util::Indexed;
use crate::event::GraphListener;
use crate::graph::{DirectedGraph, Graph, GraphExt};
use crate::traverse::DfsPostOrder;
use crate::view::AsSubgraph;
use crate::view::EdgeReversed;
use crate::{DiGraph, GraphMut, GraphMutExt};
use crate::{GraphError, Result};
use indexmap::IndexSet;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet, VecDeque};
use std::hash::Hash;
use std::rc::Rc;

pub use super::tree_connectivity::TreeDynamicConnectivity;

/// Connected components (weak components for directed graphs), in vertex order.
pub fn connected_components<G: Graph>(g: &G) -> Vec<Vec<G::Vertex>> {
    let ix = Indexed::new(g);
    let n = ix.n();
    let mut comp = vec![usize::MAX; n];
    let mut out: Vec<Vec<usize>> = Vec::new();
    for s in 0..n {
        if comp[s] != usize::MAX {
            continue;
        }
        let c = out.len();
        comp[s] = c;
        let mut members = vec![s];
        let mut stack = vec![s];
        while let Some(v) = stack.pop() {
            for &(u, _) in ix.out[v].iter().chain(&ix.inn[v]) {
                if comp[u] == usize::MAX {
                    comp[u] = c;
                    members.push(u);
                    stack.push(u);
                }
            }
        }
        members.sort_unstable();
        out.push(members);
    }
    out.iter().map(|m| ix.vertices_of(m)).collect()
}

/// The (weak) component containing `v`.
pub fn connected_component_of<G: Graph>(g: &G, v: &G::Vertex) -> Option<Vec<G::Vertex>> {
    connected_components(g).into_iter().find(|c| c.contains(v))
}

/// Strongly connected components (Tarjan, iterative). Components come out in reverse
/// topological order of the condensation.
pub fn strongly_connected_components<G: DirectedGraph>(g: &G) -> Vec<Vec<G::Vertex>> {
    let ix = Indexed::new(g);
    let n = ix.n();
    let (mut index, mut low) = (vec![usize::MAX; n], vec![0; n]);
    let (mut on, mut stack, mut next) = (vec![false; n], Vec::new(), 0);
    let mut comps = Vec::new();
    for root in 0..n {
        if index[root] != usize::MAX {
            continue;
        }
        let mut call = vec![(root, 0usize)];
        index[root] = next;
        low[root] = next;
        next += 1;
        stack.push(root);
        on[root] = true;
        while let Some(&mut (v, ref mut i)) = call.last_mut() {
            if *i < ix.out[v].len() {
                let u = ix.out[v][*i].0;
                *i += 1;
                if index[u] == usize::MAX {
                    index[u] = next;
                    low[u] = next;
                    next += 1;
                    stack.push(u);
                    on[u] = true;
                    call.push((u, 0));
                } else if on[u] {
                    low[v] = low[v].min(index[u]);
                }
            } else {
                call.pop();
                if let Some(&(p, _)) = call.last() {
                    low[p] = low[p].min(low[v]);
                }
                if low[v] == index[v] {
                    let mut c = Vec::new();
                    loop {
                        let u = stack.pop().unwrap();
                        on[u] = false;
                        c.push(u);
                        if u == v {
                            break;
                        }
                    }
                    c.sort_unstable();
                    comps.push(ix.vertices_of(&c));
                }
            }
        }
    }
    comps
}

/// Strongly connected components via Kosaraju–Sharir (two DFS passes).
pub fn strongly_connected_components_kosaraju<G: DirectedGraph>(g: &G) -> Vec<Vec<G::Vertex>> {
    let order: Vec<_> = DfsPostOrder::all(g).collect();
    let rev = EdgeReversed(g);
    let mut seen: HashSet<&G::Vertex> = HashSet::new();
    let mut comps = Vec::new();
    for &root in order.iter().rev() {
        if seen.contains(root) {
            continue;
        }
        let mut comp = Vec::new();
        let mut stack = vec![root];
        seen.insert(root);
        while let Some(v) = stack.pop() {
            comp.push(v.clone());
            for e in rev.edges_of(v) {
                let n = if rev.edge_source(e) == Some(v) {
                    rev.edge_target(e)
                } else {
                    None
                };
                if let Some(n) = n {
                    if seen.insert(n) {
                        stack.push(n);
                    }
                }
            }
        }
        comps.push(comp);
    }
    comps
}

pub fn is_strongly_connected<G: DirectedGraph>(g: &G) -> bool {
    g.vertex_count() > 0 && strongly_connected_components(g).len() == 1
}

/// Condensation: one vertex per strongly connected component (its index in `components`),
/// with an edge for every pair of components joined by at least one edge.
pub fn condensation<G: DirectedGraph>(g: &G) -> (DiGraph<usize>, Vec<Vec<G::Vertex>>) {
    let comps = strongly_connected_components(g);
    let comp_of: std::collections::HashMap<&G::Vertex, usize> = comps
        .iter()
        .enumerate()
        .flat_map(|(i, c)| c.iter().map(move |v| (v, i)))
        .collect();
    let mut dag = DiGraph::<usize>::simple();
    dag.add_vertices(0..comps.len());
    for e in g.edges() {
        let (a, b) = (
            comp_of[g.edge_source(e).unwrap()],
            comp_of[g.edge_target(e).unwrap()],
        );
        if a != b {
            let _ = dag.add_edge(&a, &b);
        }
    }
    (dag, comps)
}

/// Result of a biconnectivity analysis of an undirected graph.
#[derive(Debug, Clone)]
pub struct Biconnectivity<V, E> {
    pub articulation_points: Vec<V>,
    pub bridges: Vec<E>,
    /// Each block (biconnected component) as its set of edges; isolated vertices have none.
    pub blocks: Vec<Vec<E>>,
}

/// Articulation points, bridges and blocks of an undirected graph (edge directions ignored).
pub fn biconnectivity<G: Graph>(g: &G) -> Biconnectivity<G::Vertex, G::Edge> {
    let ix = Indexed::new(g);
    let n = ix.n();
    let mut adj: Vec<Vec<(usize, usize)>> = vec![Vec::new(); n];
    for (k, &(s, t)) in ix.ends.iter().enumerate() {
        if s != t {
            adj[s].push((t, k));
            adj[t].push((s, k));
        }
    }
    let (mut disc, mut low) = (vec![usize::MAX; n], vec![0usize; n]);
    let (mut art, mut bridges, mut blocks) = (vec![false; n], Vec::new(), Vec::new());
    let (mut time, mut estack) = (0, Vec::<usize>::new());
    for root in 0..n {
        if disc[root] != usize::MAX {
            continue;
        }
        disc[root] = time;
        low[root] = time;
        time += 1;
        let mut root_children = 0;
        let mut call = vec![(root, usize::MAX, 0usize)]; // vertex, parent edge, next adj index
        while let Some(&mut (v, pe, ref mut i)) = call.last_mut() {
            if *i < adj[v].len() {
                let (u, k) = adj[v][*i];
                *i += 1;
                if k == pe {
                    continue;
                }
                if disc[u] == usize::MAX {
                    disc[u] = time;
                    low[u] = time;
                    time += 1;
                    estack.push(k);
                    call.push((u, k, 0));
                } else if disc[u] < disc[v] {
                    low[v] = low[v].min(disc[u]);
                    estack.push(k);
                }
            } else {
                call.pop();
                if let Some(&(p, _, _)) = call.last() {
                    low[p] = low[p].min(low[v]);
                    if low[v] >= disc[p] {
                        if p == root {
                            root_children += 1;
                        } else {
                            art[p] = true;
                        }
                        let mut block = Vec::new();
                        while let Some(k) = estack.pop() {
                            block.push(k);
                            if k == pe {
                                break;
                            }
                        }
                        if low[v] > disc[p] {
                            bridges.push(pe);
                        }
                        blocks.push(block);
                    }
                }
            }
        }
        if root_children > 1 {
            art[root] = true;
        }
    }
    Biconnectivity {
        articulation_points: (0..n).filter(|&i| art[i]).map(|i| ix.vertex(i)).collect(),
        bridges: bridges.into_iter().map(|k| ix.es[k].clone()).collect(),
        blocks: blocks
            .into_iter()
            .map(|b| b.into_iter().map(|k| ix.es[k].clone()).collect())
            .collect(),
    }
}

/// No articulation points and connected (and at least 3 vertices, or a single edge).
pub fn is_biconnected<G: Graph>(g: &G) -> bool {
    crate::graph::props::is_connected(g) && biconnectivity(g).articulation_points.is_empty()
}

/// Block–cut tree of an undirected graph: one node per block and per articulation point, with
/// an edge between a cut vertex and every block containing it.
#[derive(Debug, Clone)]
pub struct BlockCutTree<V> {
    /// Vertices of each block.
    pub blocks: Vec<Vec<V>>,
    pub cut_vertices: Vec<V>,
    /// `(block index, cut vertex index)` pairs.
    pub edges: Vec<(usize, usize)>,
}

pub fn block_cut_tree<G: Graph>(g: &G) -> BlockCutTree<G::Vertex> {
    let bc = biconnectivity(g);
    let mut blocks: Vec<Vec<G::Vertex>> = Vec::new();
    for b in &bc.blocks {
        let mut vs: Vec<G::Vertex> = Vec::new();
        for e in b {
            for v in [g.edge_source(e).unwrap(), g.edge_target(e).unwrap()] {
                if !vs.contains(v) {
                    vs.push(v.clone());
                }
            }
        }
        blocks.push(vs);
    }
    // isolated vertices form singleton blocks
    for v in g.vertices() {
        if g.edges_of(v).next().is_none() {
            blocks.push(vec![v.clone()]);
        }
    }
    let mut edges = Vec::new();
    for (ci, c) in bc.articulation_points.iter().enumerate() {
        for (bi, b) in blocks.iter().enumerate() {
            if b.contains(c) {
                edges.push((bi, ci));
            }
        }
    }
    BlockCutTree {
        blocks,
        cut_vertices: bc.articulation_points,
        edges,
    }
}

// ---------------------------------------------------------------------------------------------
// ConnectivityInspector
// ---------------------------------------------------------------------------------------------

/// Connected sets of an undirected graph, or the weakly connected sets of a directed one
/// (`ConnectivityInspector`).
///
/// The inspector does not hold the graph; every query takes it. To keep results current while the
/// graph changes, share the inspector as `Rc<RefCell<ConnectivityInspector<V>>>` and register that
/// as a listener of a [`Listenable`](crate::event::Listenable) graph.
#[derive(Debug, Clone)]
pub struct ConnectivityInspector<V: Eq + Hash> {
    /// `None` until the full computation ran; removed sets leave a `None` slot behind.
    connected_sets: Option<Vec<Option<HashSet<V>>>>,
    vertex_to_set: HashMap<V, usize>,
    /// Sets computed for single vertices before the full computation.
    partial: HashMap<V, HashSet<V>>,
}

impl<V: Eq + Hash + Clone> Default for ConnectivityInspector<V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<V: Eq + Hash + Clone> ConnectivityInspector<V> {
    pub fn new() -> Self {
        ConnectivityInspector {
            connected_sets: None,
            vertex_to_set: HashMap::new(),
            partial: HashMap::new(),
        }
    }

    /// An inspector that can be registered as a graph listener and queried afterwards.
    pub fn shared() -> Rc<RefCell<Self>> {
        Rc::new(RefCell::new(Self::new()))
    }

    fn init(&mut self) {
        self.connected_sets = None;
        self.vertex_to_set.clear();
        self.partial.clear();
    }

    fn lazy_find<G: Graph<Vertex = V>>(&mut self, g: &G) -> &Vec<Option<HashSet<V>>> {
        if self.connected_sets.is_none() {
            let mut sets: Vec<Option<HashSet<V>>> = Vec::new();
            self.vertex_to_set.clear();
            for s in g.vertices() {
                if self.vertex_to_set.contains_key(s) {
                    continue;
                }
                let idx = sets.len();
                let mut set = HashSet::new();
                for v in bfs_undirected(g, s) {
                    self.vertex_to_set.insert(v.clone(), idx);
                    set.insert(v);
                }
                sets.push(Some(set));
            }
            self.partial.clear();
            self.connected_sets = Some(sets);
        }
        self.connected_sets.as_ref().unwrap()
    }

    pub fn is_connected<G: Graph<Vertex = V>>(&mut self, g: &G) -> bool {
        self.lazy_find(g).iter().flatten().count() == 1
    }

    /// The connected set containing `v`.
    pub fn connected_set_of<G: Graph<Vertex = V>>(&mut self, g: &G, v: &V) -> Result<HashSet<V>> {
        if !g.contains_vertex(v) {
            return Err(GraphError::VertexNotFound);
        }
        if let (Some(sets), Some(&i)) = (&self.connected_sets, self.vertex_to_set.get(v)) {
            return Ok(sets[i].clone().unwrap());
        }
        if let Some(s) = self.partial.get(v) {
            return Ok(s.clone());
        }
        let set: HashSet<V> = bfs_undirected(g, v).into_iter().collect();
        self.partial.insert(v.clone(), set.clone());
        Ok(set)
    }

    pub fn connected_sets<G: Graph<Vertex = V>>(&mut self, g: &G) -> Vec<HashSet<V>> {
        self.lazy_find(g).iter().flatten().cloned().collect()
    }

    pub fn path_exists<G: Graph<Vertex = V>>(&mut self, g: &G, s: &V, t: &V) -> Result<bool> {
        Ok(self.connected_set_of(g, s)?.contains(t))
    }

    pub fn vertex_added(&mut self, v: &V) {
        if let Some(sets) = &mut self.connected_sets {
            self.vertex_to_set.insert(v.clone(), sets.len());
            sets.push(Some(HashSet::from([v.clone()])));
        }
    }

    pub fn vertex_removed(&mut self, _v: &V) {
        self.init();
    }

    pub fn edge_added(&mut self, source: &V, target: &V) {
        let Some(sets) = &mut self.connected_sets else {
            self.partial.clear();
            return;
        };
        let (Some(&a), Some(&b)) = (
            self.vertex_to_set.get(source),
            self.vertex_to_set.get(target),
        ) else {
            self.init();
            return;
        };
        if a != b {
            let sa = sets[a].take().unwrap();
            let sb = sets[b].take().unwrap();
            let mut merge = HashSet::with_capacity(sa.len() + sb.len());
            merge.extend(sa);
            merge.extend(sb);
            let idx = sets.len();
            for v in &merge {
                self.vertex_to_set.insert(v.clone(), idx);
            }
            sets.push(Some(merge));
        }
    }

    pub fn edge_removed(&mut self, _source: &V, _target: &V) {
        self.init();
    }
}

impl<V: Eq + Hash + Clone, E> GraphListener<V, E> for Rc<RefCell<ConnectivityInspector<V>>> {
    fn vertex_added(&mut self, v: &V) {
        self.borrow_mut().vertex_added(v);
    }
    fn vertex_removed(&mut self, v: &V) {
        self.borrow_mut().vertex_removed(v);
    }
    fn edge_added(&mut self, _: &E, source: &V, target: &V) {
        self.borrow_mut().edge_added(source, target);
    }
    fn edge_removed(&mut self, _: &E, source: &V, target: &V) {
        self.borrow_mut().edge_removed(source, target);
    }
}

/// Breadth-first search from `start` ignoring edge directions.
fn bfs_undirected<G: Graph>(g: &G, start: &G::Vertex) -> Vec<G::Vertex> {
    let mut seen: HashSet<&G::Vertex> = HashSet::from([start]);
    let mut order = vec![start];
    let mut head = 0;
    while head < order.len() {
        let v = order[head];
        head += 1;
        for e in g.edges_of(v) {
            if let Some(u) = g.opposite_vertex(e, v) {
                if seen.insert(u) {
                    order.push(u);
                }
            }
        }
    }
    order.into_iter().cloned().collect()
}

// ---------------------------------------------------------------------------------------------
// Strong connectivity: Gabow and Kosaraju
// ---------------------------------------------------------------------------------------------

/// Path-based strong component algorithm of Gabow (`GabowStrongConnectivityInspector`).
pub struct GabowStrongConnectivityInspector<'g, G: DirectedGraph> {
    graph: &'g G,
    sets: Vec<HashSet<G::Vertex>>,
}

impl<'g, G: DirectedGraph> GabowStrongConnectivityInspector<'g, G> {
    pub fn new(graph: &'g G) -> Self {
        let ix = Indexed::new(graph);
        let n = ix.n();
        let mut number = vec![0usize; n];
        let (mut stack_s, mut stack_b) = (Vec::<usize>::new(), Vec::<usize>::new());
        let mut c = n;
        let mut sets = Vec::new();
        for root in 0..n {
            if number[root] != 0 {
                continue;
            }
            stack_s.push(root);
            number[root] = stack_s.len();
            stack_b.push(root);
            let mut call = vec![(root, 0usize)];
            while let Some(&mut (v, ref mut i)) = call.last_mut() {
                if *i < ix.out[v].len() {
                    let w = ix.out[v][*i].0;
                    *i += 1;
                    if number[w] == 0 {
                        stack_s.push(w);
                        number[w] = stack_s.len();
                        stack_b.push(w);
                        call.push((w, 0));
                    } else {
                        while number[w] < number[*stack_b.last().unwrap()] {
                            stack_b.pop();
                        }
                    }
                } else {
                    call.pop();
                    if stack_b.last() == Some(&v) {
                        stack_b.pop();
                        c += 1;
                        let size = stack_s.len() - number[v] + 1;
                        let mut scc = HashSet::with_capacity(size);
                        for _ in 0..size {
                            let r = stack_s.pop().unwrap();
                            scc.insert(ix.vertex(r));
                            number[r] = c;
                        }
                        sets.push(scc);
                    }
                }
            }
        }
        GabowStrongConnectivityInspector { graph, sets }
    }
}

impl<G: DirectedGraph> StrongConnectivityAlgorithm for GabowStrongConnectivityInspector<'_, G> {
    type G = G;
    fn graph(&self) -> &G {
        self.graph
    }
    fn strongly_connected_sets(&self) -> &[HashSet<G::Vertex>] {
        &self.sets
    }
}

/// Kosaraju–Sharir strong component algorithm (`KosarajuStrongConnectivityInspector`).
pub struct KosarajuStrongConnectivityInspector<'g, G: DirectedGraph> {
    graph: &'g G,
    sets: Vec<HashSet<G::Vertex>>,
}

impl<'g, G: DirectedGraph> KosarajuStrongConnectivityInspector<'g, G> {
    pub fn new(graph: &'g G) -> Self {
        enum Entry {
            Visit(usize),
            Finish(usize),
        }
        let ix = Indexed::new(graph);
        let n = ix.n();
        // one depth-first pass over `adj`; with `collect` it records the finishing order
        let dfs = |adj: &Vec<Vec<(usize, usize)>>,
                   discovered: &mut Vec<bool>,
                   start: usize,
                   mut visited: Option<&mut HashSet<G::Vertex>>,
                   ordered: &mut VecDeque<usize>| {
            let mut stack = vec![Entry::Visit(start)];
            while let Some(entry) = stack.pop() {
                match entry {
                    Entry::Visit(v) => {
                        if !discovered[v] {
                            discovered[v] = true;
                            if let Some(set) = visited.as_deref_mut() {
                                set.insert(ix.vertex(v));
                            }
                            stack.push(Entry::Finish(v));
                            for &(w, _) in &adj[v] {
                                if !discovered[w] {
                                    stack.push(Entry::Visit(w));
                                }
                            }
                        }
                    }
                    Entry::Finish(v) => {
                        if visited.is_none() {
                            ordered.push_front(v);
                        }
                    }
                }
            }
        };
        let mut ordered = VecDeque::new();
        let mut discovered = vec![false; n];
        for v in 0..n {
            if !discovered[v] {
                dfs(&ix.out, &mut discovered, v, None, &mut ordered);
            }
        }
        discovered.iter_mut().for_each(|d| *d = false);
        let mut sets = Vec::new();
        let mut unused = VecDeque::new();
        for &v in &ordered {
            if !discovered[v] {
                let mut set = HashSet::new();
                dfs(&ix.inn, &mut discovered, v, Some(&mut set), &mut unused);
                sets.push(set);
            }
        }
        KosarajuStrongConnectivityInspector { graph, sets }
    }
}

impl<G: DirectedGraph> StrongConnectivityAlgorithm for KosarajuStrongConnectivityInspector<'_, G> {
    type G = G;
    fn graph(&self) -> &G {
        self.graph
    }
    fn strongly_connected_sets(&self) -> &[HashSet<G::Vertex>] {
        &self.sets
    }
}

// ---------------------------------------------------------------------------------------------
// BiconnectivityInspector and BlockCutpointGraph
// ---------------------------------------------------------------------------------------------

/// Blocks, cutpoints, bridges and connected components of a graph (`BiconnectivityInspector`).
/// Edge directions are ignored. Results are computed when the inspector is constructed.
pub struct BiconnectivityInspector<'g, G: Graph> {
    graph: &'g G,
    blocks: Vec<HashSet<G::Vertex>>,
    cutpoints: IndexSet<G::Vertex>,
    bridges: IndexSet<G::Edge>,
    connected_sets: Vec<HashSet<G::Vertex>>,
    vertex_to_blocks: HashMap<G::Vertex, Vec<usize>>,
    vertex_to_component: HashMap<G::Vertex, usize>,
}

impl<'g, G: Graph> BiconnectivityInspector<'g, G> {
    pub fn new(graph: &'g G) -> Self {
        let vs: Vec<G::Vertex> = graph.vertices().cloned().collect();
        let n = vs.len();
        let idx: HashMap<&G::Vertex, usize> = vs.iter().enumerate().map(|(i, v)| (v, i)).collect();
        let adj: Vec<Vec<(G::Edge, usize)>> = vs
            .iter()
            .map(|v| {
                graph
                    .edges_of(v)
                    .map(|e| {
                        let nv = graph.opposite_vertex(e, v).unwrap();
                        (e.clone(), idx[nv])
                    })
                    .collect()
            })
            .collect();

        struct State<E> {
            time: i64,
            disc: Vec<i64>,
            stack: Vec<(usize, usize)>, // edge endpoints, `(source, target)`
            blocks: Vec<Vec<usize>>,
            cutpoints: Vec<usize>,
            bridges: Vec<E>,
        }
        fn build_block<E>(st: &mut State<E>, cut: i64) {
            let mut comp = Vec::new();
            while let Some(&(s, t)) = st.stack.last() {
                if st.disc[s] < cut && st.disc[t] < cut {
                    break;
                }
                st.stack.pop();
                comp.push(s);
                comp.push(t);
            }
            st.blocks.push(comp);
        }

        let mut st = State::<G::Edge> {
            time: 0,
            disc: vec![-1; n],
            stack: Vec::new(),
            blocks: Vec::new(),
            cutpoints: Vec::new(),
            bridges: Vec::new(),
        };
        let mut connected_sets: Vec<Vec<usize>> = Vec::new();
        for root in 0..n {
            if st.disc[root] != -1 {
                continue;
            }
            let mut connected = Vec::new();
            struct Frame {
                v: usize,
                parent: Option<usize>,
                low: i64,
                children: u32,
                pos: usize,
            }
            let enter = |st: &mut State<G::Edge>, connected: &mut Vec<usize>, v, parent| {
                st.time += 1;
                st.disc[v] = st.time;
                connected.push(v);
                Frame {
                    v,
                    parent,
                    low: st.time,
                    children: 0,
                    pos: 0,
                }
            };
            let mut call = vec![enter(&mut st, &mut connected, root, None)];
            loop {
                let top = call.len() - 1;
                let v = call[top].v;
                if call[top].pos < adj[v].len() {
                    let (ref edge, nv) = adj[v][call[top].pos];
                    call[top].pos += 1;
                    if st.disc[nv] == -1 {
                        call[top].children += 1;
                        let (s, t) = graph.edge_endpoints(edge).unwrap();
                        st.stack.push((idx[s], idx[t]));
                        let frame = enter(&mut st, &mut connected, nv, Some(v));
                        call.push(frame);
                    } else if st.disc[nv] < st.disc[v] && call[top].parent != Some(nv) {
                        let (s, t) = graph.edge_endpoints(edge).unwrap();
                        st.stack.push((idx[s], idx[t]));
                        call[top].low = call[top].low.min(st.disc[nv]);
                    }
                    continue;
                }
                // `v` is finished: report its low point to the parent
                let child = call.pop().unwrap();
                let Some(f) = call.last_mut() else { break };
                let low_nv = child.low;
                f.low = f.low.min(low_nv);
                if low_nv > st.disc[f.v] {
                    st.bridges.push(adj[f.v][f.pos - 1].0.clone());
                }
                // a non-root vertex is a cutpoint iff some child has low point >= its depth;
                // the root is one iff it has more than one child
                if (f.parent.is_some() && low_nv >= st.disc[f.v])
                    || (f.parent.is_none() && f.children > 1)
                {
                    st.cutpoints.push(f.v);
                    let cut = st.disc[child.v];
                    build_block(&mut st, cut);
                }
            }
            if !st.stack.is_empty() {
                build_block(&mut st, 0);
            }
            connected_sets.push(connected);
        }

        let mut blocks: Vec<HashSet<G::Vertex>> = Vec::new();
        for b in &st.blocks {
            let set: HashSet<G::Vertex> = b.iter().map(|&i| vs[i].clone()).collect();
            if !blocks.contains(&set) {
                blocks.push(set);
            }
        }
        let mut bridges: IndexSet<G::Edge> = st.bridges.into_iter().collect();
        if graph.graph_type().multiple_edges {
            bridges.retain(|e| {
                let (s, t) = graph.edge_endpoints(e).unwrap();
                graph.edges_between(s, t).count() <= 1
            });
        }
        let cutpoints: IndexSet<G::Vertex> = st.cutpoints.iter().map(|&i| vs[i].clone()).collect();
        let connected_sets: Vec<HashSet<G::Vertex>> = connected_sets
            .iter()
            .map(|c| c.iter().map(|&i| vs[i].clone()).collect())
            .collect();
        let mut vertex_to_blocks: HashMap<G::Vertex, Vec<usize>> =
            vs.iter().map(|v| (v.clone(), Vec::new())).collect();
        for (bi, b) in blocks.iter().enumerate() {
            for v in b {
                vertex_to_blocks.get_mut(v).unwrap().push(bi);
            }
        }
        let mut vertex_to_component = HashMap::new();
        for (ci, c) in connected_sets.iter().enumerate() {
            for v in c {
                vertex_to_component.insert(v.clone(), ci);
            }
        }
        BiconnectivityInspector {
            graph,
            blocks,
            cutpoints,
            bridges,
            connected_sets,
            vertex_to_blocks,
            vertex_to_component,
        }
    }

    pub fn cutpoints(&self) -> &IndexSet<G::Vertex> {
        &self.cutpoints
    }

    pub fn bridges(&self) -> &IndexSet<G::Edge> {
        &self.bridges
    }

    /// Vertex sets of the blocks; the block subgraph is the subgraph induced by its set.
    pub fn blocks(&self) -> &[HashSet<G::Vertex>] {
        &self.blocks
    }

    /// The blocks containing `v`; a cutpoint lies in several.
    pub fn blocks_of(&self, v: &G::Vertex) -> Result<Vec<&HashSet<G::Vertex>>> {
        let ix = self
            .vertex_to_blocks
            .get(v)
            .ok_or(GraphError::VertexNotFound)?;
        Ok(ix.iter().map(|&i| &self.blocks[i]).collect())
    }

    pub fn block_subgraphs(&self) -> Vec<AsSubgraph<&'g G>> {
        self.blocks
            .iter()
            .map(|b| AsSubgraph::induced(self.graph, b.iter().cloned()))
            .collect()
    }

    /// Vertex sets of the connected components.
    pub fn connected_components(&self) -> &[HashSet<G::Vertex>] {
        &self.connected_sets
    }

    pub fn connected_component_of(&self, v: &G::Vertex) -> Result<&HashSet<G::Vertex>> {
        let i = self
            .vertex_to_component
            .get(v)
            .ok_or(GraphError::VertexNotFound)?;
        Ok(&self.connected_sets[*i])
    }

    pub fn is_biconnected(&self) -> bool {
        self.graph.vertex_count() >= 2 && self.blocks.len() == 1
    }

    pub fn is_connected(&self) -> bool {
        self.connected_sets.len() == 1
    }
}

/// The block-cutpoint graph (`BlockCutpointGraph`): a forest with one node per block and one node
/// per cutpoint, where a cutpoint is adjacent to every block containing it.
///
/// Nodes are numbered: `0..blocks.len()` are the blocks, the rest the cutpoints.
#[derive(Debug, Clone)]
pub struct BlockCutpointGraph<V: Eq + Hash> {
    graph: crate::UGraph<usize>,
    nodes: Vec<HashSet<V>>,
    block_count: usize,
    cutpoints: IndexSet<V>,
    vertex_to_node: HashMap<V, usize>,
}

impl<V: Eq + Hash + Clone> BlockCutpointGraph<V> {
    pub fn new<G: Graph<Vertex = V>>(graph: &G) -> Self {
        let bi = BiconnectivityInspector::new(graph);
        let block_count = bi.blocks().len();
        let mut nodes: Vec<HashSet<V>> = bi.blocks().to_vec();
        let mut vertex_to_node = HashMap::new();
        for (i, b) in bi.blocks().iter().enumerate() {
            for v in b {
                vertex_to_node.insert(v.clone(), i);
            }
        }
        let mut g = crate::UGraph::<usize>::simple();
        g.add_vertices(0..block_count);
        let cutpoints = bi.cutpoints().clone();
        for c in &cutpoints {
            let node = nodes.len();
            nodes.push(HashSet::from([c.clone()]));
            vertex_to_node.insert(c.clone(), node);
            g.add_vertex(node);
            for (bi_idx, b) in bi.blocks().iter().enumerate() {
                if b.contains(c) {
                    let _ = g.add_edge(&node, &bi_idx);
                }
            }
        }
        BlockCutpointGraph {
            graph: g,
            nodes,
            block_count,
            cutpoints,
            vertex_to_node,
        }
    }

    /// The tree of nodes.
    pub fn graph(&self) -> &crate::UGraph<usize> {
        &self.graph
    }

    /// The vertices represented by a node: a block's vertices, or a single cutpoint.
    pub fn node(&self, node: usize) -> &HashSet<V> {
        &self.nodes[node]
    }

    /// The node of `v`: its singleton node if it is a cutpoint, else the block containing it.
    pub fn block_of(&self, v: &V) -> Option<usize> {
        self.vertex_to_node.get(v).copied()
    }

    pub fn block_nodes(&self) -> std::ops::Range<usize> {
        0..self.block_count
    }

    pub fn cutpoints(&self) -> &IndexSet<V> {
        &self.cutpoints
    }

    pub fn is_cutpoint(&self, v: &V) -> bool {
        self.cutpoints.contains(v)
    }
}

// ---------------------------------------------------------------------------------------------
// KConnectivityFlowAlgorithm
// ---------------------------------------------------------------------------------------------

/// Edge and vertex connectivity through maximum flows (`KConnectivityFlowAlgorithm`).
///
/// The maximum-flow routine is a parameter taking the (unit-capacity) graph and two vertices and
/// returning the flow value; the default is Edmonds–Karp.
pub struct KConnectivityFlowAlgorithm<'g, G: Graph> {
    graph: crate::view::AsUnweighted<&'g G>,
    vertex_network: crate::DiGraph<usize>,
    vertex_mapping: HashMap<G::Vertex, usize>,
    min_degree_vertex: Option<G::Vertex>,
    min_degree_value: usize,
    edge_flow: Box<dyn Fn(&crate::view::AsUnweighted<&'g G>, &G::Vertex, &G::Vertex) -> f64 + 'g>,
    vertex_flow: Box<dyn Fn(&crate::DiGraph<usize>, &usize, &usize) -> f64 + 'g>,
    vertex_connectivity: Option<usize>,
    edge_connectivity: Option<usize>,
}

impl<'g, G: Graph> KConnectivityFlowAlgorithm<'g, G> {
    pub fn new(graph: &'g G) -> Self {
        Self::with_flow(
            graph,
            |g, s, t| {
                super::flow::edmonds_karp(g, s, t)
                    .map(|f| f.value)
                    .unwrap_or(0.0)
            },
            |g, s, t| {
                super::flow::edmonds_karp(g, s, t)
                    .map(|f| f.value)
                    .unwrap_or(0.0)
            },
        )
    }

    pub fn with_flow(
        graph: &'g G,
        edge_flow: impl Fn(&crate::view::AsUnweighted<&'g G>, &G::Vertex, &G::Vertex) -> f64 + 'g,
        vertex_flow: impl Fn(&crate::DiGraph<usize>, &usize, &usize) -> f64 + 'g,
    ) -> Self {
        let mut network = crate::DiGraph::<usize>::simple();
        let mut vertex_mapping = HashMap::new();
        let mut counter = 0usize;
        for v in graph.vertices() {
            vertex_mapping.insert(v.clone(), counter);
            network.add_vertex(counter);
            network.add_vertex(counter + 1);
            let _ = network.add_edge(&counter, &(counter + 1));
            counter += 2;
        }
        for s in graph.vertices() {
            for t in successor_list(graph, s) {
                let _ = network.add_edge(&(vertex_mapping[s] + 1), &vertex_mapping[&t]);
            }
        }
        let mut min_degree = usize::MAX;
        let mut min_degree_vertex = None;
        for v in graph.vertices() {
            let d = graph.degree_of(v);
            if d < min_degree {
                min_degree = d;
                min_degree_vertex = Some(v.clone());
            }
        }
        KConnectivityFlowAlgorithm {
            graph: crate::view::AsUnweighted(graph),
            vertex_network: network,
            vertex_mapping,
            min_degree_vertex,
            min_degree_value: if min_degree == usize::MAX {
                0
            } else {
                min_degree
            },
            edge_flow: Box::new(edge_flow),
            vertex_flow: Box::new(vertex_flow),
            vertex_connectivity: None,
            edge_connectivity: None,
        }
    }

    pub fn edge_connectivity(&mut self) -> usize {
        if self.edge_connectivity.is_none() {
            self.edge_connectivity = Some(if self.graph.graph_type().directed {
                self.directed_edge_connectivity()
            } else {
                self.undirected_edge_connectivity()
            });
        }
        self.edge_connectivity.unwrap()
    }

    /// Vertex connectivity; an error for the empty graph.
    pub fn vertex_connectivity(&mut self) -> Result<usize> {
        if self.vertex_connectivity.is_none() {
            let m = self
                .min_degree_vertex
                .clone()
                .ok_or_else(|| GraphError::InvalidArgument("graph has no vertices".into()))?;
            let a = self.connectivity_from_non_neighbors(&m);
            let b = self.connectivity_from_neighbors(&m);
            self.vertex_connectivity = Some(a.min(b));
        }
        Ok(self.vertex_connectivity.unwrap())
    }

    pub fn is_k_edge_connected(&mut self, k: usize) -> bool {
        k <= self.edge_connectivity()
    }

    pub fn is_k_vertex_connected(&mut self, k: usize) -> Result<bool> {
        Ok(k <= self.vertex_connectivity()?)
    }

    fn local_edge(&self, s: &G::Vertex, t: &G::Vertex) -> usize {
        (self.edge_flow)(&self.graph, s, t) as usize
    }

    fn local_vertex(&self, s: &G::Vertex, t: &G::Vertex) -> usize {
        let (sid, tid) = (self.vertex_mapping[s], self.vertex_mapping[t]);
        (self.vertex_flow)(&self.vertex_network, &(sid + 1), &tid) as usize
    }

    fn connectivity_from_non_neighbors(&self, m: &G::Vertex) -> usize {
        let mut min = usize::MAX;
        for v in self.graph.vertices() {
            if v == m {
                continue;
            }
            if !self.graph.contains_edge_between(m, v) {
                min = min.min(self.local_vertex(m, v));
            }
            if !self.graph.contains_edge_between(v, m) {
                min = min.min(self.local_vertex(v, m));
            }
        }
        min
    }

    fn connectivity_from_neighbors(&self, m: &G::Vertex) -> usize {
        let nbrs = successor_list(&self.graph, m);
        let mut min = usize::MAX;
        for i in 0..nbrs.len() {
            for j in i + 1..nbrs.len() {
                let (a, b) = (&nbrs[i], &nbrs[j]);
                if !self.graph.contains_edge_between(a, b) {
                    min = min.min(self.local_vertex(a, b));
                }
                if !self.graph.contains_edge_between(b, a) {
                    min = min.min(self.local_vertex(b, a));
                }
            }
        }
        if min == usize::MAX {
            min = self.graph.vertex_count() - 1; // complete graph
        }
        min
    }

    fn undirected_edge_connectivity(&self) -> usize {
        let mut dominating: Vec<&G::Vertex> = Vec::new();
        let mut covered: HashSet<G::Vertex> = HashSet::new();
        for v in self.graph.vertices() {
            if !covered.contains(v) {
                dominating.push(v);
                covered.insert(v.clone());
                covered.extend(self.graph.neighbor_list_of(v));
            }
        }
        let Some((reference, rest)) = dominating.split_first() else {
            return 0;
        };
        let mut min = usize::MAX;
        for v in rest {
            min = min.min(self.local_edge(reference, v));
        }
        min.min(self.min_degree_value)
    }

    fn directed_edge_connectivity(&self) -> usize {
        let vs: Vec<&G::Vertex> = self.graph.vertices().collect();
        let mut min = usize::MAX;
        for i in 0..vs.len() {
            for j in i + 1..vs.len() {
                min = min.min(self.local_edge(vs[i], vs[j]));
            }
        }
        min
    }
}

/// Successors in a directed graph, neighbours in an undirected one (`Graphs.successorListOf`).
fn successor_list<G: Graph>(g: &G, v: &G::Vertex) -> Vec<G::Vertex> {
    g.out_edges_of(v)
        .filter_map(|e| g.opposite_vertex(e, v).cloned())
        .collect()
}
