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

use jgrapht::{DefaultEdge, DiGraph, GraphMut, GraphMutExt, UGraph};
use jgrapht_io::GraphExporter;
use jgrapht_io::matrix::{MatrixExporter, MatrixFormat};

const LAPLACIAN: &str = "1 1 2\n1 2 -1\n1 3 -1\n2 2 1\n2 1 -1\n3 3 1\n3 1 -1\n";
const NORMALIZED_LAPLACIAN: &str = "1 1 1\n1 2 -0.7071067811865475\n1 3 -0.7071067811865475\n2 2 1\n2 1 -0.7071067811865475\n3 3 1\n3 1 -0.7071067811865475\n";
const UNDIRECTED_ADJACENCY: &str = "1 2 1\n1 3 1\n1 1 2\n2 1 1\n3 1 1\n";
const DIRECTED_ADJACENCY: &str = "1 2 1\n3 1 2\n";

#[test]
fn laplacian() {
    let mut graph = UGraph::<&str>::simple();
    graph.add_vertices(["v1", "v2", "v3"]);
    graph.add_edge(&"v1", &"v2").unwrap();
    graph.add_edge(&"v3", &"v1").unwrap();

    let mut exporter: MatrixExporter<&str, DefaultEdge> =
        MatrixExporter::new(MatrixFormat::SparseLaplacian);
    assert_eq!(exporter.format(), MatrixFormat::SparseLaplacian);
    assert_eq!(exporter.export_to_string(&graph).unwrap(), LAPLACIAN);

    exporter.set_format(MatrixFormat::SparseNormalizedLaplacian);
    assert_eq!(exporter.format(), MatrixFormat::SparseNormalizedLaplacian);
    assert_eq!(
        exporter.export_to_string(&graph).unwrap(),
        NORMALIZED_LAPLACIAN
    );
}

#[test]
fn adjacency_undirected() {
    let mut graph = UGraph::<&str>::pseudo();
    graph.add_vertices(["v1", "v2", "v3"]);
    graph.add_edge(&"v1", &"v2").unwrap();
    graph.add_edge(&"v3", &"v1").unwrap();
    graph.add_edge(&"v1", &"v1").unwrap();

    let mut exporter: MatrixExporter<&str, DefaultEdge> = MatrixExporter::default();
    assert_eq!(exporter.format(), MatrixFormat::SparseAdjacency);
    assert_eq!(
        exporter.export_to_string(&graph).unwrap(),
        UNDIRECTED_ADJACENCY
    );
}

#[test]
fn adjacency_directed() {
    let mut graph = DiGraph::<&str>::multi();
    graph.add_vertices(["v1", "v2", "v3"]);
    graph.add_edge(&"v1", &"v2").unwrap();
    graph.add_edge(&"v3", &"v1").unwrap();
    graph.add_edge(&"v3", &"v1").unwrap();

    let mut exporter: MatrixExporter<&str, DefaultEdge> = MatrixExporter::default();
    assert_eq!(
        exporter.export_to_string(&graph).unwrap(),
        DIRECTED_ADJACENCY
    );
}
