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

//! Induced cycles ("holes") of length five or more: weak chordality and Berge's perfection test.
//!
//! These function helpers delegate to the Java-faithful classes in [`super::cycle`]
//! ([`WeakChordalityInspector`] and [`BergeGraphInspector`]), so they have the same complexity as
//! JGraphT's.

use super::cycle::{BergeGraphInspector, WeakChordalityInspector};
use crate::graph::Graph;
use crate::graph::IncidenceGraph;

/// A hole or antihole (an induced cycle of length >= 5 in `g` or its complement), as a vertex list
/// without the repeated closing vertex, if any.
pub fn find_weak_chordality_certificate<G: IncidenceGraph>(g: &G) -> Option<Vec<G::Vertex>> {
    let mut inspector = WeakChordalityInspector::new(g);
    let certificate = inspector.get_certificate()?;
    let mut vertices = certificate.vertex_list().to_vec();
    vertices.pop();
    Some(vertices)
}

pub fn is_weakly_chordal<G: IncidenceGraph>(g: &G) -> bool {
    WeakChordalityInspector::new(g).is_weakly_chordal()
}

/// An odd hole or odd antihole of an undirected graph, if any (a certificate that the graph is
/// not perfect), as a vertex list without the repeated closing vertex.
pub fn find_odd_hole_or_antihole<G: Graph>(g: &G) -> Option<Vec<G::Vertex>> {
    let mut inspector = BergeGraphInspector::new();
    if inspector.is_berge_with_certificate(g, true).unwrap_or(true) {
        return None;
    }
    let mut vertices = inspector.get_certificate()?.vertex_list().to_vec();
    vertices.pop();
    Some(vertices)
}

/// Berge graph / perfect graph test (strong perfect graph theorem); `false` for directed graphs.
pub fn is_perfect<G: Graph>(g: &G) -> bool {
    BergeGraphInspector::new().is_berge(g).unwrap_or(false)
}
