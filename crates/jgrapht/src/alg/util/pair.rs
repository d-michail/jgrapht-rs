use std::hash::{Hash, Hasher};

/// Ordered pair (`Pair`). Java's `null` elements are `Option`s.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct Pair<A, B> {
    pub first: A,
    pub second: B,
}

impl<A, B> Pair<A, B> {
    pub fn of(first: A, second: B) -> Self {
        Pair { first, second }
    }
    pub fn first(&self) -> &A {
        &self.first
    }
    pub fn second(&self) -> &B {
        &self.second
    }
}

impl<A: PartialEq, B: PartialEq> Pair<A, B> {
    pub fn has_first(&self, a: &A) -> bool {
        self.first == *a
    }
    pub fn has_second(&self, b: &B) -> bool {
        self.second == *b
    }
}

impl<T: PartialEq> Pair<T, T> {
    /// Whether `t` is one of the two elements (Java's `hasElement`).
    pub fn has_element(&self, t: &T) -> bool {
        self.first == *t || self.second == *t
    }
}

/// Ordered triple (`Triple`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct Triple<A, B, C> {
    pub first: A,
    pub second: B,
    pub third: C,
}

impl<A, B, C> Triple<A, B, C> {
    pub fn of(first: A, second: B, third: C) -> Self {
        Triple {
            first,
            second,
            third,
        }
    }
}

/// Pair that is equal to its swap and hashes independently of the order (`UnorderedPair`).
///
/// Java lets the two elements have different static types; here they share one type (use an enum
/// for heterogeneous elements).
#[derive(Debug, Clone, Copy)]
pub struct UnorderedPair<T> {
    pub first: T,
    pub second: T,
}

impl<T> UnorderedPair<T> {
    pub fn of(first: T, second: T) -> Self {
        UnorderedPair { first, second }
    }
}

impl<T: PartialEq> UnorderedPair<T> {
    pub fn has_element(&self, t: &T) -> bool {
        self.first == *t || self.second == *t
    }
}

impl<T: PartialEq> PartialEq for UnorderedPair<T> {
    fn eq(&self, other: &Self) -> bool {
        (self.first == other.first && self.second == other.second)
            || (self.first == other.second && self.second == other.first)
    }
}
impl<T: Eq> Eq for UnorderedPair<T> {}

impl<T: Hash> Hash for UnorderedPair<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let h = |t: &T| {
            let mut s = std::collections::hash_map::DefaultHasher::new();
            t.hash(&mut s);
            s.finish()
        };
        let (h1, h2) = (h(&self.first), h(&self.second));
        // same shape as JGraphT's `hash1 > hash2 ? hash1 * 31 + hash2 : hash2 * 31 + hash1`
        state.write_u64(if h1 > h2 {
            h1.wrapping_mul(31).wrapping_add(h2)
        } else {
            h2.wrapping_mul(31).wrapping_add(h1)
        });
    }
}
