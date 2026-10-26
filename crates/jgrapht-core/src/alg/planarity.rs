//! Planarity testing with the left–right (LR) algorithm of de Fraysseix and Rosenstiehl,
//! in the formulation of Brandes ("The Left-Right Planarity Test"). Linear time.
//!
//! Direction, loops and parallel edges are ignored. The DFS is recursive, so extremely deep
//! graphs (hundreds of thousands of vertices on a single path) may need a bigger stack.

use super::util::Indexed;
use crate::graph::Graph;
use std::collections::HashSet;

const NONE: usize = usize::MAX;

#[derive(Clone, Copy)]
struct Interval {
    low: usize,
    high: usize,
}
impl Interval {
    const EMPTY: Interval = Interval {
        low: NONE,
        high: NONE,
    };
    fn is_empty(&self) -> bool {
        self.low == NONE && self.high == NONE
    }
}

#[derive(Clone, Copy)]
struct Pair {
    left: Interval,
    right: Interval,
}
impl Pair {
    fn swap(&mut self) {
        std::mem::swap(&mut self.left, &mut self.right);
    }
}

struct Lr {
    adj: Vec<Vec<usize>>,
    height: Vec<usize>,
    /// oriented edges
    ends: Vec<(usize, usize)>,
    parent_edge: Vec<usize>,
    lowpt: Vec<usize>,
    lowpt2: Vec<usize>,
    nesting: Vec<usize>,
    refs: Vec<usize>,
    side: Vec<i8>,
    lowpt_edge: Vec<usize>,
    stack_bottom: Vec<usize>,
    ordered: Vec<Vec<usize>>,
    stack: Vec<Pair>,
    done: HashSet<(usize, usize)>,
}

impl Lr {
    fn orient(&mut self, v: usize) {
        let e = self.parent_edge[v];
        for i in 0..self.adj[v].len() {
            let w = self.adj[v][i];
            if self.done.contains(&(v, w)) || self.done.contains(&(w, v)) {
                continue;
            }
            self.done.insert((v, w));
            let vw = self.ends.len();
            self.ends.push((v, w));
            self.lowpt.push(self.height[v]);
            self.lowpt2.push(self.height[v]);
            self.nesting.push(0);
            self.refs.push(NONE);
            self.side.push(1);
            self.lowpt_edge.push(NONE);
            self.stack_bottom.push(0);
            self.parent_edge
                .resize(self.parent_edge.len().max(self.height.len()), NONE);
            self.ordered[v].push(vw);
            if self.height[w] == NONE {
                self.parent_edge[w] = vw;
                self.height[w] = self.height[v] + 1;
                self.orient(w);
            } else {
                self.lowpt[vw] = self.height[w];
            }
            self.nesting[vw] = 2 * self.lowpt[vw];
            if self.lowpt2[vw] < self.height[v] {
                self.nesting[vw] += 1;
            }
            if e != NONE {
                if self.lowpt[vw] < self.lowpt[e] {
                    self.lowpt2[e] = self.lowpt[e].min(self.lowpt2[vw]);
                    self.lowpt[e] = self.lowpt[vw];
                } else if self.lowpt[vw] > self.lowpt[e] {
                    self.lowpt2[e] = self.lowpt2[e].min(self.lowpt[vw]);
                } else {
                    self.lowpt2[e] = self.lowpt2[e].min(self.lowpt2[vw]);
                }
            }
        }
    }

    fn conflicting(&self, i: Interval, b: usize) -> bool {
        !i.is_empty() && self.lowpt[i.high] > self.lowpt[b]
    }

    fn lowest(&self, p: &Pair) -> usize {
        if p.left.is_empty() {
            self.lowpt[p.right.low]
        } else if p.right.is_empty() {
            self.lowpt[p.left.low]
        } else {
            self.lowpt[p.left.low].min(self.lowpt[p.right.low])
        }
    }

    fn test(&mut self, v: usize) -> bool {
        let e = self.parent_edge[v];
        let edges = self.ordered[v].clone();
        for (idx, &ei) in edges.iter().enumerate() {
            self.stack_bottom[ei] = self.stack.len();
            let w = self.ends[ei].1;
            if ei == self.parent_edge[w] {
                if !self.test(w) {
                    return false;
                }
            } else {
                self.lowpt_edge[ei] = ei;
                self.stack.push(Pair {
                    left: Interval::EMPTY,
                    right: Interval { low: ei, high: ei },
                });
            }
            if self.lowpt[ei] < self.height[v] {
                if idx == 0 {
                    self.lowpt_edge[e] = self.lowpt_edge[ei];
                } else if !self.add_constraints(ei, e) {
                    return false;
                }
            }
        }
        if e != NONE {
            let u = self.ends[e].0;
            self.trim_back_edges(u);
            if self.lowpt[e] < self.height[u] {
                let top = *self.stack.last().expect("pair exists for a returning edge");
                let (hl, hr) = (top.left.high, top.right.high);
                self.refs[e] = if hl != NONE && (hr == NONE || self.lowpt[hl] > self.lowpt[hr]) {
                    hl
                } else {
                    hr
                };
            }
        }
        true
    }

