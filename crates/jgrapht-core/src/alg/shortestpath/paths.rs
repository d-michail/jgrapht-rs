use crate::walk::GraphWalk;
use std::collections::HashMap;
use std::hash::Hash;

/// Shortest paths from one source to everything reachable from it.
#[derive(Debug, Clone)]
pub struct SingleSourcePaths<V, E> {
    pub source: V,
    /// Distance for every reachable vertex (the source included, at `0.0`).
    pub dist: HashMap<V, f64>,
    /// For each reached vertex except the source: the last edge and its tail on the path.
    pub pred: HashMap<V, (E, V)>,
}

impl<V: Clone + Eq + Hash, E: Clone + Eq + Hash> SingleSourcePaths<V, E> {
    /// Path weight, `f64::INFINITY` if `v` is unreachable.
    pub fn weight(&self, v: &V) -> f64 {
        self.dist.get(v).copied().unwrap_or(f64::INFINITY)
    }

    pub fn is_reachable(&self, v: &V) -> bool {
        self.dist.contains_key(v)
    }

    pub fn path(&self, v: &V) -> Option<GraphWalk<V, E>> {
        let weight = *self.dist.get(v)?;
        let mut vertices = vec![v.clone()];
        let mut edges = Vec::new();
        let mut cur = v;
        while let Some((e, p)) = self.pred.get(cur) {
            edges.push(e.clone());
            vertices.push(p.clone());
            cur = p;
        }
        vertices.reverse();
        edges.reverse();
        Some(GraphWalk {
            start: self.source.clone(),
            end: v.clone(),
            vertices,
            edges,
            weight,
        })
    }
}

/// Shortest paths between all pairs of vertices.
#[derive(Debug, Clone)]
pub struct AllPairsPaths<V, E> {
    pub(crate) by_source: HashMap<V, SingleSourcePaths<V, E>>,
}

impl<V: Clone + Eq + Hash, E: Clone + Eq + Hash> AllPairsPaths<V, E> {
    pub fn from_source(&self, s: &V) -> Option<&SingleSourcePaths<V, E>> {
        self.by_source.get(s)
    }
    /// `f64::INFINITY` when `t` is unreachable from `s` (or `s` is unknown).
    pub fn weight(&self, s: &V, t: &V) -> f64 {
        self.by_source.get(s).map_or(f64::INFINITY, |p| p.weight(t))
    }
    pub fn path(&self, s: &V, t: &V) -> Option<GraphWalk<V, E>> {
        self.by_source.get(s)?.path(t)
    }
}
