//! Layout model: points, boxes and the maps from vertices to positions
//! (`org.jgrapht.alg.drawing.model`).

use indexmap::IndexMap;
use std::hash::{Hash, Hasher};

/// A point in the plane. Equality and hashing compare the bit patterns of the coordinates, like
/// Java's `Double.doubleToLongBits`: `NaN == NaN` and `0.0 != -0.0`.
#[derive(Debug, Clone, Copy)]
pub struct Point2D {
    pub x: f64,
    pub y: f64,
}

impl Point2D {
    pub fn of(x: f64, y: f64) -> Self {
        Point2D { x, y }
    }
    pub fn num_dimensions(&self) -> usize {
        2
    }
}

impl PartialEq for Point2D {
    fn eq(&self, o: &Self) -> bool {
        self.x.to_bits() == o.x.to_bits() && self.y.to_bits() == o.y.to_bits()
    }
}
impl Eq for Point2D {}
impl Hash for Point2D {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.x.to_bits().hash(state);
        self.y.to_bits().hash(state);
    }
}
impl std::fmt::Display for Point2D {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({}, {})", self.x, self.y)
    }
}

/// Operations on [`Point2D`] (`Points`).
pub mod points {
    use super::Point2D;
    use crate::util::ToleranceDoubleComparator;

    pub fn length(v: Point2D) -> f64 {
        (v.x * v.x + v.y * v.y).sqrt()
    }
    pub fn add(a: Point2D, b: Point2D) -> Point2D {
        Point2D::of(a.x + b.x, a.y + b.y)
    }
    pub fn subtract(a: Point2D, b: Point2D) -> Point2D {
        Point2D::of(a.x - b.x, a.y - b.y)
    }
    pub fn negate(a: Point2D) -> Point2D {
        scalar_multiply(a, -1.0)
    }
    pub fn scalar_multiply(a: Point2D, scalar: f64) -> Point2D {
        Point2D::of(a.x * scalar, a.y * scalar)
    }
    /// Equal up to a tolerance of `1e-9` in each coordinate.
    pub fn equals(p1: Point2D, p2: Point2D) -> bool {
        let c = ToleranceDoubleComparator::new(1e-9).expect("positive tolerance");
        c.compare(p1.x, p2.x) == 0 && c.compare(p1.y, p2.y) == 0
    }
}

/// An axis-parallel box given by its lower-left corner and its sides (`Box2D`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Box2D {
    min_x: f64,
    min_y: f64,
    width: f64,
    height: f64,
}

impl Box2D {
    /// A box with its corner at the origin.
    pub fn of(width: f64, height: f64) -> Self {
        Self::at(0.0, 0.0, width, height)
    }
    pub fn at(x: f64, y: f64, width: f64, height: f64) -> Self {
        debug_assert!(width >= 0.0 && height >= 0.0);
        Box2D {
            min_x: x,
            min_y: y,
            width,
            height,
        }
    }
    pub fn min_x(&self) -> f64 {
        self.min_x
    }
    pub fn min_y(&self) -> f64 {
        self.min_y
    }
    pub fn width(&self) -> f64 {
        self.width
    }
    pub fn height(&self) -> f64 {
        self.height
    }
    pub fn max_x(&self) -> f64 {
        self.min_x + self.width
    }
    pub fn max_y(&self) -> f64 {
        self.min_y + self.height
    }
}

/// Operations on [`Box2D`] (`Boxes`).
pub mod boxes {
    use super::{Box2D, Point2D};
    use std::cmp::Ordering;

    pub fn contains_point(b: &Box2D, p: Point2D) -> bool {
        contains_point_with(b, p, |a, b| a.total_cmp(&b))
    }

    /// Containment where coordinates are compared with `cmp`.
    pub fn contains_point_with(b: &Box2D, p: Point2D, cmp: impl Fn(f64, f64) -> Ordering) -> bool {
        cmp(p.x, b.min_x() + b.width()).is_le()
            && cmp(p.x, b.min_x()).is_ge()
            && cmp(p.y, b.min_y() + b.height()).is_le()
            && cmp(p.y, b.min_y()).is_ge()
    }

    /// The west and east halves.
    pub fn split_along_x_axis(b: &Box2D) -> (Box2D, Box2D) {
        let w = b.width() / 2.0;
        (
            Box2D::at(b.min_x(), b.min_y(), w, b.height()),
            Box2D::at(b.min_x() + w, b.min_y(), w, b.height()),
        )
    }

    /// The south and north halves.
    pub fn split_along_y_axis(b: &Box2D) -> (Box2D, Box2D) {
        let h = b.height() / 2.0;
        (
            Box2D::at(b.min_x(), b.min_y(), b.width(), h),
            Box2D::at(b.min_x(), b.min_y() + h, b.width(), h),
        )
    }
}

