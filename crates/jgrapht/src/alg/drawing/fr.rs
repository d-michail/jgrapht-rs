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

//! Fruchterman–Reingold force-directed layout, with and without a quad tree index
//! (`FRLayoutAlgorithm2D`, `IndexedFRLayoutAlgorithm2D`, `FRQuadTree`).

use super::layouts::{Initializer, LayoutAlgorithm2D};
use super::model::{Box2D, LayoutModel2D, MapLayoutModel2D, Point2D, boxes, points};
use crate::graph::Graph;
use crate::util::ToleranceDoubleComparator;
use crate::util::java::RandomSource;
use crate::{GraphError, Result};
use std::collections::{HashMap, VecDeque};

const NW: usize = 0;
const NE: usize = 1;
const SW: usize = 2;
const SE: usize = 3;

#[derive(Debug, Clone)]
struct QNode {
    boxx: Box2D,
    total_points: usize,
    centroid: Option<Point2D>,
    children: Option<[usize; 4]>,
    /// `Some` for leaves.
    points: Option<Vec<Point2D>>,
}

/// Quad tree over points for Barnes–Hut style approximations (`FRQuadTree`).
#[derive(Debug, Clone)]
pub struct FRQuadTree {
    nodes: Vec<QNode>,
}

/// Handle to a node of an [`FRQuadTree`].
pub type NodeId = usize;

impl FRQuadTree {
    /// Create a new tree for a certain area.
    pub fn new(boxx: Box2D) -> Self {
        FRQuadTree {
            nodes: vec![QNode {
                boxx,
                total_points: 0,
                centroid: None,
                children: None,
                points: Some(Vec::new()),
            }],
        }
    }

    fn new_node(&mut self, boxx: Box2D) -> usize {
        self.nodes.push(QNode {
            boxx,
            total_points: 0,
            centroid: None,
            children: None,
            points: Some(Vec::new()),
        });
        self.nodes.len() - 1
    }

    /// Get the root node of the tree.
    pub fn root(&self) -> NodeId {
        0
    }

    /// Insert a new point.
    ///
    /// # Errors
    ///
    /// Returns a [`GraphError`] if an argument violates a precondition; errors of the underlying graph operations are propagated.
    ///
    /// # Panics
    ///
    /// Panics only if an internal invariant is violated, which indicates a bug in this library.
    pub fn insert(&mut self, p: Point2D) -> Result<()> {
        let mut cur = 0;
        loop {
            if self.nodes[cur].points.is_some() {
                if self.nodes[cur].points.as_ref().unwrap().is_empty() {
                    self.nodes[cur].points.as_mut().unwrap().push(p);
                    return Ok(());
                }
                // split the leaf
                let rect = self.nodes[cur].boxx;
                let (west_all, east_all) = boxes::split_along_x_axis(&rect);
                let west = boxes::split_along_y_axis(&west_all);
                let east = boxes::split_along_y_axis(&east_all);
                let mut kids = [0usize; 4];
                kids[NW] = self.new_node(west.1);
                kids[NE] = self.new_node(east.1);
                kids[SW] = self.new_node(west.0);
                kids[SE] = self.new_node(east.0);
                let old = self.nodes[cur].points.take().unwrap();
                let (mut cx, mut cy) = (0.0, 0.0);
                for point in &old {
                    for &k in &[NW, NE, SW, SE] {
                        if boxes::contains_point(&self.nodes[kids[k]].boxx, *point) {
                            self.nodes[kids[k]].points.as_mut().unwrap().push(*point);
                            break;
                        }
                    }
                    cx += point.x;
                    cy += point.y;
                }
                let n = old.len();
                self.nodes[cur].total_points = n;
                self.nodes[cur].centroid = Some(Point2D::of(cx / n as f64, cy / n as f64));
                self.nodes[cur].children = Some(kids);
            }
            let node = &mut self.nodes[cur];
            node.total_points += 1;
            let c = node.centroid.unwrap();
            let t = node.total_points as f64;
            node.centroid = Some(Point2D::of(
                (c.x * (t - 1.0) + p.x) / t,
                (c.y * (t - 1.0) + p.y) / t,
            ));
            let kids = node.children.unwrap();
            let next = [NW, NE, SW, SE]
                .into_iter()
                .find(|&k| boxes::contains_point(&self.nodes[kids[k]].boxx, p));
            match next {
                Some(k) => cur = kids[k],
                None => return Err(GraphError::InvalidArgument("point outside the tree".into())),
            }
        }
    }

    /// Check if a node is a leaf.
    pub fn is_leaf(&self, n: NodeId) -> bool {
        self.nodes[n].points.is_some()
    }

    /// Returns the box of.
    pub fn box_of(&self, n: NodeId) -> Box2D {
        self.nodes[n].boxx
    }

