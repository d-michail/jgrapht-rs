//! Java-compatible building blocks, so that seeded tests reproduce JGraphT's exact outputs.
//!
//! Kept only for the places that must match numbers produced by the JVM: the seeded generator
//! tests and the NETGEN golden file, the Fruchterman–Reingold and random layouts, label
//! propagation, the alias sampler and the random-greedy colouring tests. Everything else seeds
//! `rand`'s `StdRng`; the algorithms accept any `rand::RngCore` through [`RandomSource`].

/// `java.util.Random`: the same 48-bit linear congruential generator, so equal seeds give equal
/// streams.
#[derive(Debug, Clone)]
pub struct JavaRandom {
    seed: u64,
}

const MULTIPLIER: u64 = 0x5DEECE66D;
const MASK: u64 = (1 << 48) - 1;

impl JavaRandom {
    pub fn new(seed: i64) -> Self {
        JavaRandom {
            seed: (seed as u64 ^ MULTIPLIER) & MASK,
        }
    }
    /// `new Random()`-like seeding from the system clock.
    pub fn from_time() -> Self {
        let t = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos() as i64);
        Self::new(t)
    }
    pub fn set_seed(&mut self, seed: i64) {
        self.seed = (seed as u64 ^ MULTIPLIER) & MASK;
    }
    fn next(&mut self, bits: u32) -> i32 {
        self.seed = (self.seed.wrapping_mul(MULTIPLIER).wrapping_add(0xB)) & MASK;
        (self.seed >> (48 - bits)) as i64 as i32
    }
    pub fn next_int(&mut self) -> i32 {
        self.next(32)
    }
    /// `nextInt(bound)`: uniform in `[0, bound)`.
    pub fn next_int_bound(&mut self, bound: i32) -> i32 {
        assert!(bound > 0, "bound must be positive");
        let mut r = self.next(31);
        let m = bound - 1;
        if bound & m == 0 {
            ((bound as i64 * r as i64) >> 31) as i32
        } else {
            let mut u = r;
            loop {
                r = u % bound;
                if u.wrapping_sub(r).wrapping_add(m) >= 0 {
                    break;
                }
                u = self.next(31);
            }
            r
        }
    }
    pub fn next_long(&mut self) -> i64 {
        ((self.next(32) as i64) << 32).wrapping_add(self.next(32) as i64)
    }
    pub fn next_boolean(&mut self) -> bool {
        self.next(1) != 0
    }
    pub fn next_double(&mut self) -> f64 {
        let hi = (self.next(26) as i64) << 27;
        (hi + self.next(27) as i64) as f64 * (1.0 / (1u64 << 53) as f64)
    }
    pub fn next_float(&mut self) -> f32 {
        self.next(24) as f32 / (1 << 24) as f32
    }
    /// `nextGaussian()` (polar method, without the cached second value across calls being shared).
    pub fn next_gaussian(&mut self) -> f64 {
        loop {
            let v1 = 2.0 * self.next_double() - 1.0;
            let v2 = 2.0 * self.next_double() - 1.0;
            let s = v1 * v1 + v2 * v2;
            if s < 1.0 && s != 0.0 {
                let multiplier = (-2.0 * s.ln() / s).sqrt();
                return v1 * multiplier;
            }
        }
    }
}

/// Source of randomness for algorithms that need to match JGraphT draw for draw.
pub trait RandomSource {
    /// Uniform in `[0, bound)`.
    fn next_int_bound(&mut self, bound: usize) -> usize;
    /// Uniform in `[0, 1)`.
    fn next_double(&mut self) -> f64;
    fn next_boolean(&mut self) -> bool;
    /// Uniform in `[0, 1)` with float precision.
    fn next_float(&mut self) -> f32;
}

impl RandomSource for JavaRandom {
    fn next_int_bound(&mut self, bound: usize) -> usize {
        JavaRandom::next_int_bound(self, bound as i32) as usize
    }
    fn next_double(&mut self) -> f64 {
        JavaRandom::next_double(self)
    }
    fn next_boolean(&mut self) -> bool {
        JavaRandom::next_boolean(self)
    }
    fn next_float(&mut self) -> f32 {
        JavaRandom::next_float(self)
    }
}

impl<R: rand::RngCore> RandomSource for R {
    fn next_int_bound(&mut self, bound: usize) -> usize {
        rand::Rng::random_range(self, 0..bound)
    }
    fn next_double(&mut self) -> f64 {
        rand::Rng::random::<f64>(self)
    }
    fn next_boolean(&mut self) -> bool {
        rand::Rng::random_bool(self, 0.5)
    }
    fn next_float(&mut self) -> f32 {
        rand::Rng::random::<f32>(self)
    }
}

