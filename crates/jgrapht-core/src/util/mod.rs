//! Small shared helpers.

pub mod heap;
pub mod java;
pub mod sampler;

use std::cmp::Ordering;

/// `f64` with a total order (`f64::total_cmp`), usable as a heap key.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OrderedF64(pub f64);

impl Eq for OrderedF64 {}
impl std::hash::Hash for OrderedF64 {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.to_bits().hash(state);
    }
}
impl PartialOrd for OrderedF64 {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for OrderedF64 {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.total_cmp(&other.0)
    }
}

/// Disjoint-set forest with path compression and union by size.
#[derive(Debug, Clone)]
pub struct UnionFind {
    parent: Vec<usize>,
    size: Vec<usize>,
    sets: usize,
}

impl UnionFind {
    pub fn new(n: usize) -> Self {
        UnionFind {
            parent: (0..n).collect(),
            size: vec![1; n],
            sets: n,
        }
    }
    pub fn find(&mut self, mut x: usize) -> usize {
        while self.parent[x] != x {
            self.parent[x] = self.parent[self.parent[x]];
            x = self.parent[x];
        }
        x
    }
    /// Merges the sets of `a` and `b`; returns `false` if they were already together.
    pub fn union(&mut self, a: usize, b: usize) -> bool {
        let (mut a, mut b) = (self.find(a), self.find(b));
        if a == b {
            return false;
        }
        if self.size[a] < self.size[b] {
            std::mem::swap(&mut a, &mut b);
        }
        self.parent[b] = a;
        self.size[a] += self.size[b];
        self.sets -= 1;
        true
    }
    pub fn connected(&mut self, a: usize, b: usize) -> bool {
        self.find(a) == self.find(b)
    }
    pub fn set_count(&self) -> usize {
        self.sets
    }
}

/// Bijection between a set of vertices and `0..n` (JGraphT's `VertexToIntegerMapping`).
#[derive(Debug, Clone)]
pub struct VertexToIntegerMapping<V> {
    vertex_map: std::collections::HashMap<V, usize>,
    index_list: Vec<V>,
}

impl<V: Clone + Eq + std::hash::Hash> VertexToIntegerMapping<V> {
    /// Indexes the vertices in iteration order; duplicates are an error.
    pub fn new(vertices: impl IntoIterator<Item = V>) -> crate::Result<Self> {
        let mut vertex_map = std::collections::HashMap::new();
        let mut index_list = Vec::new();
        for v in vertices {
            if vertex_map.insert(v.clone(), index_list.len()).is_some() {
                return Err(crate::GraphError::InvalidArgument(
                    "elements are not unique".into(),
                ));
            }
            index_list.push(v);
        }
        Ok(VertexToIntegerMapping {
            vertex_map,
            index_list,
        })
    }
    pub fn vertex_map(&self) -> &std::collections::HashMap<V, usize> {
        &self.vertex_map
    }
    pub fn index_list(&self) -> &[V] {
        &self.index_list
    }
}

/// Factories for fresh vertices/edges, the Rust form of JGraphT's `SupplierUtil`.
pub mod supplier {
    /// Integers `start, start + 1, ...`
    pub fn integer_supplier(start: i32) -> impl FnMut() -> i32 {
        let mut next = start;
        move || {
            next += 1;
            next - 1
        }
    }
    /// Longs `start, start + 1, ...`
    pub fn long_supplier(start: i64) -> impl FnMut() -> i64 {
        let mut next = start;
        move || {
            next += 1;
            next - 1
        }
    }
    /// Decimal strings `"start", "start + 1", ...`
    pub fn string_supplier(start: i32) -> impl FnMut() -> String {
        let mut next = integer_supplier(start);
        move || next().to_string()
    }
    /// Pseudo-random, version-4 style UUID strings (not cryptographically random).
    pub fn random_uuid_string_supplier() -> impl FnMut() -> String {
        let mut rng = super::java::JavaRandom::from_time();
        move || {
            let (a, b) = (rng.next_long() as u64, rng.next_long() as u64);
            let a = (a & 0xffff_ffff_ffff_0fff) | 0x4000;
            let b = (b & 0x3fff_ffff_ffff_ffff) | 0x8000_0000_0000_0000;
            format!(
                "{:08x}-{:04x}-{:04x}-{:04x}-{:012x}",
                a >> 32,
                (a >> 16) & 0xffff,
                a & 0xffff,
                b >> 48,
                b & 0xffff_ffff_ffff
            )
        }
    }
}