    /// Check if the node contains any points.
    pub fn has_points(&self, n: NodeId) -> bool {
        match &self.nodes[n].points {
            Some(p) => !p.is_empty(),
            None => self.nodes[n].total_points != 0,
        }
    }

    /// Get the total number of points under this node.
    pub fn number_of_points(&self, n: NodeId) -> usize {
        match &self.nodes[n].points {
            Some(p) => p.len(),
            None => self.nodes[n].total_points,
        }
    }

    /// All points below `n`.
    pub fn points(&self, n: NodeId) -> Vec<Point2D> {
        match &self.nodes[n].points {
            Some(p) => p.clone(),
            None => self
                .children(n)
                .into_iter()
                .flat_map(|c| self.points(c))
                .collect(),
        }
    }

    /// Get the centroid of all points contained in this node.
    ///
    /// # Errors
    ///
    /// Returns a [`GraphError`] if an argument violates a precondition; errors of the underlying graph operations are propagated.
    ///
    /// # Panics
    ///
    /// Panics only if an internal invariant is violated, which indicates a bug in this library.
    pub fn centroid(&self, n: NodeId) -> Result<Point2D> {
        match &self.nodes[n].points {
            Some(p) => {
                if p.is_empty() {
                    return Err(GraphError::InvalidArgument("No points".into()));
                }
                let (x, y) = p.iter().fold((0.0, 0.0), |(x, y), q| (x + q.x, y + q.y));
                Ok(Point2D::of(x / p.len() as f64, y / p.len() as f64))
            }
            None => Ok(self.nodes[n].centroid.unwrap()),
        }
    }

    /// Children in the order north-west, north-east, south-west, south-east; empty for leaves.
    pub fn children(&self, n: NodeId) -> Vec<NodeId> {
        self.nodes[n]
            .children
            .map(|c| c.to_vec())
            .unwrap_or_default()
    }
}

/// Temperature as a function of the iteration and the iteration limit (`TemperatureModel`).
pub type TemperatureModel = Box<dyn Fn(usize, usize) -> f64>;
type TemperatureSupplier = Box<dyn Fn(&Box2D, usize) -> TemperatureModel>;

/// Linearly falling temperature `a * iteration + b`, zero in the last iteration.
pub fn inverse_linear_temperature(a: f64, b: f64) -> TemperatureModel {
    Box::new(move |iteration, max| {
        if iteration + 1 >= max {
            0.0
        } else {
            a * iteration as f64 + b
        }
    })
}

/// Fruchterman and Reingold Force-Directed Placement Algorithm.
///
/// The algorithm belongs in the broad category of
/// [force directed graph drawing](https://en.wikipedia.org/wiki/Force-directed_graph_drawing) algorithms and is described in the paper:
///
/// - Thomas M. J. Fruchterman and Edward M. Reingold. Graph drawing by force-directed placement.
///   Software: Practice and experience, 21(11):1129--1164, 1991.
///
/// # Rust notes
///
/// The Fruchterman–Reingold layout (`FRLayoutAlgorithm2D`); with a `theta` (see
/// [`indexed`](Self::indexed)) repulsive forces are approximated through a quad tree
/// (`IndexedFRLayoutAlgorithm2D`).
pub struct FRLayoutAlgorithm2D<V, R> {
    rng: R,
    iterations: usize,
    normalization_factor: f64,
    temperature_model_supplier: TemperatureSupplier,
    comparator: ToleranceDoubleComparator,
    theta: Option<f64>,
    saved_comparisons: u64,
    optimal_distance: f64,
    /// The initializer.
    pub initializer: Option<Initializer<V>>,
}

/// The default iterations constant.
pub const DEFAULT_ITERATIONS: usize = 100;
/// The default normalization factor constant.
pub const DEFAULT_NORMALIZATION_FACTOR: f64 = 0.5;
/// The default theta factor constant.
pub const DEFAULT_THETA_FACTOR: f64 = 0.5;

impl<V, R: RandomSource> FRLayoutAlgorithm2D<V, R> {
    /// Create a new layout algorithm
    ///
    /// # Panics
    ///
    /// Panics only if an internal invariant is violated, which indicates a bug in this library.
    pub fn new(iterations: usize, normalization_factor: f64, rng: R) -> Self {
        Self::with_tolerance(
            iterations,
            normalization_factor,
            rng,
            ToleranceDoubleComparator::DEFAULT_EPSILON,
        )
        .unwrap()
    }

