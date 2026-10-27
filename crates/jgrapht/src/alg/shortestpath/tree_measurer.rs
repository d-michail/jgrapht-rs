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
use crate::graph::{GraphExt};
use crate::{GraphError, Result};
use std::collections::{HashMap, HashSet, VecDeque};

/// Algorithm class which computes a number of distance related metrics for trees.
///
/// # Rust notes
///
/// Linear-time center computation for an undirected tree.
pub struct TreeMeasurer<'g, G> {
    graph: &'g G,
}

impl<'g, G: OutgoingGraph> TreeMeasurer<'g, G> {
    /// Constructs a new instance of TreeMeasurer.
    pub fn new(graph: &'g G) -> Self {
        Self { graph }
    }

    /// Compute the [graph center](http://mathworld.wolfram.com/GraphCenter.html). The
    /// center of a graph is the set of vertices of graph eccentricity equal to the graph radius.
    ///
    /// Note: The input graph must be undirected.
    pub fn graph_center(&self) -> Result<Vec<G::Vertex>> {
        if self.graph.graph_type().directed {
            return Err(GraphError::InvalidGraph(crate::GraphProperty::Undirected));
        }
        let Some(root) = self.graph.vertices().next() else {
            return Ok(Vec::new());
        };
        let (end, _) = farthest(self.graph, root);
        let (other_end, parent) = farthest(self.graph, end);

        let mut diameter = vec![(*other_end).clone()];
        let mut current = other_end;
        while let Some(previous) = parent.get(current) {
            diameter.push((**previous).clone());
            current = *previous;
        }
        let middle = diameter.len() / 2;
        if diameter.len() % 2 == 1 {
            Ok(vec![diameter[middle].clone()])
        } else {
            Ok(vec![diameter[middle].clone(), diameter[middle - 1].clone()])
        }
    }
}

fn farthest<'g, G: OutgoingGraph>(
    graph: &'g G,
    root: &'g G::Vertex,
) -> (&'g G::Vertex, HashMap<&'g G::Vertex, &'g G::Vertex>) {
    let mut queue = VecDeque::from([(root, 0usize)]);
    let mut seen = HashSet::from([root]);
    let mut parent = HashMap::new();
    let mut result = (root, 0usize);
    while let Some((vertex, depth)) = queue.pop_front() {
        if depth > result.1 {
            result = (vertex, depth);
        }
        for (_, target) in graph.outgoing_of(vertex) {
            if seen.insert(target) {
                parent.insert(target, vertex);
                queue.push_back((target, depth + 1));
            }
        }
    }
    (result.0, parent)
}
