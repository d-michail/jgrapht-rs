use super::{GraphGenerator, Structure, invalid};
use crate::Result;
use rand::Rng;
use rand::seq::index;
use std::collections::HashSet;

fn pairs(n: usize, directed: bool, loops: bool) -> Vec<(usize, usize)> {
    (0..n)
        .flat_map(|i| (0..n).map(move |j| (i, j)))
        .filter(|&(i, j)| if i == j { loops } else { directed || i < j })
        .collect()
}

/// Erdős–Rényi `G(n, p)`: each possible edge appears independently with probability `p`.
pub struct Gnp<R> {
    pub n: usize,
    pub p: f64,
    pub loops: bool,
    pub rng: R,
}
impl<R: Rng> GraphGenerator for Gnp<R> {
    fn structure(&mut self, directed: bool) -> Result<Structure> {
        if !(0.0..=1.0).contains(&self.p) {
            return invalid("probability must be in [0, 1]");
        }
        let edges = pairs(self.n, directed, self.loops)
            .into_iter()
            .filter(|_| self.rng.random_bool(self.p))
            .collect();
        Ok(Structure {
            vertex_count: self.n,
            edges,
            ..Default::default()
        })
    }
}

/// Erdős–Rényi `G(n, m)`: exactly `m` distinct edges chosen uniformly.
pub struct Gnm<R> {
    pub n: usize,
    pub m: usize,
    pub loops: bool,
    pub rng: R,
}
impl<R: Rng> GraphGenerator for Gnm<R> {
    fn structure(&mut self, directed: bool) -> Result<Structure> {
        let all = pairs(self.n, directed, self.loops);
        if self.m > all.len() {
            return invalid("too many edges requested");
        }
        let edges = index::sample(&mut self.rng, all.len(), self.m)
            .iter()
            .map(|i| all[i])
            .collect();
        Ok(Structure {
            vertex_count: self.n,
            edges,
            ..Default::default()
        })
    }
}

/// Barabási–Albert preferential attachment: start from a complete graph on `m0` vertices, then
/// add vertices until `n`, each attaching to `m` distinct existing vertices chosen with
/// probability proportional to degree.
pub struct BarabasiAlbert<R> {
    pub m0: usize,
    pub m: usize,
    pub n: usize,
    pub rng: R,
}
impl<R: Rng> GraphGenerator for BarabasiAlbert<R> {
    fn structure(&mut self, _: bool) -> Result<Structure> {
        let (m0, m, n) = (self.m0, self.m, self.n);
        if m0 < 1 || m < 1 || m > m0 || n < m0 {
            return invalid("require 1 <= m <= m0 <= n");
        }
        let mut edges = Vec::new();
        let mut urn: Vec<usize> = Vec::new(); // vertex repeated once per incident edge end
        for i in 0..m0 {
            for j in i + 1..m0 {
                edges.push((i, j));
                urn.extend([i, j]);
            }
        }
        for v in m0..n {
            let mut chosen = HashSet::new();
            while chosen.len() < m {
                let t = if urn.is_empty() {
                    self.rng.random_range(0..v)
                } else {
                    urn[self.rng.random_range(0..urn.len())]
                };
                chosen.insert(t);
            }
            let mut chosen: Vec<_> = chosen.into_iter().collect();
            chosen.sort_unstable();
            for t in chosen {
                edges.push((v, t));
                urn.extend([v, t]);
            }
        }
        Ok(Structure {
            vertex_count: n,
            edges,
            ..Default::default()
        })
    }
}

/// Watts–Strogatz small world: ring lattice with `k` nearest neighbours (even), each edge
/// rewired with probability `p`.
pub struct WattsStrogatz<R> {
    pub n: usize,
    pub k: usize,
    pub p: f64,
    pub rng: R,
}
impl<R: Rng> GraphGenerator for WattsStrogatz<R> {
    fn structure(&mut self, _: bool) -> Result<Structure> {
        let (n, k) = (self.n, self.k);
        if k % 2 != 0 || k >= n || !(0.0..=1.0).contains(&self.p) {
            return invalid("require even k < n and p in [0, 1]");
        }
        let norm = |a: usize, b: usize| (a.min(b), a.max(b));
        let mut edges: Vec<(usize, usize)> = (0..n)
            .flat_map(|i| (1..=k / 2).map(move |j| (i, (i + j) % n)))
            .collect();
        let mut present: HashSet<_> = edges.iter().map(|&(a, b)| norm(a, b)).collect();
        for e in edges.iter_mut() {
            if self.rng.random_bool(self.p) {
                let (a, b) = *e;
                // give up after a few tries when the vertex is already adjacent to everything
                for _ in 0..4 * n {
                    let w = self.rng.random_range(0..n);
                    if w != a && !present.contains(&norm(a, w)) {
                        present.remove(&norm(a, b));
                        present.insert(norm(a, w));
                        *e = (a, w);
                        break;
                    }
                }
            }
        }
        Ok(Structure {
            vertex_count: n,
            edges,
            ..Default::default()
        })
    }
}

/// Labelled tree on `n` vertices from a Prüfer sequence (random unless given).
pub struct Prufer<R> {
    pub n: usize,
    pub sequence: Option<Vec<usize>>,
    pub rng: R,
}
impl<R: Rng> GraphGenerator for Prufer<R> {
    fn structure(&mut self, _: bool) -> Result<Structure> {
        let n = self.n;
        if n == 0 {
            return invalid("tree needs at least one vertex");
        }
        if n <= 2 {
            return Ok(Structure {
                vertex_count: n,
                edges: if n == 2 { vec![(0, 1)] } else { vec![] },
                ..Default::default()
            });
        }
        let seq = match &self.sequence {
            Some(s) if s.len() == n - 2 && s.iter().all(|&x| x < n) => s.clone(),
            Some(_) => return invalid("Prüfer sequence must have n-2 entries below n"),
            None => (0..n - 2).map(|_| self.rng.random_range(0..n)).collect(),
        };
        let mut degree = vec![1usize; n];
        seq.iter().for_each(|&x| degree[x] += 1);
        let mut edges = Vec::with_capacity(n - 1);
        let mut leaves: std::collections::BTreeSet<usize> =
            (0..n).filter(|&v| degree[v] == 1).collect();
        for &x in &seq {
            let leaf = leaves.pop_first().expect("a leaf always exists");
            edges.push((leaf, x));
            degree[x] -= 1;
            if degree[x] == 1 {
                leaves.insert(x);
            }
        }
        let (a, b) = (leaves.pop_first().unwrap(), leaves.pop_first().unwrap());
        edges.push((a, b));
        Ok(Structure {
            vertex_count: n,
            edges,
            ..Default::default()
        })
    }
}