    /// Creates an instance with the given tolerance.
    ///
    /// # Errors
    ///
    /// Returns a [`GraphError`] propagated from the underlying graph operations or from validating the input.
    pub fn with_tolerance(
        iterations: usize,
        normalization_factor: f64,
        rng: R,
        tolerance: f64,
    ) -> Result<Self> {
        Ok(FRLayoutAlgorithm2D {
            rng,
            iterations,
            normalization_factor,
            temperature_model_supplier: Box::new(|area, total| {
                let dimension = area.width().min(area.height());
                inverse_linear_temperature(-dimension / (10.0 * total as f64), dimension / 10.0)
            }),
            comparator: ToleranceDoubleComparator::new(tolerance)?,
            theta: None,
            saved_comparisons: 0,
            optimal_distance: 0.0,
            initializer: None,
        })
    }

    /// The indexed variant: `theta` in `[0, 1]` trades accuracy for speed (0 is exact).
    ///
    /// # Errors
    ///
    /// Returns a [`GraphError`] if an argument violates a precondition; errors of the underlying graph operations are propagated.
    pub fn indexed(
        iterations: usize,
        theta: f64,
        normalization_factor: f64,
        rng: R,
    ) -> Result<Self> {
        if !(0.0..=1.0).contains(&theta) {
            return Err(GraphError::InvalidArgument("Illegal theta value".into()));
        }
        let mut alg = Self::new(iterations, normalization_factor, rng);
        alg.theta = Some(theta);
        Ok(alg)
    }

    /// Creates an instance with the given temperature model.
    pub fn with_temperature_model(
        mut self,
        supplier: impl Fn(&Box2D, usize) -> TemperatureModel + 'static,
    ) -> Self {
        self.temperature_model_supplier = Box::new(supplier);
        self
    }

    /// Repulsive force computations avoided by the quad tree in the last layout.
    pub fn saved_comparisons(&self) -> u64 {
        self.saved_comparisons
    }

    /// Calculate the attractive force.
    fn attractive_force(&self, d: f64) -> f64 {
        d * d / self.optimal_distance
    }

    /// Calculate the repulsive force.
    fn repulsive_force(&self, d: f64) -> f64 {
        self.optimal_distance * self.optimal_distance / d
    }

    fn differs(&self, a: Point2D, b: Point2D) -> bool {
        self.comparator.compare(a.x, b.x) != 0 || self.comparator.compare(a.y, b.y) != 0
    }

    fn position<M: LayoutModel2D<V>>(model: &M, v: &V) -> Point2D {
        model.get(v).expect("every vertex has a position")
    }
}

impl<V: Eq + std::hash::Hash + Clone, R: RandomSource> FRLayoutAlgorithm2D<V, R> {
    fn repulsive_forces<G: Graph<Vertex = V>, M: LayoutModel2D<V>>(
        &mut self,
        graph: &G,
        model: &M,
    ) -> HashMap<V, Point2D> {
        let area = model.drawable_area();
        let origin = Point2D::of(area.min_x(), area.min_y());
        match self.theta {
            None => self.exact_repulsive_forces(graph, model, origin),
            Some(theta) => self.indexed_repulsive_forces(graph, model, origin, theta),
        }
    }

    fn exact_repulsive_forces<G: Graph<Vertex = V>, M: LayoutModel2D<V>>(
        &self,
        graph: &G,
        model: &M,
        origin: Point2D,
    ) -> HashMap<V, Point2D> {
        let mut disp = HashMap::new();
        for v in graph.vertices() {
            let v_pos = points::subtract(Self::position(model, v), origin);
            let mut v_disp = Point2D::of(0.0, 0.0);
            for u in graph.vertices() {
                if v == u {
                    continue;
                }
                let u_pos = points::subtract(Self::position(model, u), origin);
                if self.differs(v_pos, u_pos) {
                    let delta = points::subtract(v_pos, u_pos);
                    let len = points::length(delta);
                    v_disp = points::add(
                        v_disp,
                        points::scalar_multiply(delta, self.repulsive_force(len) / len),
                    );
                }
            }
            disp.insert(v.clone(), v_disp);
        }
        disp
    }

