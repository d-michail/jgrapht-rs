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

//! Simple graph layouts. Positions are `(x, y)` pairs.
//!
//! [`model`] holds the layout model types; the layouts that write into a model are in [`layouts`].

pub mod fr;
pub mod layouts;
pub mod model;

use crate::alg::util::Indexed;
use crate::graph::Graph;
use rand::Rng;
use std::borrow::Borrow;
use std::collections::HashMap;

/// A point as an `(x, y)` pair.
pub type Point = (f64, f64);

/// Vertices evenly spaced on a circle of the given radius centred at the origin.
pub fn circular_layout<G: Graph>(g: &G, radius: f64) -> HashMap<G::Vertex, Point> {
    let n = g.vertex_count().max(1) as f64;
    g.vertices()
        .enumerate()
        .map(|(i, v)| {
            let a = 2.0 * std::f64::consts::PI * i as f64 / n;
            (v.borrow().clone(), (radius * a.cos(), radius * a.sin()))
        })
        .collect()
}

/// Vertices placed uniformly at random in `[0, width] x [0, height]`.
pub fn random_layout<G: Graph>(
    g: &G,
    width: f64,
    height: f64,
    rng: &mut impl Rng,
) -> HashMap<G::Vertex, Point> {
    g.vertices()
        .map(|v| {
            let v = v.borrow();
            {
                (
                    v.clone(),
                    (rng.random::<f64>() * width, rng.random::<f64>() * height),
                )
            }
        })
        .collect()
}

/// Fruchterman–Reingold force-directed layout inside `[0, width] x [0, height]`.
pub fn fruchterman_reingold<G: Graph>(
    g: &G,
    width: f64,
    height: f64,
    iterations: usize,
    rng: &mut impl Rng,
) -> HashMap<G::Vertex, Point> {
    let ix = Indexed::new(g);
    let n = ix.n();
    if n == 0 {
        return HashMap::new();
    }
    let mut pos: Vec<Point> = (0..n)
        .map(|_| (rng.random::<f64>() * width, rng.random::<f64>() * height))
        .collect();
    let k = (width * height / n as f64).sqrt();
    let mut t = width.min(height) / 10.0;
    let cool = t / (iterations.max(1) as f64 + 1.0);
    for _ in 0..iterations {
        let mut disp = vec![(0.0f64, 0.0f64); n];
        for a in 0..n {
            for b in a + 1..n {
                let (dx, dy) = (pos[a].0 - pos[b].0, pos[a].1 - pos[b].1);
                let d = (dx * dx + dy * dy).sqrt().max(1e-9);
                let f = k * k / d;
                let (fx, fy) = (dx / d * f, dy / d * f);
                disp[a] = (disp[a].0 + fx, disp[a].1 + fy);
                disp[b] = (disp[b].0 - fx, disp[b].1 - fy);
            }
        }
        for &(a, b) in &ix.ends {
            if a == b {
                continue;
            }
            let (dx, dy) = (pos[a].0 - pos[b].0, pos[a].1 - pos[b].1);
            let d = (dx * dx + dy * dy).sqrt().max(1e-9);
            let f = d * d / k;
            let (fx, fy) = (dx / d * f, dy / d * f);
            disp[a] = (disp[a].0 - fx, disp[a].1 - fy);
            disp[b] = (disp[b].0 + fx, disp[b].1 + fy);
        }
        for v in 0..n {
            let d = (disp[v].0.powi(2) + disp[v].1.powi(2)).sqrt().max(1e-9);
            let step = d.min(t);
            pos[v].0 = (pos[v].0 + disp[v].0 / d * step).clamp(0.0, width);
            pos[v].1 = (pos[v].1 + disp[v].1 / d * step).clamp(0.0, height);
        }
        t -= cool;
    }
    (0..n).map(|v| (ix.vertex(v), pos[v])).collect()
}
