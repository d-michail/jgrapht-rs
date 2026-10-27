//! Ports of `org.jgrapht.util` tests for the pieces that exist in Rust:
//! `VertexToIntegerMappingTest`, `SupplierUtilTest`, `RadixSortTest`.
//!
//! `AVLTreeTest` and `DoublyLinkedListTest` live in their own modules. Not ported (Java collection
//! classes replaced by standard Rust facilities): `PrefetchIteratorTest`,
//! `UnmodifiableUnionSetTest` (iterator chains), `StopWatch` (helper).

use jgrapht::util::supplier::*;
use jgrapht::util::{VertexToIntegerMapping, radix_sort};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::collections::HashSet;

#[test]
fn vertex_mapping_empty_set() {
    let m = VertexToIntegerMapping::<i32>::new([]).unwrap();
    assert!(m.index_list().is_empty() && m.vertex_map().is_empty());
}

#[test]
fn vertex_mapping_not_unique_elements() {
    assert!(VertexToIntegerMapping::new([1, 2, 1]).is_err());
}

#[test]
fn vertex_mapping_random_instances() {
    let mut random = StdRng::seed_from_u64((0x88) as u64);
    let mut supplier = string_supplier(random.random_range(0..100));
    for _ in 0..1024 {
        let n = 10 + random.random_range(0..1024) as usize;
        let vertices: HashSet<String> = (0..n).map(|_| supplier()).collect();
        let m = VertexToIntegerMapping::new(vertices).unwrap();
        assert_eq!((m.vertex_map().len(), m.index_list().len()), (n, n));
        for (i, v) in m.index_list().iter().enumerate() {
            assert_eq!(m.vertex_map()[v], i);
        }
        for (v, &i) in m.vertex_map() {
            assert_eq!(&m.index_list()[i], v);
        }
    }
}

#[test]
fn supplier_util() {
    let mut s = integer_supplier(0);
    assert_eq!([s(), s(), s(), s(), s()], [0, 1, 2, 3, 4]);
    let mut s = integer_supplier(4);
    assert_eq!([s(), s(), s(), s()], [4, 5, 6, 7]);
    let mut s = long_supplier(0);
    assert_eq!([s(), s(), s()], [0, 1, 2]);
    let mut s = long_supplier(44);
    assert_eq!([s(), s(), s(), s()], [44, 45, 46, 47]);
    let mut s = string_supplier(0);
    assert_eq!([s(), s(), s()], ["0", "1", "2"]);
    let mut s = string_supplier(44);
    assert_eq!([s(), s(), s(), s()], ["44", "45", "46", "47"]);
    let mut s = random_uuid_string_supplier();
    let values: HashSet<String> = (0..5).map(|_| s()).collect();
    assert_eq!(values.len(), 5);
    assert!(
        values
            .iter()
            .all(|u| u.len() == 36 && u.matches('-').count() == 4)
    );
}

fn is_sorted(v: &[u32]) -> bool {
    v.windows(2).all(|w| w[0] <= w[1])
}

#[test]
fn radix_sort_empty_and_small() {
    let mut empty: Vec<u32> = vec![];
    radix_sort(&mut empty);
    assert!(empty.is_empty());
    let mut small = vec![3, 1, 10, 2, 5, 3];
    radix_sort(&mut small);
    assert!(is_sorted(&small));
}

#[test]
fn radix_sort_random_huge_array() {
    let mut random = StdRng::seed_from_u64((0x881) as u64);
    let mut list: Vec<u32> = (0..1_000_000)
        .map(|_| random.random_range(0..i32::MAX) as u32)
        .collect();
    radix_sort(&mut list);
    assert!(is_sorted(&list));
}

#[test]
fn radix_sort_random_arrays() {
    let mut random = StdRng::seed_from_u64((0x88) as u64);
    for _ in 0..50_000 {
        let n = 1 + random.random_range(0..100) as usize;
        let mut list: Vec<u32> = (0..n)
            .map(|_| random.random_range(0..i32::MAX) as u32)
            .collect();
        let mut expected = list.clone();
        expected.sort_unstable();
        radix_sort(&mut list);
        assert_eq!(list, expected);
    }
}