    fn indexed_repulsive_forces<G: Graph<Vertex = V>, M: LayoutModel2D<V>>(
        &mut self,
        graph: &G,
        model: &M,
        origin: Point2D,
        theta: f64,
    ) -> HashMap<V, Point2D> {
        let mut tree = FRQuadTree::new(model.drawable_area());
        for v in graph.vertices() {
            tree.insert(Self::position(model, v))
                .expect("point inside the drawable area");
        }
        let mut disp = HashMap::new();
        for v in graph.vertices() {
            let v_pos = points::subtract(Self::position(model, v), origin);
            let mut v_disp = Point2D::of(0.0, 0.0);
            let mut queue = VecDeque::from([tree.root()]);
            while let Some(node) = queue.pop_front() {
                let box_width = tree.box_of(node).width();
                let u_pos;
                if tree.is_leaf(node) {
                    if !tree.has_points(node) {
                        continue;
                    }
                    u_pos = points::subtract(tree.points(node)[0], origin);
                } else {
                    let centroid = tree.centroid(node).unwrap();
                    let distance = points::length(points::subtract(v_pos, centroid));
                    if self.comparator.compare(distance, 0.0) == 0 {
                        self.saved_comparisons += tree.number_of_points(node) as u64 - 1;
                        continue;
                    } else if self.comparator.compare(box_width / distance, theta) < 0 {
                        u_pos = points::subtract(centroid, origin);
                        self.saved_comparisons += tree.number_of_points(node) as u64 - 1;
                    } else {
                        queue.extend(tree.children(node));
                        continue;
                    }
                }
                if self.differs(v_pos, u_pos) {
                    let delta = points::subtract(v_pos, u_pos);
                    let len = points::length(delta);
                    v_disp = points::add(
                        v_disp,
                        points::scalar_multiply(delta, self.repulsive_force(len) / len),
                    );
                }
            }
            disp.insert(v.clone(), v_disp);
        }
        disp
    }

    fn attractive_forces<G: Graph<Vertex = V>, M: LayoutModel2D<V>>(
        &self,
        graph: &G,
        model: &M,
    ) -> HashMap<V, Point2D> {
        let area = model.drawable_area();
        let origin = Point2D::of(area.min_x(), area.min_y());
        let mut disp: HashMap<V, Point2D> = HashMap::new();
        let zero = Point2D::of(0.0, 0.0);
        for e in graph.edges() {
            let (v, u) = (graph.edge_source(e).unwrap(), graph.edge_target(e).unwrap());
            let v_pos = points::subtract(Self::position(model, v), origin);
            let u_pos = points::subtract(Self::position(model, u), origin);
            if self.differs(v_pos, u_pos) {
                let delta = points::subtract(v_pos, u_pos);
                let len = points::length(delta);
                let contribution = points::scalar_multiply(delta, self.attractive_force(len) / len);
                let dv = disp.get(v).copied().unwrap_or(zero);
                disp.insert(v.clone(), points::add(dv, points::negate(contribution)));
                let du = disp.get(u).copied().unwrap_or(zero);
                disp.insert(u.clone(), points::add(du, contribution));
            }
        }
        disp
    }
}

impl<G: Graph, R: RandomSource> LayoutAlgorithm2D<G> for FRLayoutAlgorithm2D<G::Vertex, R> {
    /// Layout a graph.
    fn layout<M: LayoutModel2D<G::Vertex>>(&mut self, graph: &G, model: &mut M) -> Result<()> {
        self.saved_comparisons = 0;
        let area = model.drawable_area();
        let (min_x, min_y) = (area.min_x(), area.min_y());
        if let Some(f) = &self.initializer {
            for v in graph.vertices() {
                if let Some(p) = f(v) {
                    model.put(v.clone(), p);
                }
            }
            for v in graph.vertices() {
                if model.get(v).is_none() {
                    model.put(v.clone(), Point2D::of(min_x, min_y));
                }
            }
        } else {
            // a random layout in a temporary model, as in Java
            let mut random = MapLayoutModel2D::new(area);
            let (rmin_x, rmin_y) = (area.min_x(), area.min_x());
            for v in graph.vertices() {
                let x = self.rng.next_double() * area.width();
                let y = self.rng.next_double() * area.height();
                random.put(v.clone(), Point2D::of(rmin_x + x, rmin_y + y));
            }
            for v in graph.vertices() {
                model.put(v.clone(), random.get(v).unwrap());
            }
        }
        let (width, height) = (area.width(), area.height());
        let n = graph.vertex_count();
        if n == 0 {
            return Ok(());
        }
        self.optimal_distance = self.normalization_factor * (width * height / n as f64).sqrt();
        let temperature =
            (self.temperature_model_supplier)(&model.drawable_area(), self.iterations);
        for i in 0..self.iterations {
            let repulsive = self.repulsive_forces(graph, model);
            let attractive = self.attractive_forces(graph, model);
            let temp = temperature(i, self.iterations);
            for v in graph.vertices() {
                let zero = Point2D::of(0.0, 0.0);
                let v_disp = points::add(repulsive[v], attractive.get(v).copied().unwrap_or(zero));
                if self.comparator.compare(v_disp.x, 0.0) != 0
                    || self.comparator.compare(v_disp.y, 0.0) != 0
                {
                    let len = points::length(v_disp);
                    let pos = points::add(
                        model.get(v).unwrap(),
                        points::scalar_multiply(v_disp, len.min(temp) / len),
                    );
                    let pos = Point2D::of(
                        (min_x + width).min(min_x.max(pos.x)),
                        (min_y + height).min(min_y.max(pos.y)),
                    );
                    model.put(v.clone(), pos);
                }
            }
        }
        Ok(())
    }
}
