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

//! Layout algorithms that write vertex positions into a [`LayoutModel2D`].

use super::model::{Box2D, LayoutModel2D, Point2D};
use crate::alg::interfaces::PartitioningAlgorithm;
use crate::alg::partition::BipartitePartitioning;
use crate::graph::IncidenceGraph;
use crate::graph::{Graph, GraphExt};
use crate::util::ToleranceDoubleComparator;
use crate::util::java::RandomSource;
use crate::{GraphError, Result};
use rand::{SeedableRng, rngs::StdRng};
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::hash::Hash;

/// A general interface for a layout 2D algorithm.
///
/// A layout algorithm takes as input a graph and computes point coordinates for each of the graph
/// vertices. Details such as the dimensions of the drawable area, the storage of the vertices'
/// coordinates, etc. are provided using a `LayoutModel2D`.
///
/// # Rust notes
///
/// A layout algorithm (`LayoutAlgorithm2D`).
pub trait LayoutAlgorithm2D<G: Graph> {
    /// Returns the layout.
    ///
    /// # Errors
    ///
    /// Returns an error if a layout parameter is invalid or the model lacks a position the algorithm needs.
    fn layout<M: LayoutModel2D<G::Vertex>>(&mut self, graph: &G, model: &mut M) -> Result<()>;
}

/// Optional start positions applied before the layout proper (`BaseLayoutAlgorithm2D`).
pub type Initializer<V> = Box<dyn Fn(&V) -> Option<Point2D>>;

fn init<G: Graph, M: LayoutModel2D<G::Vertex>>(
    initializer: &Option<Initializer<G::Vertex>>,
    graph: &G,
    model: &mut M,
) {
    if let Some(f) = initializer {
        for v in graph.vertices() {
            if let Some(p) = f(v) {
                model.put(v.clone(), p);
            }
        }
    }
}

fn tolerance() -> ToleranceDoubleComparator {
    ToleranceDoubleComparator::new(ToleranceDoubleComparator::DEFAULT_EPSILON).unwrap()
}

/// Circular layout.
///
/// The algorithm places the graph vertices on a circle evenly spaced. The vertices are iterated
/// based on the iteration order of the vertex set of the graph. The order can be adjusted by
/// providing an external comparator.
///
/// # Rust notes
///
/// Vertices evenly spaced on a circle (`CircularLayoutAlgorithm2D`).
pub struct CircularLayoutAlgorithm2D<V> {
    radius: f64,
    vertex_comparator: Option<Box<dyn Fn(&V, &V) -> Ordering>>,
    /// The initializer.
    pub initializer: Option<Initializer<V>>,
}

impl<V> CircularLayoutAlgorithm2D<V> {
    /// Create a new layout algorithm
    ///
    /// # Errors
    ///
    /// Returns a [`GraphError`] if a numeric or collection parameter is out of range; errors of the underlying graph operations are propagated.
    pub fn new(radius: f64) -> Result<Self> {
        if tolerance().compare(radius, 0.0) <= 0 {
            return Err(GraphError::must_be_positive("radius"));
        }
        Ok(CircularLayoutAlgorithm2D {
            radius,
            vertex_comparator: None,
            initializer: None,
        })
    }

    /// Vertices are placed in the order given by `cmp`.
    pub fn with_vertex_comparator(mut self, cmp: impl Fn(&V, &V) -> Ordering + 'static) -> Self {
        self.vertex_comparator = Some(Box::new(cmp));
        self
    }
}

impl<V> Default for CircularLayoutAlgorithm2D<V> {
    fn default() -> Self {
        Self::new(0.5).unwrap()
    }
}

impl<G: IncidenceGraph> LayoutAlgorithm2D<G> for CircularLayoutAlgorithm2D<G::Vertex> {
    /// Layout a graph.
    fn layout<M: LayoutModel2D<G::Vertex>>(&mut self, graph: &G, model: &mut M) -> Result<()> {
        init(&self.initializer, graph, model);
        let area = model.drawable_area();
        let (width, height) = (area.width(), area.height());
        let cmp = tolerance();
        if cmp.compare(2.0 * self.radius, width) > 0 {
            return Err(GraphError::InvalidArgument(
                "Circle does not fit into drawable area width".into(),
            ));
        }
        if cmp.compare(2.0 * self.radius, height) > 0 {
            return Err(GraphError::InvalidArgument(
                "Circle does not fit into drawable area height".into(),
            ));
        }
        let n = graph.vertex_count();
        let angle_step = 2.0 * std::f64::consts::PI / n as f64;
        let mut vertices: Vec<&G::Vertex> = graph.vertices().collect();
        if let Some(c) = &self.vertex_comparator {
            vertices.sort_by(|a, b| c(a, b));
        }
        for (i, v) in vertices.into_iter().enumerate() {
            let x = self.radius * (angle_step * i as f64).cos() + width / 2.0;
            let y = self.radius * (angle_step * i as f64).sin() + height / 2.0;
            model.put(v.clone(), Point2D::of(area.min_x() + x, area.min_y() + y));
        }
        Ok(())
    }
}

