use super::{AllPairsPaths, ShortestPathAlgorithm, SingleSourcePaths};
use crate::graph::{Graph, GraphExt};
use crate::walk::GraphWalk;
use crate::{GraphError, Result};
use std::collections::HashMap;

/// All-pairs shortest paths in `O(V^3)`. Negative weights are fine; a negative cycle gives
/// `Err(NegativeCycle)`.
pub fn floyd_warshall<G: Graph>(g: &G) -> Result<AllPairsPaths<G::Vertex, G::Edge>> {
    let vs: Vec<&G::Vertex> = g.vertices().collect();
    let n = vs.len();
    let idx: HashMap<&G::Vertex, usize> = vs.iter().enumerate().map(|(i, v)| (*v, i)).collect();
    let mut d = vec![vec![f64::INFINITY; n]; n];
    let mut pred: Vec<Vec<Option<usize>>> = vec![vec![None; n]; n]; // predecessor of j on i ~> j
    let mut edge: Vec<Vec<Option<&G::Edge>>> = vec![vec![None; n]; n]; // lightest direct edge
    for i in 0..n {
        d[i][i] = 0.0;
        for (e, t) in g.outgoing_of(vs[i]) {
            let j = idx[t];
            let w = g.edge_weight(e).unwrap_or(1.0);
            if w < d[i][j] || (i == j && w < 0.0) {
                d[i][j] = w;
                pred[i][j] = Some(i);
                edge[i][j] = Some(e);
            }
        }
    }
    for k in 0..n {
        for i in 0..n {
            if d[i][k].is_infinite() {
                continue;
            }
            for j in 0..n {
                let nd = d[i][k] + d[k][j];
                if nd < d[i][j] {
                    d[i][j] = nd;
                    pred[i][j] = pred[k][j];
                }
            }
        }
    }
    if (0..n).any(|i| d[i][i] < 0.0) {
        return Err(GraphError::NegativeCycle);
    }
    let mut by_source = HashMap::new();
    for i in 0..n {
        let mut sp = SingleSourcePaths {
            source: vs[i].clone(),
            dist: HashMap::new(),
            pred: HashMap::new(),
        };
        for j in 0..n {
            if d[i][j].is_finite() {
                sp.dist.insert(vs[j].clone(), d[i][j]);
                if i != j {
                    let p = pred[i][j].expect("finite distance has a predecessor");
                    let e = edge[p][j].expect("direct edge into j");
                    sp.pred.insert(vs[j].clone(), (e.clone(), vs[p].clone()));
                }
            }
        }
        by_source.insert(vs[i].clone(), sp);
    }
    Ok(AllPairsPaths { by_source })
}

/// Floyd--Warshall all-pairs shortest paths.
///
/// The matrix is built eagerly in Rust; subsequent weight queries are `O(1)`
/// and path reconstruction is linear in the path length.
pub struct FloydWarshallShortestPaths<'g, G: Graph> {
    graph: &'g G,
    paths: AllPairsPaths<G::Vertex, G::Edge>,
}

impl<'g, G: Graph> FloydWarshallShortestPaths<'g, G> {
    pub fn new(graph: &'g G) -> Result<Self> {
        Ok(Self {
            graph,
            paths: floyd_warshall(graph)?,
        })
    }

    pub fn graph(&self) -> &G {
        self.graph
    }

    pub fn shortest_paths_count(&self) -> usize {
        self.graph
            .vertices()
            .map(|source| {
                self.graph
                    .vertices()
                    .filter(|target| {
                        *target != source && self.paths.weight(source, target).is_finite()
                    })
                    .count()
            })
            .sum()
    }

    pub fn first_hop(&self, source: &G::Vertex, target: &G::Vertex) -> Option<G::Vertex> {
        if source == target {
            return None;
        }
        self.paths.path(source, target)?.vertices.get(1).cloned()
    }

    pub fn last_hop(&self, source: &G::Vertex, target: &G::Vertex) -> Option<G::Vertex> {
        if source == target {
            return None;
        }
        let path = self.paths.path(source, target)?;
        path.vertices
            .get(path.vertices.len().checked_sub(2)?)
            .cloned()
    }
}

impl<G: Graph> ShortestPathAlgorithm<G> for FloydWarshallShortestPaths<'_, G> {
    fn single_source(&self, source: &G::Vertex) -> Result<SingleSourcePaths<G::Vertex, G::Edge>> {
        self.paths
            .from_source(source)
            .cloned()
            .ok_or(GraphError::VertexNotFound)
    }

    fn path(
        &self,
        source: &G::Vertex,
        target: &G::Vertex,
    ) -> Result<Option<GraphWalk<G::Vertex, G::Edge>>> {
        if !self.graph.contains_vertex(source) || !self.graph.contains_vertex(target) {
            return Err(GraphError::VertexNotFound);
        }
        Ok(self.paths.path(source, target))
    }

    fn path_weight(&self, source: &G::Vertex, target: &G::Vertex) -> Result<f64> {
        if !self.graph.contains_vertex(source) || !self.graph.contains_vertex(target) {
            return Err(GraphError::VertexNotFound);
        }
        Ok(self.paths.weight(source, target))
    }
}
