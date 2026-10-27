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

//! Private integer set with the iteration order of JDK 21's `HashSet<Integer>`.
//!
//! The weak-chordality algorithm removes `iterator().next()` from its label buckets, so the
//! backing-table order is observable there. This implementation keeps JDK 21's power-of-two table,
//! `h ^ (h >>> 16)` spreading, insertion order within a bucket, resize splitting, and table-order
//! iteration while using Rust arena indices instead of object links.
//!
//! JDK hash maps treeify a collision chain at table sizes of at least 64. Tree bins are deliberately
//! not represented: the dense integer indices used by the inspector and its tests do not reach that
//! case, and treeification therefore does not change their iteration order. Operations have the
//! same expected `O(1)` cost as `HashSet`; iteration scans the table just as JDK iteration does.

const DEFAULT_CAPACITY: usize = 16;
const MAXIMUM_CAPACITY: usize = 1 << 30;
const TREEIFY_THRESHOLD: usize = 8;
const MIN_TREEIFY_CAPACITY: usize = 64;

#[derive(Debug, Clone, Copy)]
struct Entry {
    key: i32,
    hash: u32,
    next: Option<usize>,
}

#[derive(Debug, Default)]
pub(crate) struct JavaIntHashSet {
    table: Vec<Option<usize>>,
    entries: Vec<Entry>,
    free: Vec<usize>,
    len: usize,
    threshold: usize,
}

impl JavaIntHashSet {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Equivalent to JDK 21's `HashSet.newHashSet(expected_size)`.
    pub(crate) fn with_expected_size(expected_size: usize) -> Self {
        let requested = expected_size
            .saturating_mul(4)
            .div_ceil(3)
            .min(MAXIMUM_CAPACITY);
        JavaIntHashSet {
            threshold: table_size_for(requested),
            ..JavaIntHashSet::default()
        }
    }

    #[allow(dead_code)] // part of the set primitive; the inspector currently only needs is_empty
    pub(crate) fn len(&self) -> usize {
        self.len
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.len == 0
    }

    #[allow(dead_code)] // required set operation, exercised directly by the golden unit tests
    pub(crate) fn contains(&self, key: i32) -> bool {
        if self.table.is_empty() {
            return false;
        }
        let hash = spread(key);
        let mut current = self.table[bucket(hash, self.table.len())];
        while let Some(entry) = current {
            if self.entries[entry].hash == hash && self.entries[entry].key == key {
                return true;
            }
            current = self.entries[entry].next;
        }
        false
    }

    pub(crate) fn insert(&mut self, key: i32) -> bool {
        if self.table.is_empty() {
            self.resize();
        }

        let hash = spread(key);
        let bin = bucket(hash, self.table.len());
        let mut tail = None;
        let mut current = self.table[bin];
        let mut chain_len = 0;
        while let Some(entry) = current {
            if self.entries[entry].hash == hash && self.entries[entry].key == key {
                return false;
            }
            tail = Some(entry);
            current = self.entries[entry].next;
            chain_len += 1;
        }

        let entry = self.allocate(Entry {
            key,
            hash,
            next: None,
        });
        if let Some(tail) = tail {
            self.entries[tail].next = Some(entry);
        } else {
            self.table[bin] = Some(entry);
        }

        // HashMap.treeifyBin resizes short tables before it considers a tree bin.
        if chain_len >= TREEIFY_THRESHOLD && self.table.len() < MIN_TREEIFY_CAPACITY {
            self.resize();
        }
        self.len += 1;
        if self.len > self.threshold {
            self.resize();
        }
        true
    }

    pub(crate) fn remove(&mut self, key: i32) -> bool {
        if self.table.is_empty() {
            return false;
        }
        let hash = spread(key);
        let bin = bucket(hash, self.table.len());
        let mut previous: Option<usize> = None;
        let mut current = self.table[bin];
        while let Some(entry) = current {
            let next = self.entries[entry].next;
            if self.entries[entry].hash == hash && self.entries[entry].key == key {
                if let Some(previous) = previous {
                    self.entries[previous].next = next;
                } else {
                    self.table[bin] = next;
                }
                self.entries[entry].next = None;
                self.free.push(entry);
                self.len -= 1;
                return true;
            }
            previous = current;
            current = next;
        }
        false
    }

    /// The value returned by `set.iterator().next()` in JDK 21.
    pub(crate) fn first(&self) -> Option<i32> {
        self.iter().next()
    }

