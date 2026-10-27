use super::base::require_not_empty;
use crate::alg::connectivity::{BiconnectivityInspector, ConnectivityInspector};
use crate::error::{GraphError, Result};
use crate::graph::{Graph, GraphExt};
use crate::walk::GraphWalk;
use indexmap::IndexSet;
use std::collections::{HashMap, HashSet, VecDeque};

/// `HamiltonianPathSearchResult.Status`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchStatus {
    PathFound,
    ProvenAbsent,
    Aborted,
}

/// `HamiltonianPathSearchResult`: the outcome of a Hamiltonian path search.
#[derive(Debug, Clone)]
pub struct HamiltonianPathSearchResult<V, E> {
    status: SearchStatus,
    path: Option<GraphWalk<V, E>>,
    states_expanded: u64,
}

impl<V, E> HamiltonianPathSearchResult<V, E> {
    pub fn found(path: GraphWalk<V, E>, states_expanded: u64) -> Self {
        HamiltonianPathSearchResult {
            status: SearchStatus::PathFound,
            path: Some(path),
            states_expanded,
        }
    }

    pub fn proven_absent(states_expanded: u64) -> Self {
        HamiltonianPathSearchResult {
            status: SearchStatus::ProvenAbsent,
            path: None,
            states_expanded,
        }
    }

    pub fn aborted(states_expanded: u64) -> Self {
        HamiltonianPathSearchResult {
            status: SearchStatus::Aborted,
            path: None,
            states_expanded,
        }
    }

    pub fn status(&self) -> SearchStatus {
        self.status
    }

    pub fn path(&self) -> Option<&GraphWalk<V, E>> {
        self.path.as_ref()
    }

    pub fn states_expanded(&self) -> u64 {
        self.states_expanded
    }
}

/// `HamiltonianPathAlgorithm`.
pub trait HamiltonianPathAlgorithm<G: Graph> {
    fn get_path(&mut self, graph: &G) -> Result<HamiltonianPathSearchResult<G::Vertex, G::Edge>>;
}

fn require_directed_or_undirected<G: Graph>(graph: &G) -> Result<()> {
    if graph.graph_type().mixed {
        return Err(GraphError::InvalidArgument(
            "Graph must be directed or undirected".into(),
        ));
    }
    Ok(())
}

fn singleton_path<G: Graph>(graph: &G) -> GraphWalk<G::Vertex, G::Edge> {
    let only = graph.vertices().next().expect("one vertex").clone();
    GraphWalk::new(vec![only], Vec::new(), 0.0)
}

fn vertex_list_to_path<G: Graph>(
    vertices: Vec<G::Vertex>,
    graph: &G,
) -> Result<GraphWalk<G::Vertex, G::Edge>> {
    if vertices.len() == 1 {
        return Ok(GraphWalk::new(vertices, Vec::new(), 0.0));
    }
    let mut edges = Vec::with_capacity(vertices.len() - 1);
    let mut weight = 0.0;
    for pair in vertices.windows(2) {
        let e = graph
            .get_edge(&pair[0], &pair[1])
            .ok_or(GraphError::EdgeNotFound)?;
        weight += graph.edge_weight(e).unwrap_or(1.0);
        edges.push(e.clone());
    }
    Ok(GraphWalk::new(vertices, edges, weight))
}

/// Distinct neighbours (loops dropped) in first-seen order.
fn build_adjacency<G: Graph>(
    graph: &G,
    index_list: &[G::Vertex],
    vertex_map: &HashMap<G::Vertex, usize>,
    directed: bool,
) -> Vec<Vec<usize>> {
    index_list
        .iter()
        .map(|u_vertex| {
            let mut neighbours: IndexSet<usize> = IndexSet::new();
            let others: Vec<&G::Vertex> = if directed {
                graph
                    .out_edges_of(u_vertex)
                    .map(|e| graph.edge_target(e).expect("edge of the graph"))
                    .collect()
            } else {
                graph
                    .edges_of(u_vertex)
                    .map(|e| {
                        graph
                            .opposite_vertex(e, u_vertex)
                            .expect("edge of the graph")
                    })
                    .collect()
            };
            for other in others {
                if other == u_vertex {
                    continue;
                }
                neighbours.insert(vertex_map[other]);
            }
            neighbours.into_iter().collect()
        })
        .collect()
}