/// Random layout. The algorithm assigns vertex coordinates uniformly at random.
///
/// # Rust notes
///
/// Uniformly random positions (`RandomLayoutAlgorithm2D`).
pub struct RandomLayoutAlgorithm2D<R = StdRng> {
    rng: R,
}

impl RandomLayoutAlgorithm2D<StdRng> {
    /// A layout drawn from a seeded `StdRng`.
    pub fn with_seed(seed: i64) -> Self {
        Self::new(StdRng::seed_from_u64(seed as u64))
    }
}

impl<R: RandomSource> RandomLayoutAlgorithm2D<R> {
    /// Create a new layout algorithm
    pub fn new(rng: R) -> Self {
        RandomLayoutAlgorithm2D { rng }
    }
}

impl<G: IncidenceGraph, R: RandomSource> LayoutAlgorithm2D<G> for RandomLayoutAlgorithm2D<R> {
    /// Layout a graph.
    fn layout<M: LayoutModel2D<G::Vertex>>(&mut self, graph: &G, model: &mut M) -> Result<()> {
        let area = model.drawable_area();
        // like the Java original, both offsets use the area's minimum x
        let (min_x, min_y) = (area.min_x(), area.min_x());
        for v in graph.vertices() {
            let x = self.rng.next_double() * area.width();
            let y = self.rng.next_double() * area.height();
            model.put(v.clone(), Point2D::of(min_x + x, min_y + y));
        }
        Ok(())
    }
}

/// A layout algorithm which re-scales vertex positions to (center-scale,center+scale) in all
/// dimensions.
///
/// The algorithm first subtracts the mean on each axis separately, then all values are adjusted so
/// that the maximum magnitude becomes scale. The result is finally translated back to the old
/// center. This procedure preserves the aspect ratio.
///
/// # Rust notes
///
/// Recentres a layout and scales it to `[-scale, scale]` around the old centre
/// (`RescaleLayoutAlgorithm2D`).
pub struct RescaleLayoutAlgorithm2D {
    scale: f64,
}

impl RescaleLayoutAlgorithm2D {
    /// Create a new layout algorithm
    ///
    /// # Errors
    ///
    /// Returns a [`GraphError`] if a numeric or collection parameter is out of range; errors of the underlying graph operations are propagated.
    pub fn new(scale: f64) -> Result<Self> {
        if scale <= 0.0 {
            return Err(GraphError::must_be_positive("scale"));
        }
        Ok(RescaleLayoutAlgorithm2D { scale })
    }
}

impl<G: IncidenceGraph> LayoutAlgorithm2D<G> for RescaleLayoutAlgorithm2D {
    /// Layout a graph.
    fn layout<M: LayoutModel2D<G::Vertex>>(&mut self, graph: &G, model: &mut M) -> Result<()> {
        let old = model.drawable_area();
        let cx = old.min_x() + old.width() / 2.0;
        let cy = old.min_y() + old.height() / 2.0;
        let entries = model.entries();
        let (mut max_x, mut max_y) = (0.0f64, 0.0f64);
        if !entries.is_empty() {
            let mean_x = entries.iter().map(|(_, p)| p.x).sum::<f64>() / entries.len() as f64;
            for v in graph.vertices() {
                let p = model.get(v).ok_or_else(missing)?;
                let nx = p.x - mean_x;
                model.put(v.clone(), Point2D::of(nx, p.y));
                max_x = max_x.max(nx.abs());
            }
        }
        let entries = model.entries();
        if !entries.is_empty() {
            let mean_y = entries.iter().map(|(_, p)| p.y).sum::<f64>() / entries.len() as f64;
            for v in graph.vertices() {
                let p = model.get(v).ok_or_else(missing)?;
                let ny = p.y - mean_y;
                model.put(v.clone(), Point2D::of(p.x, ny));
                max_y = max_y.max(ny.abs());
            }
        }
        let all_max = max_x.max(max_y);
        if all_max > 0.0 {
            for v in graph.vertices() {
                let p = model.get(v).ok_or_else(missing)?;
                model.put(v.clone(), Point2D::of(cx + p.x * self.scale / all_max, p.y));
            }
            for v in graph.vertices() {
                let p = model.get(v).ok_or_else(missing)?;
                model.put(v.clone(), Point2D::of(p.x, cy + p.y * self.scale / all_max));
            }
        }
        model.set_drawable_area(Box2D::at(
            cx - self.scale,
            cy - self.scale,
            2.0 * self.scale,
            2.0 * self.scale,
        ));
        Ok(())
    }
}

