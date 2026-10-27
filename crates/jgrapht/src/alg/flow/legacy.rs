//! Function-style minimum cost flow (successive shortest paths) and the netgen problem type.
//!
//! Kept only for `paths.rs` (disjoint shortest paths, replaced together with the shortest-path
//! package) and the network generator, which has yet to return the faithful problem class.

use crate::alg::util::Indexed;
use crate::graph::{DirectedGraph, Graph};
use crate::{GraphError, Result};
use std::collections::{HashMap, VecDeque};

const EPS: f64 = 1e-9;

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