/// `DagHamiltonianPath`: a Hamiltonian path of a DAG exists iff its topological order is a path.
#[derive(Default)]
pub struct DagHamiltonianPath;

impl DagHamiltonianPath {
    pub fn new() -> Self {
        DagHamiltonianPath
    }
}

impl<G: Graph> HamiltonianPathAlgorithm<G> for DagHamiltonianPath {
    fn get_path(&mut self, graph: &G) -> Result<HamiltonianPathSearchResult<G::Vertex, G::Edge>> {
        if !graph.graph_type().directed {
            return Err(GraphError::InvalidArgument("Graph must be directed".into()));
        }
        require_not_empty(graph)?;
        let n = graph.vertex_count();
        if n == 1 {
            return Ok(HamiltonianPathSearchResult::found(singleton_path(graph), 0));
        }
        // Kahn's algorithm
        let index_list: Vec<G::Vertex> = graph.vertices().cloned().collect();
        let vertex_map: HashMap<G::Vertex, usize> = index_list
            .iter()
            .enumerate()
            .map(|(i, v)| (v.clone(), i))
            .collect();
        let mut in_degree: Vec<usize> = index_list.iter().map(|v| graph.in_degree_of(v)).collect();
        let mut queue: VecDeque<usize> = (0..n).filter(|&i| in_degree[i] == 0).collect();
        let mut topo = Vec::with_capacity(n);
        while let Some(u) = queue.pop_front() {
            topo.push(u);
            for e in graph.out_edges_of(&index_list[u]) {
                let v = vertex_map[graph.edge_target(e).expect("edge of the graph")];
                in_degree[v] -= 1;
                if in_degree[v] == 0 {
                    queue.push_back(v);
                }
            }
        }
        if topo.len() != n {
            return Err(GraphError::InvalidArgument(
                "DagHamiltonianPath requires an acyclic graph; cycle detected. Use \
                 BacktrackingHamiltonianPath or HeldKarpHamiltonianPath instead."
                    .into(),
            ));
        }
        let mut position = vec![0usize; n];
        for (i, &v) in topo.iter().enumerate() {
            position[v] = i;
        }
        let mut longest = vec![1usize; n];
        let mut predecessor = vec![usize::MAX; n];
        let mut best_end = 0;
        let mut best_length = 1;
        for i in 0..n {
            let v = &index_list[topo[i]];
            for e in graph.in_edges_of(v) {
                let u = position[vertex_map[graph.edge_source(e).expect("edge of the graph")]];
                if longest[u] + 1 > longest[i] {
                    longest[i] = longest[u] + 1;
                    predecessor[i] = u;
                }
            }
            if longest[i] > best_length {
                best_length = longest[i];
                best_end = i;
            }
        }
        if best_length != n {
            return Ok(HamiltonianPathSearchResult::proven_absent(n as u64));
        }
        let mut reversed = Vec::with_capacity(n);
        let mut cur = best_end;
        while cur != usize::MAX {
            reversed.push(index_list[topo[cur]].clone());
            cur = predecessor[cur];
        }
        reversed.reverse();
        Ok(HamiltonianPathSearchResult::found(
            vertex_list_to_path(reversed, graph)?,
            n as u64,
        ))
    }
}

/// `HeldKarpHamiltonianPath`: bitmask dynamic programming, `O(2^n n²)`; a hard limit of 30
/// vertices.
pub struct HeldKarpHamiltonianPath {
    max_vertices: usize,
    states_expanded: u64,
}

impl HeldKarpHamiltonianPath {
    pub const DEFAULT_MAX_VERTICES: usize = 20;
    pub const HARD_MAX_VERTICES: usize = 30;
    const UNVISITED: i8 = -1;
    const START_SENTINEL: i8 = -2;

