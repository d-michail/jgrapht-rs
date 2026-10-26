//! Induced cycles ("holes") of length five or more: weak chordality and Berge's perfection test.
//! The search enumerates induced paths and is exponential in the worst case.

use super::util::Indexed;
use crate::graph::Graph;

fn adjacency<G: Graph>(ix: &Indexed<'_, G>) -> Vec<Vec<bool>> {
    let n = ix.n();
    let mut a = vec![vec![false; n]; n];
    for &(x, y) in &ix.ends {
        if x != y {
            a[x][y] = true;
            a[y][x] = true;
        }
    }
    a
}

fn complement(a: &[Vec<bool>]) -> Vec<Vec<bool>> {
    let n = a.len();
    (0..n)
        .map(|i| (0..n).map(|j| i != j && !a[i][j]).collect())
        .collect()
}

/// An induced cycle of length >= 5 (odd only if `odd_only`), as a vertex-index list.
fn find_hole(a: &[Vec<bool>], odd_only: bool) -> Option<Vec<usize>> {
    let n = a.len();
    fn rec(a: &[Vec<bool>], path: &mut Vec<usize>, start: usize, odd_only: bool) -> bool {
        let last = *path.last().unwrap();
        for next in start + 1..a.len() {
            if path.contains(&next) || !a[last][next] {
                continue;
            }
            // `next` must not touch any path vertex except `last` (and the first vertex when closing)
            let touches: Vec<usize> = path.iter().copied().filter(|&p| a[next][p]).collect();
            if touches == [last] {
                path.push(next);
                if rec(a, path, start, odd_only) {
                    return true;
                }
                path.pop();
            } else if touches.len() == 2
                && touches.contains(&last)
                && touches.contains(&path[0])
                && path.len() >= 4
            {
                // closes an induced cycle of length path.len() + 1
                let len = path.len() + 1;
                if !odd_only || len % 2 == 1 {
                    path.push(next);
                    return true;
                }
            }
        }
        false
    }
    for s in 0..n {
        let mut path = vec![s];
        // the second vertex fixes orientation: it must be larger than the closing vertex; skip that
        // refinement, duplicates are harmless for existence queries
        for v in s + 1..n {
            if a[s][v] {
                path.truncate(1);
                path.push(v);
                if rec(a, &mut path, s, odd_only) {
                    return Some(path);
                }
            }
        }
    }
    None
}

/// An induced cycle of length >= 5 in `g` or its complement, if any.
pub fn find_weak_chordality_certificate<G: Graph>(g: &G) -> Option<Vec<G::Vertex>> {
    let ix = Indexed::new(g);
    let a = adjacency(&ix);
    find_hole(&a, false)
        .or_else(|| find_hole(&complement(&a), false))
        .map(|c| ix.vertices_of(&c))
}

pub fn is_weakly_chordal<G: Graph>(g: &G) -> bool {
    find_weak_chordality_certificate(g).is_none()
}

/// An odd hole or odd antihole, if any (a certificate that the graph is not perfect).
pub fn find_odd_hole_or_antihole<G: Graph>(g: &G) -> Option<Vec<G::Vertex>> {
    let ix = Indexed::new(g);
    let a = adjacency(&ix);
    find_hole(&a, true)
        .or_else(|| find_hole(&complement(&a), true))
        .map(|c| ix.vertices_of(&c))
}

/// Berge graph / perfect graph test (strong perfect graph theorem).
pub fn is_perfect<G: Graph>(g: &G) -> bool {
    find_odd_hole_or_antihole(g).is_none()
}