/// LSD radix sort of non-negative integers (JGraphT's `RadixSort`), stable, base 256.
pub fn radix_sort(values: &mut [u32]) {
    let mut buffer = vec![0u32; values.len()];
    for shift in (0..32).step_by(8) {
        let mut counts = [0usize; 257];
        for &v in values.iter() {
            counts[((v >> shift) & 0xff) as usize + 1] += 1;
        }
        for i in 0..256 {
            counts[i + 1] += counts[i];
        }
        for &v in values.iter() {
            let slot = &mut counts[((v >> shift) & 0xff) as usize];
            buffer[*slot] = v;
            *slot += 1;
        }
        values.copy_from_slice(&buffer);
    }
}

/// Compares doubles up to a tolerance (`ToleranceDoubleComparator`).
#[derive(Debug, Clone, Copy)]
pub struct ToleranceDoubleComparator {
    epsilon: f64,
}

impl ToleranceDoubleComparator {
    pub const DEFAULT_EPSILON: f64 = 1e-9;
    pub fn new(epsilon: f64) -> crate::Result<Self> {
        if epsilon <= 0.0 {
            return Err(crate::GraphError::InvalidArgument(
                "Tolerance must be positive".into(),
            ));
        }
        Ok(ToleranceDoubleComparator { epsilon })
    }
    /// Negative, zero or positive like `Comparator.compare`.
    pub fn compare(&self, a: f64, b: f64) -> i32 {
        if (a - b).abs() < self.epsilon {
            0
        } else {
            match a.total_cmp(&b) {
                std::cmp::Ordering::Less => -1,
                std::cmp::Ordering::Equal => 0,
                std::cmp::Ordering::Greater => 1,
            }
        }
    }
}

impl Default for ToleranceDoubleComparator {
    fn default() -> Self {
        ToleranceDoubleComparator {
            epsilon: Self::DEFAULT_EPSILON,
        }
    }
}

/// Yields the given elements once each in random order (`ElementsSequenceGenerator`): each step
/// swaps a uniformly chosen element with the last one. The random source is passed per call.
#[derive(Debug, Clone)]
pub struct ElementsSequenceGenerator<T> {
    elements: Vec<T>,
}

impl<T> ElementsSequenceGenerator<T> {
    pub fn new(elements: impl IntoIterator<Item = T>) -> Self {
        ElementsSequenceGenerator {
            elements: elements.into_iter().collect(),
        }
    }

    pub fn has_next(&self) -> bool {
        !self.elements.is_empty()
    }

    pub fn next(&mut self, rng: &mut impl java::RandomSource) -> Option<T> {
        if self.elements.is_empty() {
            return None;
        }
        let index = rng.next_int_bound(self.elements.len());
        Some(self.elements.swap_remove(index))
    }
}

/// A set usable as a hash key: equal sets hash equally whatever their iteration order.
/// (JGraphT puts `Set<V>` into maps and sets all the time; Rust's `HashSet` is not `Hash`.)
#[derive(Debug, Clone, Default)]
pub struct HashableSet<T: Eq + std::hash::Hash>(pub std::collections::HashSet<T>);

impl<T: Eq + std::hash::Hash> HashableSet<T> {
    pub fn new() -> Self {
        HashableSet(std::collections::HashSet::new())
    }
}

impl<T: Eq + std::hash::Hash> std::ops::Deref for HashableSet<T> {
    type Target = std::collections::HashSet<T>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl<T: Eq + std::hash::Hash> std::ops::DerefMut for HashableSet<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
impl<T: Eq + std::hash::Hash> PartialEq for HashableSet<T> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}
impl<T: Eq + std::hash::Hash> Eq for HashableSet<T> {}
impl<T: Eq + std::hash::Hash> std::hash::Hash for HashableSet<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        let sum = self.0.iter().fold(0u64, |acc, t| {
            let mut h = std::collections::hash_map::DefaultHasher::new();
            t.hash(&mut h);
            acc.wrapping_add(std::hash::Hasher::finish(&h))
        });
        state.write_u64(sum);
        state.write_usize(self.0.len());
    }
}
impl<T: Eq + std::hash::Hash> FromIterator<T> for HashableSet<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        HashableSet(iter.into_iter().collect())
    }
}
impl<T: Eq + std::hash::Hash, const N: usize> From<[T; N]> for HashableSet<T> {
    fn from(a: [T; N]) -> Self {
        HashableSet(a.into_iter().collect())
    }
}
