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

//! `serde` support for the sparse undirected graphs: the vertex count and the edge list (with
//! weights for the weighted graph). The incidence matrix is rebuilt on deserialization, which also
//! validates that every endpoint is a vertex.

use super::*;
use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Serialize, Deserialize)]
struct Repr<T> {
    num_vertices: usize,
    edges: Vec<T>,
}

impl Serialize for SparseIntUndirectedGraph {
    fn serialize<S: Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
        Repr {
            num_vertices: self.core.incidence.rows(),
            edges: self
                .core
                .source
                .iter()
                .copied()
                .zip(self.core.target.iter().copied())
                .collect(),
        }
        .serialize(s)
    }
}

impl<'de> Deserialize<'de> for SparseIntUndirectedGraph {
    fn deserialize<D: Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
        let r = Repr::<(usize, usize)>::deserialize(d)?;
        Self::new(r.num_vertices, &r.edges).map_err(D::Error::custom)
    }
}

impl Serialize for SparseIntUndirectedWeightedGraph {
    fn serialize<S: Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
        Repr {
            num_vertices: self.core.incidence.rows(),
            edges: self
                .core
                .source
                .iter()
                .zip(&self.core.target)
                .zip(&self.weights)
                .map(|((&s, &t), &w)| (s, t, w))
                .collect(),
        }
        .serialize(s)
    }
}

impl<'de> Deserialize<'de> for SparseIntUndirectedWeightedGraph {
    fn deserialize<D: Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
        let r = Repr::<(usize, usize, f64)>::deserialize(d)?;
        Self::new(r.num_vertices, &r.edges).map_err(D::Error::custom)
    }
}
