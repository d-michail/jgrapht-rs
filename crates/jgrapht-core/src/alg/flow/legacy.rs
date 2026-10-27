//! Maximum flow, minimum cuts, Gomory–Hu trees, minimum cost flow, Stoer–Wagner.
//!
//! Edge weights are capacities. In undirected graphs each edge carries capacity in both
//! directions and the reported flow of an edge is signed relative to (source -> target).

use crate::alg::util::Indexed;
use crate::graph::{DirectedGraph, Graph};
use crate::{GraphError, Result};
use std::collections::{HashMap, VecDeque};

const EPS: f64 = 1e-9;

/// Result of a maximum-flow computation.
#[derive(Debug, Clone)]
pub struct MaxFlow<V, E> {
    pub value: f64,
    /// Net flow per edge (signed in undirected graphs).
    pub flow: HashMap<E, f64>,
    /// Vertices on the source side of a minimum cut (reachable in the residual network).
    pub source_side: Vec<V>,
}

/// A cut separating two vertices.
#[derive(Debug, Clone)]
pub struct MinCut<V, E> {
    pub weight: f64,
    pub source_side: Vec<V>,
    pub sink_side: Vec<V>,
    pub cut_edges: Vec<E>,
}

struct Network {
    adj: Vec<Vec<usize>>,
    to: Vec<usize>,
    cap: Vec<f64>,
}