/// `Collections.shuffle(list, rng)`: Fisher–Yates from the back, `swap(i - 1, nextInt(i))`.
pub fn java_shuffle<T>(list: &mut [T], rng: &mut impl RandomSource) {
    for i in (2..=list.len()).rev() {
        let j = rng.next_int_bound(i);
        list.swap(i - 1, j);
    }
}

/// `java.util.PriorityQueue` with a comparator: same sift-up/sift-down, so ties and even
/// inconsistent comparators behave exactly as in Java.
pub struct JavaPriorityQueue<T, C: Fn(&T, &T) -> i32> {
    queue: Vec<T>,
    cmp: C,
}

impl<T, C: Fn(&T, &T) -> i32> JavaPriorityQueue<T, C> {
    pub fn new(cmp: C) -> Self {
        JavaPriorityQueue {
            queue: Vec::new(),
            cmp,
        }
    }
    pub fn len(&self) -> usize {
        self.queue.len()
    }
    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }
    pub fn peek(&self) -> Option<&T> {
        self.queue.first()
    }
    pub fn offer(&mut self, x: T) {
        let mut k = self.queue.len();
        self.queue.push(x);
        // sift up from position k without moving the element until its final slot is known
        while k > 0 {
            let parent = (k - 1) >> 1;
            if (self.cmp)(&self.queue[k], &self.queue[parent]) >= 0 {
                break;
            }
            self.queue.swap(k, parent);
            k = parent;
        }
    }
    pub fn poll(&mut self) -> Option<T> {
        let n = self.queue.len();
        if n == 0 {
            return None;
        }
        self.queue.swap(0, n - 1);
        let result = self.queue.pop();
        let n = n - 1;
        if n > 1 {
            // sift the element now at index 0 down, as Java's siftDown(0, x)
            let mut k = 0;
            let half = n >> 1;
            while k < half {
                let mut child = 2 * k + 1;
                let right = child + 1;
                if right < n && (self.cmp)(&self.queue[child], &self.queue[right]) > 0 {
                    child = right;
                }
                if (self.cmp)(&self.queue[k], &self.queue[child]) <= 0 {
                    break;
                }
                self.queue.swap(k, child);
                k = child;
            }
        }
        result
    }
}

/// `String.hashCode()` of a Java string, for UTF-16 code units.
pub fn java_string_hash(s: &str) -> i32 {
    s.encode_utf16()
        .fold(0i32, |h, c| h.wrapping_mul(31).wrapping_add(c as i32))
}

/// The iteration order of a `java.util.HashMap` / `HashSet` that received `items` (distinct, in
/// this insertion order) with default capacity: buckets by `(h ^ h >>> 16) & (capacity - 1)`,
/// insertion order within a bucket. Tree bins (very long collision chains) are not modelled.
pub fn java_hash_order<T>(items: Vec<T>, hash: impl Fn(&T) -> i32) -> Vec<T> {
    let mut capacity = 16usize;
    while items.len() * 4 > capacity * 3 {
        capacity *= 2;
    }
    let mut keyed: Vec<(usize, T)> = items
        .into_iter()
        .map(|t| {
            let h = hash(&t) as u32;
            (((h ^ (h >> 16)) as usize) & (capacity - 1), t)
        })
        .collect();
    keyed.sort_by_key(|(bucket, _)| *bucket);
    keyed.into_iter().map(|(_, t)| t).collect()
}

/// Iteration order of a `java.util.HashSet` built as `new HashSet<>(first)` followed by adding
/// `rest` (the pattern `new HashSet<>(a); addAll(b)`), for elements with the given `hashCode`.
pub fn java_hash_set_order<T>(first: Vec<T>, rest: Vec<T>, hash: impl Fn(&T) -> i32) -> Vec<T> {
    // new HashSet<>(Collection) sizes its table for max(n / 0.75 + 1, 16) elements
    let wanted = ((first.len() as f32 / 0.75) as usize + 1).max(16);
    let mut capacity = wanted.next_power_of_two();
    let mut size = first.len();
    for _ in 0..rest.len() {
        size += 1;
        if size > capacity * 3 / 4 {
            capacity *= 2;
        }
    }
    let mut keyed: Vec<(usize, T)> = first
        .into_iter()
        .chain(rest)
        .map(|t| {
            let h = hash(&t) as u32;
            (((h ^ (h >> 16)) as usize) & (capacity - 1), t)
        })
        .collect();
    keyed.sort_by_key(|(bucket, _)| *bucket);
    keyed.into_iter().map(|(_, t)| t).collect()
}