    fn add_constraints(&mut self, ei: usize, e: usize) -> bool {
        let mut p = Pair {
            left: Interval::EMPTY,
            right: Interval::EMPTY,
        };
        loop {
            let mut q = self.stack.pop().expect("non-empty");
            if !q.left.is_empty() {
                q.swap();
            }
            if !q.left.is_empty() {
                return false;
            }
            if self.lowpt[q.right.low] > self.lowpt[e] {
                if p.right.is_empty() {
                    p.right = q.right;
                } else {
                    self.refs[p.right.low] = q.right.high;
                }
                p.right.low = q.right.low;
            } else {
                self.refs[q.right.low] = self.lowpt_edge[e];
            }
            if self.stack.len() == self.stack_bottom[ei] {
                break;
            }
        }
        while let Some(&top) = self.stack.last() {
            if !(self.conflicting(top.left, ei) || self.conflicting(top.right, ei)) {
                break;
            }
            let mut q = self.stack.pop().unwrap();
            if self.conflicting(q.right, ei) {
                q.swap();
            }
            if self.conflicting(q.right, ei) {
                return false;
            }
            if p.right.low != NONE {
                self.refs[p.right.low] = q.right.high;
            }
            if q.right.low != NONE {
                p.right.low = q.right.low;
            }
            if p.left.is_empty() {
                p.left = q.left;
            } else {
                self.refs[p.left.low] = q.left.high;
            }
            p.left.low = q.left.low;
        }
        if !(p.left.is_empty() && p.right.is_empty()) {
            self.stack.push(p);
        }
        true
    }

    fn trim_back_edges(&mut self, u: usize) {
        while let Some(&top) = self.stack.last() {
            if self.lowest(&top) != self.height[u] {
                break;
            }
            let p = self.stack.pop().unwrap();
            if p.left.low != NONE {
                self.side[p.left.low] = -1;
            }
        }
        if let Some(mut p) = self.stack.pop() {
            while p.left.high != NONE && self.ends[p.left.high].1 == u {
                p.left.high = self.refs[p.left.high];
            }
            if p.left.high == NONE && p.left.low != NONE {
                self.refs[p.left.low] = p.right.low;
                self.side[p.left.low] = -1;
                p.left.low = NONE;
            }
            while p.right.high != NONE && self.ends[p.right.high].1 == u {
                p.right.high = self.refs[p.right.high];
            }
            if p.right.high == NONE && p.right.low != NONE {
                self.refs[p.right.low] = p.left.low;
                self.side[p.right.low] = -1;
                p.right.low = NONE;
            }
            self.stack.push(p);
        }
    }
}

/// Whether the graph (as a simple undirected graph) can be drawn in the plane without crossings.
pub fn is_planar<G: Graph>(g: &G) -> bool {
    let ix = Indexed::new(g);
    let n = ix.n();
    let mut seen = HashSet::new();
    let mut adj: Vec<Vec<usize>> = vec![Vec::new(); n];
    for &(a, b) in &ix.ends {
        if a != b && seen.insert((a.min(b), a.max(b))) {
            adj[a].push(b);
            adj[b].push(a);
        }
    }
    let m = seen.len();
    if n >= 3 && m > 3 * n - 6 {
        return false;
    }
    let mut lr = Lr {
        adj,
        height: vec![NONE; n],
        ends: Vec::new(),
        parent_edge: vec![NONE; n],
        lowpt: Vec::new(),
        lowpt2: Vec::new(),
        nesting: Vec::new(),
        refs: Vec::new(),
        side: Vec::new(),
        lowpt_edge: Vec::new(),
        stack_bottom: Vec::new(),
        ordered: vec![Vec::new(); n],
        stack: Vec::new(),
        done: HashSet::new(),
    };
    let mut roots = Vec::new();
    for v in 0..n {
        if lr.height[v] == NONE {
            lr.height[v] = 0;
            roots.push(v);
            lr.orient(v);
        }
    }
    for v in 0..n {
        let nesting = &lr.nesting;
        lr.ordered[v].sort_by_key(|&e| nesting[e]);
    }
    roots.into_iter().all(|r| lr.test(r))
}