    pub(crate) fn iter(&self) -> Iter<'_> {
        Iter {
            set: self,
            bucket: 0,
            next: None,
        }
    }

    /// Matches `HashMap.clear`: retains the allocated table and its resize threshold.
    pub(crate) fn clear(&mut self) {
        for bin in &mut self.table {
            let mut current = bin.take();
            while let Some(entry) = current {
                current = self.entries[entry].next;
                self.entries[entry].next = None;
                self.free.push(entry);
            }
        }
        self.len = 0;
    }

    fn allocate(&mut self, value: Entry) -> usize {
        if let Some(entry) = self.free.pop() {
            self.entries[entry] = value;
            entry
        } else {
            let entry = self.entries.len();
            self.entries.push(value);
            entry
        }
    }

    fn resize(&mut self) {
        let old_capacity = self.table.len();
        let old_threshold = self.threshold;
        let (new_capacity, new_threshold) = if old_capacity == 0 {
            if old_threshold > 0 {
                let capacity = old_threshold;
                (capacity, threshold_for(capacity))
            } else {
                (DEFAULT_CAPACITY, DEFAULT_CAPACITY * 3 / 4)
            }
        } else if old_capacity >= MAXIMUM_CAPACITY {
            self.threshold = usize::MAX;
            return;
        } else {
            let capacity = (old_capacity * 2).min(MAXIMUM_CAPACITY);
            let threshold = if old_capacity >= DEFAULT_CAPACITY {
                old_threshold.saturating_mul(2)
            } else {
                threshold_for(capacity)
            };
            (capacity, threshold)
        };

        let old_table = std::mem::replace(&mut self.table, vec![None; new_capacity]);
        self.threshold = new_threshold;
        if old_capacity == 0 {
            return;
        }

        // JDK resize preserves the relative order of both halves of every old bucket.
        for (old_bin, head) in old_table.into_iter().enumerate() {
            let mut low_head: Option<usize> = None;
            let mut low_tail: Option<usize> = None;
            let mut high_head: Option<usize> = None;
            let mut high_tail: Option<usize> = None;
            let mut current = head;
            while let Some(entry) = current {
                current = self.entries[entry].next;
                self.entries[entry].next = None;
                let (head, tail) = if self.entries[entry].hash as usize & old_capacity == 0 {
                    (&mut low_head, &mut low_tail)
                } else {
                    (&mut high_head, &mut high_tail)
                };
                if let Some(previous_tail) = *tail {
                    self.entries[previous_tail].next = Some(entry);
                } else {
                    *head = Some(entry);
                }
                *tail = Some(entry);
            }
            self.table[old_bin] = low_head;
            self.table[old_bin + old_capacity] = high_head;
        }
    }
}

pub(crate) struct Iter<'a> {
    set: &'a JavaIntHashSet,
    bucket: usize,
    next: Option<usize>,
}

impl Iterator for Iter<'_> {
    type Item = i32;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(entry) = self.next {
            self.next = self.set.entries[entry].next;
            return Some(self.set.entries[entry].key);
        }
        while self.bucket < self.set.table.len() {
            let entry = self.set.table[self.bucket];
            self.bucket += 1;
            if let Some(entry) = entry {
                self.next = self.set.entries[entry].next;
                return Some(self.set.entries[entry].key);
            }
        }
        None
    }
}

fn spread(key: i32) -> u32 {
    let hash = key as u32;
    hash ^ (hash >> 16)
}

fn bucket(hash: u32, capacity: usize) -> usize {
    hash as usize & (capacity - 1)
}

fn threshold_for(capacity: usize) -> usize {
    if capacity < MAXIMUM_CAPACITY {
        capacity * 3 / 4
    } else {
        usize::MAX
    }
}

fn table_size_for(capacity: usize) -> usize {
    capacity.next_power_of_two().clamp(1, MAXIMUM_CAPACITY)
}

#[cfg(test)]
mod tests {
    use super::JavaIntHashSet;

    fn set(expected_size: Option<usize>, values: &[i32]) -> JavaIntHashSet {
        let mut set =
            expected_size.map_or_else(JavaIntHashSet::new, JavaIntHashSet::with_expected_size);
        for &value in values {
            set.insert(value);
        }
        set
    }

    fn values(set: &JavaIntHashSet) -> Vec<i32> {
        set.iter().collect()
    }

    // Golden orders generated with OpenJDK 21.0.12.1 by /tmp/JavaHashSetGolden.java.
    #[test]
    fn jdk21_default_order_and_hash_spreading() {
        let actual = set(None, &[10, 3, 20, 1, 17, 33, -1, 65_536, 2, 18]);
        assert_eq!(values(&actual), [-1, 1, 17, 33, 65_536, 2, 18, 3, 20, 10]);

        let actual = set(
            None,
            &[-65_537, -1, 0, 65_536, 1, 65_537, i32::MIN, i32::MAX],
        );
        assert_eq!(
            values(&actual),
            [-1, 0, 65_537, i32::MIN, i32::MAX, -65_537, 65_536, 1]
        );
    }

    #[test]
    fn jdk21_resize_and_presized_order() {
        let input = [
            31, 15, 47, 0, 32, 16, 48, 1, 33, 17, 49, 2, 34, 18, 50, 3, 35, 19, 51, 4,
        ];
        let expected = [
            0, 32, 1, 33, 2, 34, 3, 35, 4, 15, 47, 16, 48, 17, 49, 18, 50, 19, 51, 31,
        ];
        assert_eq!(values(&set(None, &input)), expected);
        assert_eq!(values(&set(Some(20), &input)), expected);

        // Nine entries in one bin make HashMap.treeifyBin resize a short table. At capacity 64,
        // key 32 splits away from the remaining keys even though it was inserted first.
        let collision_resize = set(None, &[32, 0, 64, 128, 192, 256, 320, 384, 448, 512]);
        assert_eq!(
            values(&collision_resize),
            [0, 64, 128, 192, 256, 320, 384, 448, 512, 32]
        );
    }

    #[test]
    fn jdk21_remove_and_reinsert_order() {
        let input = [
            31, 15, 47, 0, 32, 16, 48, 1, 33, 17, 49, 2, 34, 18, 50, 3, 35, 19, 51, 4,
        ];
        let mut set = set(None, &input);
        assert!(set.remove(32));
        assert!(set.remove(17));
        assert!(set.insert(32));
        assert!(set.insert(17));
        assert_eq!(
            values(&set),
            [
                0, 32, 1, 33, 2, 34, 3, 35, 4, 15, 47, 16, 48, 49, 17, 18, 50, 19, 51, 31
            ]
        );
    }

    #[test]
    fn set_operations_and_clear() {
        let mut set = JavaIntHashSet::new();
        assert!(set.is_empty());
        assert!(set.insert(7));
        assert!(!set.insert(7));
        assert!(set.contains(7));
        assert_eq!(set.len(), 1);
        assert_eq!(set.first(), Some(7));
        assert!(!set.remove(8));
        set.clear();
        assert!(set.is_empty());
        assert!(!set.contains(7));
    }
}
