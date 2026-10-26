use super::{GraphGenerator, Structure, invalid};
use crate::Result;

fn s(vertex_count: usize, edges: Vec<(usize, usize)>) -> Result<Structure> {
    Ok(Structure {
        vertex_count,
        edges,
    })
}

/// `n` isolated vertices.
pub struct Empty(pub usize);
impl GraphGenerator for Empty {
    fn structure(&mut self, _: bool) -> Result<Structure> {
        s(self.0, vec![])
    }
}

/// Complete graph; directed targets get both orientations of every pair.
pub struct Complete(pub usize);
impl GraphGenerator for Complete {
    fn structure(&mut self, directed: bool) -> Result<Structure> {
        let n = self.0;
        let edges = (0..n)
            .flat_map(|i| (0..n).map(move |j| (i, j)))
            .filter(|&(i, j)| if directed { i != j } else { i < j })
            .collect();
        s(n, edges)
    }
}

/// Complete bipartite graph `K(n1, n2)`: vertices `0..n1` against `n1..n1+n2`.
pub struct CompleteBipartite(pub usize, pub usize);
impl GraphGenerator for CompleteBipartite {
    fn structure(&mut self, _: bool) -> Result<Structure> {
        let (a, b) = (self.0, self.1);
        s(
            a + b,
            (0..a)
                .flat_map(|i| (a..a + b).map(move |j| (i, j)))
                .collect(),
        )
    }
}

/// Cycle `0 -> 1 -> ... -> n-1 -> 0`.
pub struct Ring(pub usize);
impl GraphGenerator for Ring {
    fn structure(&mut self, _: bool) -> Result<Structure> {
        let n = self.0;
        s(
            n,
            if n < 2 {
                vec![]
            } else {
                (0..n).map(|i| (i, (i + 1) % n)).collect()
            },
        )
    }
}

/// Path `0 -> 1 -> ... -> n-1`.
pub struct Linear(pub usize);
impl GraphGenerator for Linear {
    fn structure(&mut self, _: bool) -> Result<Structure> {
        s(self.0, (1..self.0).map(|i| (i - 1, i)).collect())
    }
}

/// Star with hub `0`; edges point from the hub (directed targets).
pub struct Star(pub usize);
impl GraphGenerator for Star {
    fn structure(&mut self, _: bool) -> Result<Structure> {
        if self.0 == 0 {
            return invalid("star needs at least one vertex");
        }
        s(self.0, (1..self.0).map(|i| (0, i)).collect())
    }
}

/// Wheel: hub `0` plus a ring of `n-1` rim vertices. `inward_spokes` points spokes at the hub.
pub struct Wheel {
    pub n: usize,
    pub inward_spokes: bool,
}
impl GraphGenerator for Wheel {
    fn structure(&mut self, _: bool) -> Result<Structure> {
        let n = self.n;
        if n < 4 {
            return invalid("wheel needs at least 4 vertices");
        }
        let rim = n - 1;
        let mut edges = Vec::new();
        for i in 1..n {
            edges.push(if self.inward_spokes { (i, 0) } else { (0, i) });
            edges.push((i, 1 + i % rim));
        }
        s(n, edges)
    }
}

/// `rows x cols` grid, vertex `(r, c)` is index `r * cols + c`.
/// Directed targets get edges in both directions.
pub struct Grid {
    pub rows: usize,
    pub cols: usize,
}
impl GraphGenerator for Grid {
    fn structure(&mut self, directed: bool) -> Result<Structure> {
        let (r, c) = (self.rows, self.cols);
        let mut edges = Vec::new();
        for i in 0..r {
            for j in 0..c {
                let v = i * c + j;
                let mut link = |w: usize| {
                    edges.push((v, w));
                    if directed {
                        edges.push((w, v));
                    }
                };
                if j + 1 < c {
                    link(v + 1);
                }
                if i + 1 < r {
                    link(v + c);
                }
            }
        }
        s(r * c, edges)
    }
}

/// `dim`-dimensional hypercube on `2^dim` vertices.
pub struct Hypercube(pub u32);
impl GraphGenerator for Hypercube {
    fn structure(&mut self, directed: bool) -> Result<Structure> {
        if self.0 > 24 {
            return invalid("hypercube dimension too large");
        }
        let n = 1usize << self.0;
        let mut edges = Vec::new();
        for v in 0..n {
            for b in 0..self.0 {
                let w = v ^ (1 << b);
                if directed || v < w {
                    edges.push((v, w));
                }
            }
        }
        s(n, edges)
    }
}