fn missing() -> GraphError {
    GraphError::InvalidArgument("vertex has no position in the layout model".into())
}

/// How the second layer of a [`TwoLayeredBipartiteLayout2D`] is ordered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecondLayerOrder {
    /// Keep the (optionally compared) order.
    Plain,
    /// Order by the median position of the neighbours (`MedianGreedyTwoLayeredBipartiteLayout2D`).
    MedianGreedy,
    /// Order by the mean position of the neighbours (`BarycenterGreedyTwoLayeredBipartiteLayout2D`).
    BarycenterGreedy,
}

/// A two layered bipartite layout.
///
/// The algorithm draws a bipartite graph using straight edges. Vertices are arranged along two
/// vertical or horizontal lines. No attempt is made to minimize edge crossings.
///
/// The order of the vertices can be adjusted by providing a vertex comparator. Similarly the user
/// can also determine the two partitions or can let the algorithm compute them.
///
/// # Rust notes
///
/// Draws the two sides of a bipartite graph on two parallel lines
/// (`TwoLayeredBipartiteLayout2D` and its greedy subclasses).
pub struct TwoLayeredBipartiteLayout2D<V> {
    vertex_comparator: Option<Box<dyn Fn(&V, &V) -> Ordering>>,
    vertical: bool,
    partition: Option<HashSet<V>>,
    order: SecondLayerOrder,
}

impl<V> Default for TwoLayeredBipartiteLayout2D<V> {
    fn default() -> Self {
        TwoLayeredBipartiteLayout2D {
            vertex_comparator: None,
            vertical: true,
            partition: None,
            order: SecondLayerOrder::Plain,
        }
    }
}

impl<V> TwoLayeredBipartiteLayout2D<V> {
    /// Create a new layout
    pub fn new() -> Self {
        Self::default()
    }
    /// Computes the median greedy.
    pub fn median_greedy() -> Self {
        TwoLayeredBipartiteLayout2D {
            order: SecondLayerOrder::MedianGreedy,
            ..Self::default()
        }
    }
    /// Computes the barycenter greedy.
    pub fn barycenter_greedy() -> Self {
        TwoLayeredBipartiteLayout2D {
            order: SecondLayerOrder::BarycenterGreedy,
            ..Self::default()
        }
    }
    /// Adjust the vertex comparator which specifies the vertex order.
    pub fn with_vertex_comparator(mut self, cmp: impl Fn(&V, &V) -> Ordering + 'static) -> Self {
        self.vertex_comparator = Some(Box::new(cmp));
        self
    }
    /// Adjust whether the layout will be vertical or horizontal.
    pub fn with_vertical(mut self, vertical: bool) -> Self {
        self.vertical = vertical;
        self
    }
    /// Specify the first of the two bipartite partitions. If not provided, the algorithm computes a
    /// partitioning.
    pub fn with_first_partition(mut self, partition: HashSet<V>) -> Self {
        self.partition = Some(partition);
        self
    }
}

impl<V: Eq + Hash + Clone> TwoLayeredBipartiteLayout2D<V> {
    /// Compute the vertex partitions.
    #[allow(clippy::type_complexity)]
    fn compute_partitions<G: Graph<Vertex = V> + IncidenceGraph>(
        &self,
        graph: &G,
    ) -> Result<(Vec<V>, Vec<V>)> {
        let (mut left, mut right): (Vec<V>, Vec<V>) = (Vec::new(), Vec::new());
        if let Some(partition) = &self.partition {
            for v in graph.vertices() {
                if partition.contains(v) {
                    left.push(v.clone());
                } else {
                    right.push(v.clone());
                }
            }
            for e in graph.edges() {
                let (v, u) = (graph.edge_source(e).unwrap(), graph.edge_target(e).unwrap());
                if partition.contains(v) == partition.contains(u) {
                    return Err(GraphError::InvalidArgument(
                        "Invalid provided bipartite partition.".into(),
                    ));
                }
            }
        } else {
            let mut alg = BipartitePartitioning::new(graph);
            let p = alg
                .partitioning()
                .ok_or(GraphError::InvalidGraph(crate::GraphProperty::Bipartite))?;
            left.extend(p.partition(0).iter().cloned());
            right.extend(p.partition(1).iter().cloned());
        }
        if let Some(c) = &self.vertex_comparator {
            left.sort_by(|a, b| c(a, b));
            right.sort_by(|a, b| c(a, b));
        }
        Ok((left, right))
    }

