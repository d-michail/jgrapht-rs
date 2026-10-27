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

use jgrapht::{DiGraph, Graph};
use jgrapht_io::dimacs::{DimacsExporter, DimacsFormat, DimacsImporter};
use jgrapht_io::{GraphExporter, GraphImporter, Result};

fn main() -> Result<()> {
    let input = "p sp 3 2\na 1 2 1.5\na 2 3 2.5\n";
    let mut graph = DiGraph::<usize>::multi()
        .weighted()
        .with_vertex_factory(|id| id as usize);
    DimacsImporter::new().import_str(&mut graph, input)?;

    let mut exporter =
        DimacsExporter::with_vertex_id_provider(|vertex: &usize| (vertex + 1).to_string());
    exporter.set_format(DimacsFormat::ShortestPath);
    print!("{}", exporter.export_to_string(&graph)?);
    assert_eq!(graph.edge_count(), 2);
    Ok(())
}