    pub fn new() -> Self {
        HeldKarpHamiltonianPath {
            max_vertices: Self::DEFAULT_MAX_VERTICES,
            states_expanded: 0,
        }
    }

    pub fn with_max_vertices(max_vertices: usize) -> Result<Self> {
        if max_vertices < 1 {
            return Err(GraphError::InvalidArgument(format!(
                "maxVertices must be at least 1, got {max_vertices}"
            )));
        }
        if max_vertices > Self::HARD_MAX_VERTICES {
            return Err(GraphError::InvalidArgument(format!(
                "maxVertices must be at most {} (the int-bitmask hard limit); got {max_vertices}",
                Self::HARD_MAX_VERTICES
            )));
        }
        Ok(HeldKarpHamiltonianPath {
            max_vertices,
            states_expanded: 0,
        })
    }

    pub fn max_vertices(&self) -> usize {
        self.max_vertices
    }

    /// The number of states expanded by the last search.
    pub fn states_expanded(&self) -> u64 {
        self.states_expanded
    }
}

impl Default for HeldKarpHamiltonianPath {
    fn default() -> Self {
        Self::new()
    }
}

impl<G: Graph> HamiltonianPathAlgorithm<G> for HeldKarpHamiltonianPath {
    fn get_path(&mut self, graph: &G) -> Result<HamiltonianPathSearchResult<G::Vertex, G::Edge>> {
        require_directed_or_undirected(graph)?;
        self.states_expanded = 0;
        require_not_empty(graph)?;
        let n = graph.vertex_count();
        if n == 1 {
            return Ok(HamiltonianPathSearchResult::found(singleton_path(graph), 0));
        }
        if n > self.max_vertices {
            return Err(GraphError::InvalidArgument(format!(
                "HeldKarpHamiltonianPath supports at most {} vertices; got {n}. Use \
                 BacktrackingHamiltonianPath for larger graphs or construct this class with a \
                 higher maxVertices ceiling.",
                self.max_vertices
            )));
        }
        let index_list: Vec<G::Vertex> = graph.vertices().cloned().collect();
        let vertex_map: HashMap<G::Vertex, usize> = index_list
            .iter()
            .enumerate()
            .map(|(i, v)| (v.clone(), i))
            .collect();
        let directed = graph.graph_type().directed;
        let adjacency = build_adjacency(graph, &index_list, &vertex_map, directed);
        let full_mask = (1usize << n) - 1;
        let mut pred = vec![vec![Self::UNVISITED; n]; 1 << n];
        for v in 0..n {
            pred[1 << v][v] = Self::START_SENTINEL;
            self.states_expanded += 1;
        }
        for mask in 1..(1usize << n) {
            for v in 0..n {
                if (mask >> v) & 1 == 0 || pred[mask][v] == Self::UNVISITED {
                    continue;
                }
                for &u in &adjacency[v] {
                    if (mask >> u) & 1 != 0 {
                        continue;
                    }
                    let new_mask = mask | (1 << u);
                    if pred[new_mask][u] == Self::UNVISITED {
                        pred[new_mask][u] = v as i8;
                        self.states_expanded += 1;
                    }
                }
            }
        }
        let Some(end_vertex) = (0..n).find(|&v| pred[full_mask][v] != Self::UNVISITED) else {
            return Ok(HamiltonianPathSearchResult::proven_absent(
                self.states_expanded,
            ));
        };
        let mut reversed = Vec::with_capacity(n);
        let mut mask = full_mask;
        let mut cur = end_vertex as isize;
        while cur != -1 {
            reversed.push(cur as usize);
            let p = pred[mask][cur as usize];
            mask ^= 1 << cur;
            cur = if p == Self::START_SENTINEL {
                -1
            } else {
                (p as u8) as isize
            };
        }
        reversed.reverse();
        let vertices = reversed
            .into_iter()
            .map(|i| index_list[i].clone())
            .collect();
        Ok(HamiltonianPathSearchResult::found(
            vertex_list_to_path(vertices, graph)?,
            self.states_expanded,
        ))
    }
}