impl Network {
    fn new<G: Graph>(ix: &Indexed<'_, G>, capacity: impl Fn(usize) -> f64) -> Self {
        let mut net = Network {
            adj: vec![Vec::new(); ix.n()],
            to: Vec::new(),
            cap: Vec::new(),
        };
        for (k, &(s, t)) in ix.ends.iter().enumerate() {
            let c = capacity(k);
            net.adj[s].push(2 * k);
            net.adj[t].push(2 * k + 1);
            net.to.extend([t, s]);
            net.cap.extend([c, if ix.directed { 0.0 } else { c }]);
        }
        net
    }
    fn n(&self) -> usize {
        self.adj.len()
    }
    fn reachable(&self, s: usize) -> Vec<bool> {
        let mut seen = vec![false; self.n()];
        seen[s] = true;
        let mut q = VecDeque::from([s]);
        while let Some(v) = q.pop_front() {
            for &a in &self.adj[v] {
                if self.cap[a] > EPS && !seen[self.to[a]] {
                    seen[self.to[a]] = true;
                    q.push_back(self.to[a]);
                }
            }
        }
        seen
    }
    fn edmonds_karp(&mut self, s: usize, t: usize) -> f64 {
        let mut total = 0.0;
        loop {
            let mut via = vec![usize::MAX; self.n()];
            let mut seen = vec![false; self.n()];
            seen[s] = true;
            let mut q = VecDeque::from([s]);
            while let Some(v) = q.pop_front() {
                if v == t {
                    break;
                }
                for &a in &self.adj[v] {
                    let u = self.to[a];
                    if self.cap[a] > EPS && !seen[u] {
                        seen[u] = true;
                        via[u] = a;
                        q.push_back(u);
                    }
                }
            }
            if !seen[t] {
                return total;
            }
            let (mut bottleneck, mut v) = (f64::INFINITY, t);
            while v != s {
                bottleneck = bottleneck.min(self.cap[via[v]]);
                v = self.to[via[v] ^ 1];
            }
            v = t;
            while v != s {
                self.cap[via[v]] -= bottleneck;
                self.cap[via[v] ^ 1] += bottleneck;
                v = self.to[via[v] ^ 1];
            }
            total += bottleneck;
        }
    }
    fn dinic(&mut self, s: usize, t: usize) -> f64 {
        fn dfs(
            net: &mut Network,
            v: usize,
            t: usize,
            pushed: f64,
            level: &[i64],
            it: &mut [usize],
        ) -> f64 {
            if v == t {
                return pushed;
            }
            while it[v] < net.adj[v].len() {
                let a = net.adj[v][it[v]];
                let u = net.to[a];
                if net.cap[a] > EPS && level[u] == level[v] + 1 {
                    let d = dfs(net, u, t, pushed.min(net.cap[a]), level, it);
                    if d > EPS {
                        net.cap[a] -= d;
                        net.cap[a ^ 1] += d;
                        return d;
                    }
                }
                it[v] += 1;
            }
            0.0
        }
        let mut total = 0.0;
        loop {
            let mut level = vec![-1i64; self.n()];
            level[s] = 0;
            let mut q = VecDeque::from([s]);
            while let Some(v) = q.pop_front() {
                for &a in &self.adj[v] {
                    if self.cap[a] > EPS && level[self.to[a]] < 0 {
                        level[self.to[a]] = level[v] + 1;
                        q.push_back(self.to[a]);
                    }
                }
            }
            if level[t] < 0 {
                return total;
            }
            let mut it = vec![0; self.n()];
            loop {
                let f = dfs(self, s, t, f64::INFINITY, &level, &mut it);
                if f <= EPS {
                    break;
                }
                total += f;
            }
        }
    }
    /// FIFO push–relabel with the gap heuristic omitted (kept simple).
    fn push_relabel(&mut self, s: usize, t: usize) -> f64 {
        let n = self.n();
        let (mut height, mut excess) = (vec![0usize; n], vec![0.0f64; n]);
        let mut queue = VecDeque::new();
        height[s] = n;
        for i in 0..self.adj[s].len() {
            let a = self.adj[s][i];
            let c = self.cap[a];
            if c > EPS {
                self.cap[a] = 0.0;
                self.cap[a ^ 1] += c;
                excess[self.to[a]] += c;
                excess[s] -= c;
                if self.to[a] != t && self.to[a] != s {
                    queue.push_back(self.to[a]);
                }
            }
        }
        let mut in_queue = vec![false; n];
        queue.iter().for_each(|&v| in_queue[v] = true);
        while let Some(v) = queue.pop_front() {
            in_queue[v] = false;
            while excess[v] > EPS {
                let mut pushed = false;
                for i in 0..self.adj[v].len() {
                    let a = self.adj[v][i];
                    let u = self.to[a];
                    if self.cap[a] > EPS && height[v] == height[u] + 1 {
                        let d = excess[v].min(self.cap[a]);
                        self.cap[a] -= d;
                        self.cap[a ^ 1] += d;
                        excess[v] -= d;
                        excess[u] += d;
                        if u != s && u != t && !in_queue[u] {
                            in_queue[u] = true;
                            queue.push_back(u);
                        }
                        pushed = true;
                        if excess[v] <= EPS {
                            break;
                        }
                    }
                }
                if excess[v] > EPS && !pushed {
                    height[v] = 1 + self.adj[v]
                        .iter()
                        .filter(|&&a| self.cap[a] > EPS)
                        .map(|&a| height[self.to[a]])
                        .min()
                        .unwrap_or(n);
                }
            }
        }
        excess[t]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MaxFlowAlgorithm {
    EdmondsKarp,
    Dinic,
    PushRelabel,
}

/// Maximum flow from `source` to `sink`; edge weights are capacities.
pub fn max_flow<G: Graph>(
    g: &G,
    source: &G::Vertex,
    sink: &G::Vertex,
    algorithm: MaxFlowAlgorithm,
) -> Result<MaxFlow<G::Vertex, G::Edge>> {
    let ix = Indexed::new(g);
    let s = *ix.idx.get(source).ok_or(GraphError::VertexNotFound)?;
    let t = *ix.idx.get(sink).ok_or(GraphError::VertexNotFound)?;
    if s == t {
        return Err(GraphError::InvalidArgument(
            "source and sink must differ".into(),
        ));
    }
    if ix.w.iter().any(|&c| c < 0.0 || c.is_nan()) {
        return Err(GraphError::InvalidArgument(
            "capacities must be non-negative".into(),
        ));
    }
    let mut net = Network::new(&ix, |k| ix.w[k]);
    let original = net.cap.clone();
    let value = match algorithm {
        MaxFlowAlgorithm::EdmondsKarp => net.edmonds_karp(s, t),
        MaxFlowAlgorithm::Dinic => net.dinic(s, t),
        MaxFlowAlgorithm::PushRelabel => net.push_relabel(s, t),
    };
    let side = net.reachable(s);
    let flow = (0..ix.es.len())
        .map(|k| (ix.es[k].clone(), original[2 * k] - net.cap[2 * k]))
        .collect();
    let source_side = (0..ix.n())
        .filter(|&v| side[v])
        .map(|v| ix.vertex(v))
        .collect();
    Ok(MaxFlow {
        value,
        flow,
        source_side,
    })
}

pub fn edmonds_karp<G: Graph>(
    g: &G,
    s: &G::Vertex,
    t: &G::Vertex,
) -> Result<MaxFlow<G::Vertex, G::Edge>> {
    max_flow(g, s, t, MaxFlowAlgorithm::EdmondsKarp)
}
pub fn dinic<G: Graph>(g: &G, s: &G::Vertex, t: &G::Vertex) -> Result<MaxFlow<G::Vertex, G::Edge>> {
    max_flow(g, s, t, MaxFlowAlgorithm::Dinic)
}
pub fn push_relabel<G: Graph>(
    g: &G,
    s: &G::Vertex,
    t: &G::Vertex,
) -> Result<MaxFlow<G::Vertex, G::Edge>> {
    max_flow(g, s, t, MaxFlowAlgorithm::PushRelabel)
}

/// Minimum s–t cut (max-flow min-cut).
pub fn minimum_st_cut<G: Graph>(
    g: &G,
    s: &G::Vertex,
    t: &G::Vertex,
) -> Result<MinCut<G::Vertex, G::Edge>> {
    let f = max_flow(g, s, t, MaxFlowAlgorithm::Dinic)?;
    let side: std::collections::HashSet<_> = f.source_side.iter().cloned().collect();
    let sink_side = g
        .vertices()
        .filter(|v| !side.contains(*v))
        .cloned()
        .collect();
    let directed = g.graph_type().directed;
    let cut_edges = g
        .edges()
        .filter(|e| {
            let (a, b) = (
                side.contains(g.edge_source(e).unwrap()),
                side.contains(g.edge_target(e).unwrap()),
            );
            if directed { a && !b } else { a != b }
        })
        .cloned()
        .collect();
    Ok(MinCut {
        weight: f.value,
        source_side: f.source_side,
        sink_side,
        cut_edges,
    })
}

/// Gomory–Hu tree of an undirected graph (Gusfield's algorithm): a tree on the same vertices
/// where the minimum edge on the tree path between two vertices equals their min-cut value.
#[derive(Debug, Clone)]
pub struct GomoryHuTree<V> {
    /// Tree edges `(child, parent, cut value)`.
    pub edges: Vec<(V, V, f64)>,
    index: HashMap<V, usize>,
    parent: Vec<usize>,
    value: Vec<f64>,
}

impl<V: Clone + Eq + std::hash::Hash> GomoryHuTree<V> {
    /// Min-cut value between two vertices.
    pub fn min_cut_value(&self, a: &V, b: &V) -> Option<f64> {
        let (x, y) = (*self.index.get(a)?, *self.index.get(b)?);
        if x == y {
            return Some(f64::INFINITY);
        }
        let path = |mut v: usize| {
            let mut p = vec![v];
            while v != 0 {
                v = self.parent[v];
                p.push(v);
            }
            p
        };
        let (px, py) = (path(x), path(y));
        let common = px.iter().find(|v| py.contains(v)).copied()?;
        let mut best = f64::INFINITY;
        for mut v in [x, y] {
            while v != common {
                best = best.min(self.value[v]);
                v = self.parent[v];
            }
        }
        Some(best)
    }
}

pub fn gomory_hu_tree<G: Graph>(g: &G) -> Result<GomoryHuTree<G::Vertex>> {
    if g.graph_type().directed {
        return Err(GraphError::InvalidArgument(
            "Gomory–Hu trees need an undirected graph".into(),
        ));
    }
    let ix = Indexed::new(g);
    let n = ix.n();
    let (mut p, mut fl) = (vec![0usize; n], vec![0.0f64; n]);
    for s in 1..n {
        let t = p[s];
        let mut net = Network::new(&ix, |k| ix.w[k]);
        let f = net.dinic(s, t);
        let x = net.reachable(s);
        fl[s] = f;
        for i in 0..n {
            if i != s && x[i] && p[i] == t {
                p[i] = s;
            }
        }
        if x[p[t]] {
            p[s] = p[t];
            p[t] = s;
            fl[s] = fl[t];
            fl[t] = f;
        }
    }
    // Gusfield's output is a tree but not necessarily rooted at 0; re-root at 0.
    let mut adj: Vec<Vec<(usize, f64)>> = vec![Vec::new(); n];
    for s in 1..n {
        adj[s].push((p[s], fl[s]));
        adj[p[s]].push((s, fl[s]));
    }
    let (mut parent, mut value, mut seen) = (vec![0usize; n], vec![0.0f64; n], vec![false; n]);
    let mut edges = Vec::new();
    if n > 0 {
        seen[0] = true;
        let mut q = VecDeque::from([0]);
        while let Some(v) = q.pop_front() {
            for &(u, w) in &adj[v] {
                if !seen[u] {
                    seen[u] = true;
                    parent[u] = v;
                    value[u] = w;
                    edges.push((ix.vertex(u), ix.vertex(v), w));
                    q.push_back(u);
                }
            }
        }
    }
    let index = (0..n).map(|i| (ix.vertex(i), i)).collect();
    Ok(GomoryHuTree {
        edges,
        index,
        parent,
        value,
    })
}

/// Stoer–Wagner global minimum cut of an undirected graph (edge weights >= 0).
pub fn stoer_wagner<G: Graph>(g: &G) -> Result<MinCut<G::Vertex, G::Edge>> {
    if g.graph_type().directed {
        return Err(GraphError::InvalidArgument(
            "Stoer–Wagner needs an undirected graph".into(),
        ));
    }
    let ix = Indexed::new(g);
    let n = ix.n();
    if n < 2 {
        return Err(GraphError::InvalidArgument(
            "need at least two vertices".into(),
        ));
    }
    let mut w = vec![vec![0.0f64; n]; n];
    for (k, &(a, b)) in ix.ends.iter().enumerate() {
        if a != b {
            w[a][b] += ix.w[k];
            w[b][a] += ix.w[k];
        }
    }
    let mut groups: Vec<Vec<usize>> = (0..n).map(|v| vec![v]).collect();
    let mut active: Vec<usize> = (0..n).collect();
    let (mut best, mut best_group) = (f64::INFINITY, Vec::new());
    while active.len() > 1 {
        let mut added = vec![false; n];
        let mut key = vec![0.0f64; n];
        let (mut prev, mut last) = (usize::MAX, usize::MAX);
        for _ in 0..active.len() {
            let v = *active
                .iter()
                .filter(|&&v| !added[v])
                .max_by(|&&a, &&b| key[a].total_cmp(&key[b]))
                .unwrap();
            added[v] = true;
            prev = last;
            last = v;
            for &u in &active {
                if !added[u] {
                    key[u] += w[v][u];
                }
            }
        }
        if key[last] < best {
            best = key[last];
            best_group = groups[last].clone();
        }
        // merge `last` into `prev`
        let moved = std::mem::take(&mut groups[last]);
        groups[prev].extend(moved);
        for &u in &active {
            if u != prev && u != last {
                w[prev][u] += w[last][u];
                w[u][prev] = w[prev][u];
            }
        }
        active.retain(|&v| v != last);
    }
    let in_side: Vec<bool> = (0..n).map(|v| best_group.contains(&v)).collect();
    Ok(MinCut {
        weight: best,
        source_side: (0..n)
            .filter(|&v| in_side[v])
            .map(|v| ix.vertex(v))
            .collect(),
        sink_side: (0..n)
            .filter(|&v| !in_side[v])
            .map(|v| ix.vertex(v))
            .collect(),
        cut_edges: (0..ix.es.len())
            .filter(|&k| in_side[ix.ends[k].0] != in_side[ix.ends[k].1])
            .map(|k| ix.es[k].clone())
            .collect(),
    })
}

/// Result of a minimum cost flow computation.
#[derive(Debug, Clone)]
pub struct MinCostFlow<E> {
    pub flow_value: f64,
    pub cost: f64,
    pub flow: HashMap<E, f64>,
}

/// Min-cost flow from `source` to `sink` by successive shortest paths (SPFA), sending up to
/// `amount` units (`None` = maximum flow). `capacity` and `cost` are per edge.
pub fn min_cost_flow<G: DirectedGraph>(
    g: &G,
    source: &G::Vertex,
    sink: &G::Vertex,
    amount: Option<f64>,
    capacity: impl Fn(&G::Edge) -> f64,
    cost: impl Fn(&G::Edge) -> f64,
) -> Result<MinCostFlow<G::Edge>> {
    let ix = Indexed::new(g);
    let s = *ix.idx.get(source).ok_or(GraphError::VertexNotFound)?;
    let t = *ix.idx.get(sink).ok_or(GraphError::VertexNotFound)?;
    let m = ix.es.len();
    let mut net = Network::new(&ix, |k| capacity(ix.es[k]));
    let mut c = vec![0.0; 2 * m];
    for k in 0..m {
        c[2 * k] = cost(ix.es[k]);
        c[2 * k + 1] = -c[2 * k];
    }
    let original = net.cap.clone();
    let (mut sent, mut total_cost) = (0.0, 0.0);
    let limit = amount.unwrap_or(f64::INFINITY);
    while sent + EPS < limit {
        let n = ix.n();
        let mut dist = vec![f64::INFINITY; n];
        let (mut via, mut inq) = (vec![usize::MAX; n], vec![false; n]);
        dist[s] = 0.0;
        let mut q = VecDeque::from([s]);
        let mut relaxations = 0usize;
        while let Some(v) = q.pop_front() {
            inq[v] = false;
            relaxations += 1;
            if relaxations > n * (2 * m + 1) + 1 {
                return Err(GraphError::NegativeCycle);
            }
            for &a in &net.adj[v] {
                let u = net.to[a];
                if net.cap[a] > EPS && dist[v] + c[a] < dist[u] - 1e-12 {
                    dist[u] = dist[v] + c[a];
                    via[u] = a;
                    if !inq[u] {
                        inq[u] = true;
                        q.push_back(u);
                    }
                }
            }
        }
        if dist[t].is_infinite() {
            break;
        }
        let (mut push, mut v) = (limit - sent, t);
        while v != s {
            push = push.min(net.cap[via[v]]);
            v = net.to[via[v] ^ 1];
        }
        v = t;
        while v != s {
            net.cap[via[v]] -= push;
            net.cap[via[v] ^ 1] += push;
            v = net.to[via[v] ^ 1];
        }
        sent += push;
        total_cost += push * dist[t];
    }
    let flow = (0..m)
        .map(|k| (ix.es[k].clone(), original[2 * k] - net.cap[2 * k]))
        .collect();
    Ok(MinCostFlow {
        flow_value: sent,
        cost: total_cost,
        flow,
    })
}

/// A minimum cost flow problem (`MinimumCostFlowProblem`): node supplies (positive for sources,
/// negative for sinks) and per-arc capacity bounds and costs. The graph is kept by the caller.
#[derive(Debug, Clone)]
pub struct MinimumCostFlowProblem<V, E> {
    supply: HashMap<V, i32>,
    lower_bounds: Option<HashMap<E, i32>>,
    upper_bounds: HashMap<E, i32>,
    costs: HashMap<E, i32>,
}

impl<V: Eq + std::hash::Hash, E: Eq + std::hash::Hash> MinimumCostFlowProblem<V, E> {
    /// Lower capacity bounds are all zero.
    pub fn new(
        supply: HashMap<V, i32>,
        upper_bounds: HashMap<E, i32>,
        costs: HashMap<E, i32>,
    ) -> Self {
        MinimumCostFlowProblem {
            supply,
            lower_bounds: None,
            upper_bounds,
            costs,
        }
    }
    pub fn with_lower_bounds(mut self, lower: HashMap<E, i32>) -> Self {
        self.lower_bounds = Some(lower);
        self
    }
    /// Supply of `v` (0 if it is not a source or sink).
    pub fn node_supply(&self, v: &V) -> i32 {
        self.supply.get(v).copied().unwrap_or(0)
    }
    pub fn arc_capacity_lower_bound(&self, e: &E) -> i32 {
        self.lower_bounds
            .as_ref()
            .map_or(0, |m| m.get(e).copied().unwrap_or(0))
    }
    pub fn arc_capacity_upper_bound(&self, e: &E) -> i32 {
        self.upper_bounds[e]
    }
    pub fn arc_cost(&self, e: &E) -> i32 {
        self.costs[e]
    }
}
