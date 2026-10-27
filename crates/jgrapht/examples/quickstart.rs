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

use jgrapht::alg::shortestpath::{Dijkstra, ShortestPathAlgorithm};
use jgrapht::alg::spanning::kruskal;
use jgrapht::*;

fn main() -> Result<()> {
    let mut graph = UGraph::<&str>::simple().weighted();
    graph.add_vertices(["a", "b", "c", "d"]);
    graph.add_weighted_edge(&"a", &"b", 1.0)?;
    graph.add_weighted_edge(&"b", &"c", 2.0)?;
    graph.add_weighted_edge(&"a", &"c", 5.0)?;
    graph.add_weighted_edge(&"c", &"d", 1.0)?;

    let path = Dijkstra::new(&graph)
        .path(&"a", &"d")?
        .expect("d is reachable");
    println!("a -> d: {:?} (weight {})", path.vertices, path.weight);
    println!("MST weight: {}", kruskal(&graph).weight);
    Ok(())
}
