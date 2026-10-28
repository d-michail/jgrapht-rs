// (C) Copyright 2003-2026, by Barak Naveh and JGraphT Contributors.
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

//! `serde` support for [`BaseGraph`] (the counterpart of JGraphT's `Serializable` graphs).
//!
//! A graph is written as its [`GraphType`], its vertices, its edges (edge object, endpoints and
//! weight, in insertion order) and the two factory counters. It is read back through
//! [`GraphMut::add_edge_with`], so the rules of the graph type (loops, parallel edges, acyclicity)
//! are enforced on untrusted input and a document that breaks them is an error, not a corrupt graph.
//!
//! The edge and vertex factories are function pointers and are not serialized: a deserialized
//! graph has none, so add edges with `add_edge_with` or install factories again.

use super::*;
use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Serialize)]
#[serde(bound(serialize = "V: Serialize, E: Serialize"))]
struct EdgeOut<'a, V, E> {
    edge: &'a E,
    source: &'a V,
    target: &'a V,
    weight: f64,
}

#[derive(Serialize)]
#[serde(bound(serialize = "V: Serialize, E: Serialize"))]
struct GraphOut<'a, V, E> {
    graph_type: GraphType,
    vertices: Vec<&'a V>,
    edges: Vec<EdgeOut<'a, V, E>>,
    next_edge_id: u64,
    next_vertex_id: u64,
    fast_lookups: bool,
}

#[derive(Deserialize)]
struct EdgeIn<V, E> {
    edge: E,
    source: V,
    target: V,
    weight: f64,
}

#[derive(Deserialize)]
struct GraphIn<V, E> {
    graph_type: GraphType,
    vertices: Vec<V>,
    edges: Vec<EdgeIn<V, E>>,
    next_edge_id: u64,
    next_vertex_id: u64,
    fast_lookups: bool,
}

impl<V, E, D, S> Serialize for BaseGraph<V, E, D, S>
where
    V: Serialize,
    E: Serialize,
{
    fn serialize<Z: Serializer>(&self, serializer: Z) -> std::result::Result<Z::Ok, Z::Error> {
        GraphOut {
            graph_type: self.ty,
            vertices: self.vertices.keys().collect(),
            edges: self
                .edges
                .iter()
                .map(|(edge, r)| EdgeOut {
                    edge,
                    source: &r.src,
                    target: &r.tgt,
                    weight: r.weight,
                })
                .collect(),
            next_edge_id: self.next_id,
            next_vertex_id: self.next_vertex,
            fast_lookups: self.endpoint_index.is_some(),
        }
        .serialize(serializer)
    }
}

impl<'de, V, E, D> Deserialize<'de> for BaseGraph<V, E, D, DefaultGraphSpecifics>
where
    V: Deserialize<'de> + Clone + Eq + Hash,
    E: Deserialize<'de> + Clone + Eq + Hash,
    D: Direction,
{
    fn deserialize<Z: Deserializer<'de>>(deserializer: Z) -> std::result::Result<Self, Z::Error> {
        let raw = GraphIn::<V, E>::deserialize(deserializer)?;
        if raw.graph_type.directed != D::DIRECTED {
            return Err(Z::Error::custom(format!(
                "the data describes a {} graph, but a {} graph was requested",
                direction_name(raw.graph_type.directed),
                direction_name(D::DIRECTED),
            )));
        }
        let mut g = Self::build(raw.graph_type, None, raw.fast_lookups);
        for v in raw.vertices {
            if !g.add_vertex(v) {
                return Err(Z::Error::custom("duplicate vertex"));
            }
        }
        let weighted = raw.graph_type.weighted;
        for e in raw.edges {
            let key = e.edge.clone();
            match g.add_edge_with(&e.source, &e.target, e.edge) {
                Ok(true) => {}
                Ok(false) => {
                    return Err(Z::Error::custom(
                        "duplicate edge, or a parallel edge in a graph without multiple edges",
                    ));
                }
                Err(err) => return Err(Z::Error::custom(err)),
            }
            // the weight of an unweighted graph stays at the default 1.0
            if weighted {
                g.set_edge_weight(&key, e.weight)
                    .map_err(Z::Error::custom)?;
            }
        }
        g.next_id = raw.next_edge_id;
        g.next_vertex = raw.next_vertex_id;
        Ok(g)
    }
}

fn direction_name(directed: bool) -> &'static str {
    if directed { "directed" } else { "undirected" }
}

/// Reading an edge moves the process-wide [`DefaultEdge`] counter past its id, so that edges
/// created later never collide with edges read from a document.
impl<'de> Deserialize<'de> for DefaultEdge {
    fn deserialize<D: Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
        let id = u64::deserialize(d)?;
        NEXT_DEFAULT_EDGE.fetch_max(id.saturating_add(1), std::sync::atomic::Ordering::Relaxed);
        Ok(DefaultEdge(id))
    }
}