/// `BacktrackingHamiltonianPath`: depth-first search with cheap necessary-condition prechecks,
/// reachability pruning and a fewest-onward-neighbours ordering.
#[derive(Default)]
pub struct BacktrackingHamiltonianPath {
    states_expanded: u64,
    max_states_limit: u64,
    aborted: bool,
}

impl BacktrackingHamiltonianPath {
    pub fn new() -> Self {
        Self::default()
    }

    /// The number of states expanded by the last search.
    pub fn states_expanded(&self) -> u64 {
        self.states_expanded
    }

    /// Like [`get_path`](HamiltonianPathAlgorithm::get_path) but gives up after `max_states`
    /// states with an `Aborted` result.
    pub fn search_with_state_limit<G: Graph>(
        &mut self,
        graph: &G,
        max_states: u64,
    ) -> Result<HamiltonianPathSearchResult<G::Vertex, G::Edge>> {
        if max_states == 0 {
            return Err(GraphError::InvalidArgument(
                "maxStates must be positive, got 0".into(),
            ));
        }
        self.search(graph, max_states)
    }

    fn search<G: Graph>(
        &mut self,
        graph: &G,
        max_states: u64,
    ) -> Result<HamiltonianPathSearchResult<G::Vertex, G::Edge>> {
        require_directed_or_undirected(graph)?;
        self.states_expanded = 0;
        self.aborted = false;
        self.max_states_limit = max_states;
        require_not_empty(graph)?;
        let n = graph.vertex_count();
        if n == 1 {
            return Ok(HamiltonianPathSearchResult::found(singleton_path(graph), 0));
        }
        let directed = graph.graph_type().directed;
        if !directed && !Self::cheap_undirected_prechecks(graph) {
            return Ok(HamiltonianPathSearchResult::proven_absent(
                self.states_expanded,
            ));
        }
        let index_to_vertex: Vec<G::Vertex> = graph.vertices().cloned().collect();
        let vertex_to_index: HashMap<G::Vertex, usize> = index_to_vertex
            .iter()
            .enumerate()
            .map(|(i, v)| (v.clone(), i))
            .collect();
        let adjacency = build_adjacency(graph, &index_to_vertex, &vertex_to_index, directed);
        if directed && !Self::cheap_directed_prechecks(&adjacency) {
            return Ok(HamiltonianPathSearchResult::proven_absent(
                self.states_expanded,
            ));
        }
        let mut path_idx = vec![0usize; n];
        let mut visited = vec![false; n];
        for start in 0..n {
            path_idx[0] = start;
            visited[start] = true;
            if self.extend(&adjacency, &mut path_idx, &mut visited, 1, n) {
                let vertices = path_idx
                    .iter()
                    .map(|&i| index_to_vertex[i].clone())
                    .collect();
                return Ok(HamiltonianPathSearchResult::found(
                    vertex_list_to_path(vertices, graph)?,
                    self.states_expanded,
                ));
            }
            visited[start] = false;
            if self.aborted {
                return Ok(HamiltonianPathSearchResult::aborted(self.states_expanded));
            }
        }
        Ok(HamiltonianPathSearchResult::proven_absent(
            self.states_expanded,
        ))
    }

    fn cheap_undirected_prechecks<G: Graph>(graph: &G) -> bool {
        if !ConnectivityInspector::new().is_connected(graph) {
            return false;
        }
        let mut leaves = 0;
        for v in graph.vertices() {
            if Self::effective_undirected_degree(graph, v) == 1 {
                leaves += 1;
                if leaves > 2 {
                    return false;
                }
            }
        }
        let bcc = BiconnectivityInspector::new(graph);
        Self::cut_vertex_block_degree_within_path_budget(&bcc)
            && Self::bridge_tree_degree_within_path_budget(graph, &bcc)
    }

