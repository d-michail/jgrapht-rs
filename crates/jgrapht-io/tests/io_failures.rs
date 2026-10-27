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

use jgrapht::{DefaultEdge, DiGraph, Graph, GraphMut};
use jgrapht_io::dimacs::{DimacsExporter, DimacsImporter};
use jgrapht_io::{GraphExporter, GraphImporter};
use std::io::{self, Read, Write};

struct FailingReader;

impl Read for FailingReader {
    fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
        Err(io::Error::other("injected read failure"))
    }
}

struct FailingWriter;

impl Write for FailingWriter {
    fn write(&mut self, _: &[u8]) -> io::Result<usize> {
        Err(io::Error::other("injected write failure"))
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[test]
fn importer_propagates_reader_failures() {
    let mut graph = DiGraph::<usize>::multi().with_vertex_factory(|id| id as usize);
    let error = DimacsImporter::new()
        .import_graph(&mut graph, FailingReader)
        .unwrap_err();
    assert!(error.to_string().contains("injected read failure"));
}

#[test]
fn exporter_propagates_writer_failures() {
    let mut graph = DiGraph::<usize>::simple();
    graph.add_vertex(1);
    let mut exporter: DimacsExporter<usize, DefaultEdge> = DimacsExporter::new();
    let error = exporter.export_graph(&graph, FailingWriter).unwrap_err();
    assert!(error.to_string().contains("injected write failure"));
}

#[test]
fn importer_can_be_reused_for_independent_graphs() {
    let input = "p edge 2 1\ne 1 2\n";
    let mut importer = DimacsImporter::new();
    for _ in 0..2 {
        let mut graph = DiGraph::<usize>::multi().with_vertex_factory(|id| id as usize);
        importer.import_str(&mut graph, input).unwrap();
        assert_eq!(graph.vertex_count(), 2);
        assert_eq!(graph.edge_count(), 1);
    }
}
