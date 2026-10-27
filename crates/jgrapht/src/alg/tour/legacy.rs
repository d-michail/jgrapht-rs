//! Temporary Christofides helper, retained until the Blossom V matching port is complete.

use crate::alg::cycle::EulerianCycleAlgorithm;
use crate::alg::spanning::kruskal;
use crate::alg::util::Indexed;
use crate::graph::Graph;
use crate::walk::GraphWalk;
use crate::{GraphError, Result};

fn matrix<G: Graph>(ix: &Indexed<'_, G>) -> Vec<Vec<f64>> {
    let n = ix.n();
    let mut d = vec![vec![f64::INFINITY; n]; n];
    for (k, &(a, b)) in ix.ends.iter().enumerate() {
        if a != b {
            d[a][b] = d[a][b].min(ix.w[k]);
            if !ix.directed {
                d[b][a] = d[b][a].min(ix.w[k]);
            }
        }
    }
    d
}

fn to_walk<G: Graph>(
    g: &G,
    ix: &Indexed<'_, G>,
    order: &[usize],
) -> Result<GraphWalk<G::Vertex, G::Edge>> {
    let mut vs = ix.vertices_of(order);
    if let Some(first) = vs.first().cloned() {
        vs.push(first);
    }
    GraphWalk::from_vertices(g, vs).ok_or_else(|| {
        GraphError::InvalidArgument("graph has no Hamiltonian cycle along these edges".into())
    })
}

fn check(n: usize) -> Result<()> {
    if n < 3 {
        return Err(GraphError::InvalidArgument(
            "a tour needs at least 3 vertices".into(),
        ));
    }
    Ok(())
}

/// Christofides' 3/2-approximation for metric instances: MST, a minimum-weight perfect matching
/// of its odd-degree vertices (exact for up to 20 such vertices, greedy beyond), an Eulerian
/// circuit of the union and shortcutting.
pub fn christofides_tour<G: Graph>(g: &G) -> Result<GraphWalk<G::Vertex, G::Edge>> {
    use crate::{BaseGraph, DefaultEdge, GraphMut, GraphMutExt};
    let ix = Indexed::new(g);
    let n = ix.n();
    check(n)?;
    let d = matrix(&ix);
    let tree = kruskal(g);
    let mut multi = BaseGraph::<usize, DefaultEdge, crate::graph::Undirected>::multi();
    multi.add_vertices(0..n);
    let mut deg = vec![0usize; n];
    for e in &tree.edges {
        let k = ix.es.iter().position(|x| *x == e).unwrap();
        let (a, b) = ix.ends[k];
        multi.add_edge(&a, &b)?;
        deg[a] += 1;
        deg[b] += 1;
    }
    if tree.edges.len() + 1 != n {
        return Err(GraphError::InvalidArgument("graph is not connected".into()));
    }
    let odd: Vec<usize> = (0..n).filter(|&v| deg[v] % 2 == 1).collect();
    let pairs: Vec<(usize, usize)> = if odd.len() <= 20 {
        // DP over subsets of odd vertices
        let m = odd.len();
        let full = 1usize << m;
        let mut cost = vec![f64::INFINITY; full];
        let mut pick = vec![0usize; full];
        cost[0] = 0.0;
        for mask in 1..full {
            let i = mask.trailing_zeros() as usize;
            let rest = mask & (mask - 1);
            let mut others = rest;
            while others != 0 {
                let j = others.trailing_zeros() as usize;
                others &= others - 1;
                let c = d[odd[i]][odd[j]] + cost[rest & !(1 << j)];
                if c < cost[mask] {
                    cost[mask] = c;
                    pick[mask] = j;
                }
            }
        }
        let (mut mask, mut out) = (full - 1, Vec::new());
        while mask != 0 {
            let i = mask.trailing_zeros() as usize;
            let j = pick[mask];
            out.push((odd[i], odd[j]));
            mask &= !(1 << i);
            mask &= !(1 << j);
        }
        out
    } else {
        let mut left = odd.clone();
        let mut out = Vec::new();
        while let Some(a) = left.pop() {
            let (pos, _) = left
                .iter()
                .enumerate()
                .min_by(|x, y| d[a][*x.1].total_cmp(&d[a][*y.1]))
                .unwrap();
            out.push((a, left.swap_remove(pos)));
        }
        out
    };
    for (a, b) in pairs {
        multi.add_edge(&a, &b)?;
    }
    let circuit = crate::alg::cycle::HierholzerEulerianCycle::new().get_eulerian_cycle(&multi)?;
    let mut seen = vec![false; n];
    let mut order = Vec::new();
    for v in circuit.vertices {
        if !std::mem::replace(&mut seen[v], true) {
            order.push(v);
        }
    }
    to_walk(g, &ix, &order)
}