/// Positions of vertices inside a drawable area (`LayoutModel2D`).
pub trait LayoutModel2D<V> {
    fn drawable_area(&self) -> Box2D;
    fn set_drawable_area(&mut self, area: Box2D);
    fn get(&self, v: &V) -> Option<Point2D>;
    /// Sets the position unless the vertex is fixed; returns the previous (or, for a fixed
    /// vertex, the current) position.
    fn put(&mut self, v: V, p: Point2D) -> Option<Point2D>;
    fn set_fixed(&mut self, v: V, fixed: bool);
    fn is_fixed(&self, v: &V) -> bool;
    /// All positions in insertion order.
    fn entries(&self) -> Vec<(V, Point2D)>;

    fn collect(&self) -> IndexMap<V, Point2D>
    where
        V: Eq + Hash,
    {
        self.entries().into_iter().collect()
    }
}

/// A layout model backed by an insertion-ordered map (`MapLayoutModel2D`).
#[derive(Debug, Clone)]
pub struct MapLayoutModel2D<V: Eq + Hash> {
    area: Box2D,
    points: IndexMap<V, Point2D>,
    fixed: std::collections::HashSet<V>,
}

impl<V: Eq + Hash> MapLayoutModel2D<V> {
    pub fn new(area: Box2D) -> Self {
        MapLayoutModel2D {
            area,
            points: IndexMap::new(),
            fixed: Default::default(),
        }
    }
}

impl<V: Eq + Hash + Clone> LayoutModel2D<V> for MapLayoutModel2D<V> {
    fn drawable_area(&self) -> Box2D {
        self.area
    }
    fn set_drawable_area(&mut self, area: Box2D) {
        self.area = area;
    }
    fn get(&self, v: &V) -> Option<Point2D> {
        self.points.get(v).copied()
    }
    fn put(&mut self, v: V, p: Point2D) -> Option<Point2D> {
        if !self.fixed.contains(&v) {
            self.points.insert(v, p)
        } else {
            // putIfAbsent
            match self.points.get(&v) {
                Some(old) => Some(*old),
                None => {
                    self.points.insert(v, p);
                    None
                }
            }
        }
    }
    fn set_fixed(&mut self, v: V, fixed: bool) {
        if fixed {
            self.fixed.insert(v);
        } else {
            self.fixed.remove(&v);
        }
    }
    fn is_fixed(&self, v: &V) -> bool {
        self.fixed.contains(v)
    }
    fn entries(&self) -> Vec<(V, Point2D)> {
        self.points.iter().map(|(v, p)| (v.clone(), *p)).collect()
    }
}

/// A layout model that notifies listeners about every accepted position change
/// (`ListenableLayoutModel2D`).
pub struct ListenableLayoutModel2D<V, M: LayoutModel2D<V>> {
    model: M,
    listeners: Vec<(usize, Box<dyn FnMut(&V, Point2D)>)>,
    next_id: usize,
}

impl<V, M: LayoutModel2D<V>> ListenableLayoutModel2D<V, M> {
    pub fn new(model: M) -> Self {
        ListenableLayoutModel2D {
            model,
            listeners: Vec::new(),
            next_id: 0,
        }
    }
    /// Registers a listener; returns an id for [`remove_listener`](Self::remove_listener).
    pub fn add_listener(&mut self, listener: impl FnMut(&V, Point2D) + 'static) -> usize {
        self.next_id += 1;
        self.listeners.push((self.next_id, Box::new(listener)));
        self.next_id
    }
    pub fn remove_listener(&mut self, id: usize) -> bool {
        let before = self.listeners.len();
        self.listeners.retain(|(i, _)| *i != id);
        before != self.listeners.len()
    }
    pub fn into_inner(self) -> M {
        self.model
    }
}

impl<V: Clone, M: LayoutModel2D<V>> LayoutModel2D<V> for ListenableLayoutModel2D<V, M> {
    fn drawable_area(&self) -> Box2D {
        self.model.drawable_area()
    }
    fn set_drawable_area(&mut self, area: Box2D) {
        self.model.set_drawable_area(area);
    }
    fn get(&self, v: &V) -> Option<Point2D> {
        self.model.get(v)
    }
    fn put(&mut self, v: V, p: Point2D) -> Option<Point2D> {
        if !self.model.is_fixed(&v) {
            let old = self.model.put(v.clone(), p);
            for (_, l) in &mut self.listeners {
                l(&v, p);
            }
            old
        } else {
            self.model.get(&v)
        }
    }
    fn set_fixed(&mut self, v: V, fixed: bool) {
        self.model.set_fixed(v, fixed);
    }
    fn is_fixed(&self, v: &V) -> bool {
        self.model.is_fixed(v)
    }
    fn entries(&self) -> Vec<(V, Point2D)> {
        self.model.entries()
    }
}
