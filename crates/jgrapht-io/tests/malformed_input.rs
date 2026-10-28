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

//! Every importer must answer corrupted input with `Err` (or a graph), never a panic.
//!
//! Each seed document is cut at every length and mutated one byte at a time with a fixed pattern
//! of replacements, so the run is deterministic.

use jgrapht::{DefaultEdge, UGraph};
use jgrapht_io::GraphImporter;
use jgrapht_io::csv::{CsvFormat, CsvImporter};
use jgrapht_io::dimacs::DimacsImporter;
use jgrapht_io::dot::DotImporter;
use jgrapht_io::gexf::GexfImporter;
use jgrapht_io::gml::GmlImporter;
use jgrapht_io::graph6::Graph6Importer;
use jgrapht_io::graphml::GraphMlImporter;
use jgrapht_io::json::JsonImporter;
use jgrapht_io::tsplib::TsplibImporter;
use std::panic::{AssertUnwindSafe, catch_unwind};

const REPLACEMENTS: [u8; 8] = [b'0', b'-', b'"', b'\\', b'{', b'<', b' ', 0xC3];

fn target() -> UGraph<usize> {
    UGraph::<usize>::pseudo()
        .weighted()
        .with_vertex_factory(|id| id as usize)
}

fn variants(seed: &str) -> Vec<String> {
    let bytes = seed.as_bytes();
    let mut out = Vec::new();
    for end in 0..=bytes.len() {
        out.push(String::from_utf8_lossy(&bytes[..end]).into_owned());
    }
    for i in 0..bytes.len() {
        for &r in &REPLACEMENTS {
            let mut b = bytes.to_vec();
            b[i] = r;
            out.push(String::from_utf8_lossy(&b).into_owned());
        }
        let mut b = bytes.to_vec();
        b.remove(i);
        out.push(String::from_utf8_lossy(&b).into_owned());
    }
    out
}

fn check(name: &str, seed: &str, run: impl Fn(&str) -> jgrapht_io::Result<()>) {
    if let Err(error) = run(seed) {
        panic!("{name}: the unmodified seed must import: {error}");
    }
    for input in variants(seed) {
        if catch_unwind(AssertUnwindSafe(|| run(&input))).is_err() {
            panic!("{name} panicked on {input:?}");
        }
    }
}

#[test]
fn importers_do_not_panic_on_corrupted_input() {
    check("csv edge list", "a,b\nb,c\n", |s| {
        CsvImporter::new(CsvFormat::EdgeList).import_str(&mut target(), s)
    });
    check("csv adjacency", "a,b,c\nb,c\n", |s| {
        CsvImporter::new(CsvFormat::AdjacencyList).import_str(&mut target(), s)
    });
    check("csv matrix", "0,1\n1,0\n", |s| {
        CsvImporter::new(CsvFormat::Matrix).import_str(&mut target(), s)
    });
    check("dimacs", "c hi\np edge 3 2\ne 1 2\ne 2 3\n", |s| {
        DimacsImporter::new().import_str(&mut target(), s)
    });
    check(
        "dot",
        "graph G { a -- b [weight=2.5]; b -- c; \"d e\" -- a; }",
        |s| DotImporter::<usize, DefaultEdge>::new().import_str(&mut target(), s),
    );
    check(
        "gexf",
        r#"<gexf><graph defaultedgetype="undirected"><nodes><node id="1"/><node id="2"/></nodes><edges><edge id="0" source="1" target="2" weight="2"/></edges></graph></gexf>"#,
        |s| GexfImporter::<usize, DefaultEdge>::new().import_str(&mut target(), s),
    );
    check(
        "gml",
        "graph [ directed 0 node [ id 1 label \"a\" ] node [ id 2 ] edge [ source 1 target 2 weight 3.5 ] ]",
        |s| GmlImporter::<usize, DefaultEdge>::new().import_str(&mut target(), s),
    );
    check("graph6", "D?{", |s| {
        Graph6Importer::new().import_str(&mut target(), s)
    });
    check(
        "graphml",
        r#"<graphml><key id="w" for="edge" attr.name="weight" attr.type="double"/><graph edgedefault="undirected"><node id="1"/><node id="2"/><edge source="1" target="2"><data key="w">2</data></edge></graph></graphml>"#,
        |s| GraphMlImporter::<usize, DefaultEdge>::new().import_str(&mut target(), s),
    );
    check(
        "json",
        r#"{"nodes":[{"id":"1"},{"id":"2","label":"\u00e9\ud83d\ude00"}],"edges":[{"source":"1","target":"2","weight":1.5e2}]}"#,
        |s| JsonImporter::<usize, DefaultEdge>::new().import_str(&mut target(), s),
    );
    check(
        "tsplib",
        "NAME: t\nTYPE: TSP\nDIMENSION: 3\nEDGE_WEIGHT_TYPE: EUC_2D\nNODE_COORD_SECTION\n1 0 0\n2 3 4\n3 6 8\nEOF\n",
        |s| TsplibImporter::new().import_str(&mut target(), s),
    );
}