    fn step(&self, n: usize, area: &Box2D) -> f64 {
        if n > 1 {
            (if self.vertical {
                area.height()
            } else {
                area.width()
            }) / (n - 1) as f64
        } else {
            0.0
        }
    }

    fn draw_first<M: LayoutModel2D<V>>(&self, partition: &[V], model: &mut M) -> Result<()> {
        if partition.is_empty() {
            return Err(GraphError::InvalidParameter {
                name: "partition",
                requirement: crate::ParameterRequirement::NonEmpty,
            });
        }
        let a = model.drawable_area();
        let step = self.step(partition.len(), &a);
        let (mut x, mut y) = (a.min_x(), a.min_y());
        for v in partition {
            model.put(v.clone(), Point2D::of(x, y));
            if self.vertical {
                y += step;
            } else {
                x += step;
            }
        }
        Ok(())
    }

    fn draw_second<M: LayoutModel2D<V>>(&self, partition: &[V], model: &mut M) -> Result<()> {
        if partition.is_empty() {
            return Err(GraphError::InvalidParameter {
                name: "partition",
                requirement: crate::ParameterRequirement::NonEmpty,
            });
        }
        let a = model.drawable_area();
        let step = self.step(partition.len(), &a);
        let (mut x, mut y) = if self.vertical {
            (a.min_x() + a.width(), a.min_y())
        } else {
            (a.min_x(), a.min_y() + a.height())
        };
        for v in partition {
            model.put(v.clone(), Point2D::of(x, y));
            if self.vertical {
                y += step;
            } else {
                x += step;
            }
        }
        Ok(())
    }

    /// Sorts the second layer by barycenter or median of the neighbours' coordinates.
    fn greedy_sort<G: Graph<Vertex = V> + IncidenceGraph, M: LayoutModel2D<V>>(
        &self,
        graph: &G,
        partition: &mut [V],
        model: &M,
    ) {
        let coord = |u: &V| {
            let p = model.get(u).expect("first layer is drawn");
            if self.vertical { p.x } else { p.y }
        };
        let mut order: HashMap<V, (f64, usize)> = HashMap::new();
        for (i, v) in partition.iter().enumerate() {
            let key = match self.order {
                SecondLayerOrder::BarycenterGreedy => {
                    let degree = graph.degree_of(v);
                    if degree == 0 {
                        -f64::MAX
                    } else {
                        let sum: f64 = graph
                            .out_edges_of(v)
                            .map(|e| coord(graph.opposite_vertex(e, v).unwrap()))
                            .sum();
                        sum / degree as f64
                    }
                }
                _ => {
                    let mut other: Vec<f64> = graph
                        .edges_of(v)
                        .map(|e| coord(graph.opposite_vertex(e, v).unwrap()))
                        .collect();
                    other.sort_by(|a, b| a.total_cmp(b));
                    if other.is_empty() {
                        -f64::MAX
                    } else {
                        other[other.len() / 2]
                    }
                }
            };
            order.insert(v.clone(), (key, i));
        }
        partition.sort_by(|v, u| {
            let (pv, pu) = (order[v], order[u]);
            let d = pv.0.total_cmp(&pu.0);
            if d != Ordering::Equal {
                return d;
            }
            let (dv, du) = (graph.degree_of(v), graph.degree_of(u));
            if dv % 2 == 1 && du % 2 == 0 {
                return Ordering::Less;
            }
            if dv % 2 == 0 && du % 2 == 1 {
                return Ordering::Greater;
            }
            pv.1.cmp(&pu.1)
        });
    }
}

impl<G: IncidenceGraph> LayoutAlgorithm2D<G> for TwoLayeredBipartiteLayout2D<G::Vertex> {
    /// Layout a graph.
    fn layout<M: LayoutModel2D<G::Vertex>>(&mut self, graph: &G, model: &mut M) -> Result<()> {
        let (left, mut right) = self.compute_partitions(graph)?;
        self.draw_first(&left, model)?;
        if right.is_empty() {
            return Err(GraphError::InvalidParameter {
                name: "partition",
                requirement: crate::ParameterRequirement::NonEmpty,
            });
        }
        if self.order != SecondLayerOrder::Plain {
            self.greedy_sort(graph, &mut right, model);
        }
        self.draw_second(&right, model)
    }
}
