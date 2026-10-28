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

use crate::walk::GraphWalk;
use std::collections::HashMap;
use std::hash::Hash;

/// Explicit single-source path representation (JGraphT's
/// `ListSingleSourcePathsImpl`).
///
/// Unlike [`SingleSourcePaths`], which stores a predecessor tree, this type
/// materializes every supplied path. A missing entry for the source still
/// denotes the zero-length path, matching the Java implementation.
#[derive(Debug, Clone)]
pub struct ListSingleSourcePaths<V, E> {
    source: V,
    paths: HashMap<V, GraphWalk<V, E>>,
}

impl<V: Clone + Eq + Hash, E: Clone> ListSingleSourcePaths<V, E> {
    /// Creates a new instance.
    pub fn new(source: V, paths: HashMap<V, GraphWalk<V, E>>) -> Self {
        Self { source, paths }
    }

    /// Returns the source.
    pub fn source(&self) -> &V {
        &self.source
    }

    /// Weight of the path to `target`; `None` if `target` is not reachable.
    pub fn weight(&self, target: &V) -> Option<f64> {
        self.paths
            .get(target)
            .map(|path| path.weight)
            .or_else(|| (target == &self.source).then_some(0.0))
    }

    /// Returns the path.
    pub fn path(&self, target: &V) -> Option<GraphWalk<V, E>> {
        self.paths.get(target).cloned().or_else(|| {
            (target == &self.source).then(|| GraphWalk::singleton(self.source.clone(), 0.0))
        })
    }

    /// Returns the paths.
    pub fn paths(&self) -> &HashMap<V, GraphWalk<V, E>> {
        &self.paths
    }
}

/// Shortest paths from one source to everything reachable from it.
#[derive(Debug, Clone)]
pub struct SingleSourcePaths<V, E> {
    /// The source.
    pub source: V,
    /// Distance for every reachable vertex (the source included, at `0.0`).
    pub dist: HashMap<V, f64>,
    /// For each reached vertex except the source: the last edge and its tail on the path.
    pub pred: HashMap<V, (E, V)>,
}

/// The compact predecessor-tree representation used by shortest-path
/// algorithms (JGraphT's `TreeSingleSourcePathsImpl`).
pub type TreeSingleSourcePaths<V, E> = SingleSourcePaths<V, E>;

impl<V: Clone + Eq + Hash, E: Clone + Eq + Hash> SingleSourcePaths<V, E> {
    /// Returns the source.
    pub fn source(&self) -> &V {
        &self.source
    }

    /// Returns the distance and predecessor map.
    pub fn distance_and_predecessor_map(&self) -> (&HashMap<V, f64>, &HashMap<V, (E, V)>) {
        (&self.dist, &self.pred)
    }
    /// Path weight; `None` if `v` is unreachable.
    pub fn weight(&self, v: &V) -> Option<f64> {
        self.dist.get(v).copied()
    }

    /// Returns whether reachable.
    pub fn is_reachable(&self, v: &V) -> bool {
        self.dist.contains_key(v)
    }

    /// Returns the path.
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
    /// Returns the from source.
    pub fn from_source(&self, s: &V) -> Option<&SingleSourcePaths<V, E>> {
        self.by_source.get(s)
    }
    /// Weight of the shortest path from `s` to `t`; `None` when `t` is unreachable from `s` (or
    /// `s` is unknown).
    pub fn weight(&self, s: &V, t: &V) -> Option<f64> {
        self.by_source.get(s)?.weight(t)
    }
    /// Returns the path.
    pub fn path(&self, s: &V, t: &V) -> Option<GraphWalk<V, E>> {
        self.by_source.get(s)?.path(t)
    }
}
