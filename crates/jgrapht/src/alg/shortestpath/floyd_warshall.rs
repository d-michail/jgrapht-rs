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

use crate::graph::OutgoingGraph;
use super::{AllPairsPaths, ShortestPathAlgorithm, SingleSourcePaths};
use crate::graph::{Graph, GraphExt};
use crate::walk::GraphWalk;
use crate::{GraphError, Result};
use std::collections::HashMap;

/// All-pairs shortest paths in `O(V^3)`. Negative weights are fine; a negative cycle gives
/// `Err(NegativeCycle)`.
pub(crate) fn floyd_warshall<G: OutgoingGraph>(g: &G) -> Result<AllPairsPaths<G::Vertex, G::Edge>> {
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
            let w = g.weight_of(e);
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

/// The Floyd-Warshall algorithm.
///
/// The [Floyd-Warshall algorithm](http://en.wikipedia.org/wiki/Floyd-Warshall_algorithm)
/// finds all shortest paths (all $n^2$ of them) in $O(n^3)$ time. Note that during construction
/// time, no computations are performed! All computations are performed the first time one of the
/// member methods of this class is invoked. The results are stored, so all subsequent calls to the
/// same method are computationally efficient.
///
/// # Rust notes
///
/// Floyd--Warshall all-pairs shortest paths.
///
/// The matrix is built eagerly in Rust; subsequent weight queries are `O(1)`
/// and path reconstruction is linear in the path length.
pub struct FloydWarshallShortestPaths<'g, G: Graph> {
    graph: &'g G,
    paths: AllPairsPaths<G::Vertex, G::Edge>,
}

impl<'g, G: OutgoingGraph> FloydWarshallShortestPaths<'g, G> {
    /// Create a new instance of the Floyd-Warshall all-pairs shortest path algorithm.
    pub fn new(graph: &'g G) -> Result<Self> {
        Ok(Self {
            graph,
            paths: floyd_warshall(graph)?,
        })
    }

    /// Returns the graph over which this set of paths is defined.
    pub fn graph(&self) -> &G {
        self.graph
    }

    /// The shortest paths between every pair of vertices.
    pub fn all_pairs(&self) -> &AllPairsPaths<G::Vertex, G::Edge> {
        &self.paths
    }

    /// Get the total number of shortest paths. Does not count the paths from a vertex to itself.
    pub fn shortest_paths_count(&self) -> usize {
        self.graph
            .vertices()
            .map(|source| {
                self.graph
                    .vertices()
                    .filter(|target| {
                        *target != source && self.paths.weight(source, target).is_some()
                    })
                    .count()
            })
            .sum()
    }

    /// Returns the first hop, i.e., the second node on the shortest path from $a$ to $b$. Lookup
    /// time is $O(1)$. If the shortest path from $a$ to $b$ is $a,c,d,e,b$, this method returns $c$.
    /// If the next invocation would query the first hop on the shortest path from $c$ to $b$, vertex
    /// $d$ would be returned, etc. This method is computationally cheaper than calling
    /// `getPath(Object, Object)` and then reading the first vertex.
    pub fn first_hop(&self, source: &G::Vertex, target: &G::Vertex) -> Option<G::Vertex> {
        if source == target {
            return None;
        }
        self.paths.path(source, target)?.vertices.get(1).cloned()
    }

    /// Returns the last hop, i.e., the second to last node on the shortest path from $a$ to $b$.
    /// Lookup time is $O(1)$. If the shortest path from $a$ to $b$ is $a,c,d,e,b$, this method
    /// returns $e$. If the next invocation would query the next hop on the shortest path from $c$ to
    /// $e$, vertex $d$ would be returned, etc. This method is computationally cheaper than calling
    /// `getPath(Object, Object)` and then reading the vertex. The first invocation of this
    /// method populates a last hop matrix.
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

    /// Get a shortest path from a source vertex to a sink vertex.
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

    /// Get the weight of the shortest path from a source vertex to a sink vertex; `None` if no
    /// path exists.
    fn path_weight(&self, source: &G::Vertex, target: &G::Vertex) -> Result<Option<f64>> {
        if !self.graph.contains_vertex(source) || !self.graph.contains_vertex(target) {
            return Err(GraphError::VertexNotFound);
        }
        Ok(self.paths.weight(source, target))
    }
}