    fn cut_vertex_block_degree_within_path_budget<G: Graph>(
        bcc: &BiconnectivityInspector<'_, G>,
    ) -> bool {
        bcc.cutpoints()
            .iter()
            .all(|cut| bcc.blocks_of(cut).map(|b| b.len() <= 2).unwrap_or(true))
    }

    fn bridge_tree_degree_within_path_budget<G: Graph>(
        graph: &G,
        bcc: &BiconnectivityInspector<'_, G>,
    ) -> bool {
        let bridges = bcc.bridges();
        if bridges.len() <= 2 {
            return true; // the bridge tree then has degree <= 2 everywhere
        }
        let bridge_set: HashSet<&G::Edge> = bridges.iter().collect();
        let bridge_free =
            crate::view::MaskSubgraph::new(graph, |_| false, |e: &G::Edge| bridge_set.contains(e));
        let component_sets = ConnectivityInspector::new().connected_sets(&bridge_free);
        let mut vertex_to_component: HashMap<&G::Vertex, usize> = HashMap::new();
        for (i, set) in component_sets.iter().enumerate() {
            for v in set {
                vertex_to_component.insert(v, i);
            }
        }
        let mut incident = vec![0usize; component_sets.len()];
        for bridge in bridges {
            let (s, t) = graph.edge_endpoints(bridge).expect("edge of the graph");
            let cu = vertex_to_component[s];
            let cv = vertex_to_component[t];
            incident[cu] += 1;
            if cu != cv {
                incident[cv] += 1;
            }
            if incident[cu] > 2 || incident[cv] > 2 {
                return false;
            }
        }
        true
    }

    /// The condensation of the digraph must itself have a Hamiltonian path.
    fn cheap_directed_prechecks(adjacency: &[Vec<usize>]) -> bool {
        let n = adjacency.len();
        let (comp_of, count) = strongly_connected_components(adjacency);
        if count <= 1 {
            return true;
        }
        let mut cond: Vec<IndexSet<usize>> = vec![IndexSet::new(); count];
        for (u, targets) in adjacency.iter().enumerate().take(n) {
            for &v in targets {
                if comp_of[u] != comp_of[v] {
                    cond[comp_of[u]].insert(comp_of[v]);
                }
            }
        }
        // longest path in the condensation, which is a DAG
        let mut in_degree = vec![0usize; count];
        for targets in &cond {
            for &v in targets {
                in_degree[v] += 1;
            }
        }
        let mut queue: VecDeque<usize> = (0..count).filter(|&i| in_degree[i] == 0).collect();
        let mut longest = vec![1usize; count];
        let mut best = 1;
        while let Some(u) = queue.pop_front() {
            for &v in &cond[u] {
                longest[v] = longest[v].max(longest[u] + 1);
                best = best.max(longest[v]);
                in_degree[v] -= 1;
                if in_degree[v] == 0 {
                    queue.push_back(v);
                }
            }
        }
        best == count
    }

    fn effective_undirected_degree<G: Graph>(graph: &G, v: &G::Vertex) -> usize {
        graph
            .edges_of(v)
            .filter(|e| graph.opposite_vertex(e, v).is_some_and(|o| o != v))
            .count()
    }

    fn extend(
        &mut self,
        adjacency: &[Vec<usize>],
        path_idx: &mut [usize],
        visited: &mut [bool],
        depth: usize,
        n: usize,
    ) -> bool {
        if self.aborted {
            return false;
        }
        if self.max_states_limit > 0 && self.states_expanded >= self.max_states_limit {
            self.aborted = true;
            return false;
        }
        self.states_expanded += 1;
        if depth == n {
            return true;
        }
        let current = path_idx[depth - 1];
        let remaining = n - depth;
        if !Self::all_remaining_reachable(adjacency, visited, current, remaining) {
            return false;
        }
        let neighbours = &adjacency[current];
        let mut candidates = vec![0usize; neighbours.len()];
        let mut onward_degrees = vec![0usize; neighbours.len()];
        let mut k = 0;
        for &next in neighbours {
            if !visited[next] {
                candidates[k] = next;
                onward_degrees[k] = Self::onward_degree(adjacency, visited, next);
                k += 1;
            }
        }
        // insertion sort by onward degree (stable)
        for i in 1..k {
            let cv = candidates[i];
            let dv = onward_degrees[i];
            let mut j = i as isize - 1;
            while j >= 0 && onward_degrees[j as usize] > dv {
                onward_degrees[j as usize + 1] = onward_degrees[j as usize];
                candidates[j as usize + 1] = candidates[j as usize];
                j -= 1;
            }
            onward_degrees[(j + 1) as usize] = dv;
            candidates[(j + 1) as usize] = cv;
        }
        for &next in candidates.iter().take(k) {
            visited[next] = true;
            path_idx[depth] = next;
            if self.extend(adjacency, path_idx, visited, depth + 1, n) {
                return true;
            }
            visited[next] = false;
        }
        false
    }

    fn all_remaining_reachable(
        adjacency: &[Vec<usize>],
        visited: &[bool],
        start: usize,
        remaining: usize,
    ) -> bool {
        if remaining == 0 {
            return true;
        }
        let n = visited.len();
        let mut reached = vec![false; n];
        let mut queue = vec![0usize; n];
        let (mut head, mut tail) = (0, 0);
        queue[tail] = start;
        tail += 1;
        reached[start] = true;
        let mut found = 0;
        while head < tail {
            let u = queue[head];
            head += 1;
            for &v in &adjacency[u] {
                if !reached[v] && !visited[v] {
                    reached[v] = true;
                    queue[tail] = v;
                    tail += 1;
                    found += 1;
                    if found == remaining {
                        return true;
                    }
                }
            }
        }
        found == remaining
    }

    fn onward_degree(adjacency: &[Vec<usize>], visited: &[bool], v: usize) -> usize {
        adjacency[v].iter().filter(|&&w| !visited[w]).count()
    }
}

impl<G: Graph> HamiltonianPathAlgorithm<G> for BacktrackingHamiltonianPath {
    fn get_path(&mut self, graph: &G) -> Result<HamiltonianPathSearchResult<G::Vertex, G::Edge>> {
        self.search(graph, 0)
    }
}

/// Tarjan's algorithm (iterative): the component of every vertex and the component count.
fn strongly_connected_components(adjacency: &[Vec<usize>]) -> (Vec<usize>, usize) {
    const NONE: usize = usize::MAX;
    let n = adjacency.len();
    let mut index = vec![NONE; n];
    let mut low = vec![0usize; n];
    let mut on_stack = vec![false; n];
    let mut comp = vec![NONE; n];
    let mut stack = Vec::new();
    let mut next_index = 0;
    let mut count = 0;
    for root in 0..n {
        if index[root] != NONE {
            continue;
        }
        let mut call: Vec<(usize, usize)> = vec![(root, 0)];
        index[root] = next_index;
        low[root] = next_index;
        next_index += 1;
        stack.push(root);
        on_stack[root] = true;
        while let Some(&mut (v, ref mut child)) = call.last_mut() {
            if *child < adjacency[v].len() {
                let w = adjacency[v][*child];
                *child += 1;
                if index[w] == NONE {
                    index[w] = next_index;
                    low[w] = next_index;
                    next_index += 1;
                    stack.push(w);
                    on_stack[w] = true;
                    call.push((w, 0));
                } else if on_stack[w] {
                    low[v] = low[v].min(index[w]);
                }
            } else {
                call.pop();
                if let Some(&(parent, _)) = call.last() {
                    low[parent] = low[parent].min(low[v]);
                }
                if low[v] == index[v] {
                    loop {
                        let w = stack.pop().expect("component member");
                        on_stack[w] = false;
                        comp[w] = count;
                        if w == v {
                            break;
                        }
                    }
                    count += 1;
                }
            }
        }
    }
    (comp, count)
}
