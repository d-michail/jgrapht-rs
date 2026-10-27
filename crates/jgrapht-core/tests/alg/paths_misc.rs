use jgrapht::alg::connectivity::block_cut_tree;
use jgrapht::alg::eulerian::*;
use jgrapht::alg::matching::*;
use jgrapht::alg::paths::*;
use jgrapht::alg::tour::*;
use jgrapht::generate::*;
use jgrapht::*;
use rand::{Rng, SeedableRng, rngs::StdRng};

fn dg(edges: &[(u32, u32, f64)]) -> DiGraph<u32> {
    let mut g = DiGraph::<u32>::simple().weighted();
    for &(a, b, w) in edges {
        g.add_vertex(a);
        g.add_vertex(b);
        g.add_weighted_edge(&a, &b, w).unwrap();
    }
    g
}

#[test]
fn simple_paths() {
    let g = dg(&[
        (1, 2, 1.0),
        (2, 4, 1.0),
        (1, 3, 1.0),
        (3, 4, 1.0),
        (2, 3, 1.0),
    ]);
    let ps = all_simple_paths(&g, &1, &4, None);
    assert_eq!(ps.len(), 3);
    assert!(ps.iter().all(|p| p.is_valid(&g)));
    assert_eq!(all_simple_paths(&g, &1, &4, Some(2)).len(), 2);
    assert!(all_simple_paths(&g, &4, &1, None).is_empty());
}

#[test]
fn negative_cycles() {
    let g = dg(&[(1, 2, 1.0), (2, 3, -2.0), (3, 1, 0.5), (3, 4, 1.0)]);
    let mut c = find_negative_cycle(&g).unwrap();
    c.sort();
    assert_eq!(c, [1, 2, 3]);
    assert!(find_negative_cycle(&dg(&[(1, 2, -1.0), (2, 3, 1.0)])).is_none());
}

#[test]
fn disjoint_paths() {
    // two routes from 0 to 5 sharing vertex 3 only in the edge-disjoint case
    let g = dg(&[
        (0, 1, 1.0),
        (1, 3, 1.0),
        (0, 2, 1.0),
        (2, 3, 1.0),
        (3, 4, 1.0),
        (3, 5, 1.0),
        (4, 5, 1.0),
        (0, 5, 10.0),
    ]);
    let ep = edge_disjoint_shortest_paths(&g, &0, &5, 3).unwrap();
    assert_eq!(ep.len(), 3);
    assert!(ep.iter().all(|p| p.is_valid(&g)));
    let all_edges: Vec<_> = ep.iter().flat_map(|p| p.edges.clone()).collect();
    let mut dedup = all_edges.clone();
    dedup.sort();
    dedup.dedup();
    assert_eq!(all_edges.len(), dedup.len());
    let vp = vertex_disjoint_shortest_paths(&g, &0, &5, 3).unwrap();
    assert_eq!(vp.len(), 2); // only the direct edge and one path through vertex 3
    assert_eq!(vp.iter().map(|p| p.weight).sum::<f64>(), 3.0 + 10.0);
    assert!(edge_disjoint_shortest_paths(&g, &0, &0, 1).is_err());
}

#[test]
fn eulerian_trails() {
    let mut g = UGraph::<usize>::simple();
    Linear(5).generate_indexed(&mut g).unwrap();
    let p = eulerian_path(&g).unwrap();
    assert!(p.is_valid(&g) && p.length() == 4);
    assert!(eulerian_cycle(&g).is_none());
    let d = dg(&[(1, 2, 1.0), (2, 3, 1.0), (3, 1, 1.0), (3, 4, 1.0)]);
    let p = eulerian_path(&d).unwrap();
    assert_eq!(
        (
            p.start_vertex().copied(),
            p.end_vertex().copied(),
            p.length()
        ),
        (Some(3), Some(4), 4)
    );
}

#[test]
fn hamiltonian() {
    // Java has no exact Hamiltonian cycle search for general graphs (see `alg_tour_tests`), so
    // this only checks the path search.
    fn path<G: Graph>(g: &G) -> Option<GraphWalk<G::Vertex, G::Edge>> {
        BacktrackingHamiltonianPath::new()
            .get_path(g)
            .unwrap()
            .path()
            .cloned()
    }
    let mut ring = UGraph::<usize>::simple();
    Ring(8).generate_indexed(&mut ring).unwrap();
    assert!(path(&ring).unwrap().is_valid(&ring));
    let mut star = UGraph::<usize>::simple();
    Star(5).generate_indexed(&mut star).unwrap();
    assert!(path(&star).is_none());
    let mut line = UGraph::<usize>::simple();
    Linear(6).generate_indexed(&mut line).unwrap();
    assert!(path(&line).unwrap().is_valid(&line));
    let mut k33 = UGraph::<usize>::simple();
    CompleteBipartite(3, 3).generate_indexed(&mut k33).unwrap();
    assert!(path(&k33).is_some());
    let mut k23 = UGraph::<usize>::simple();
    CompleteBipartite(2, 3).generate_indexed(&mut k23).unwrap();
    assert!(path(&k23).is_some());
    let mut k24 = UGraph::<usize>::simple();
    CompleteBipartite(2, 4).generate_indexed(&mut k24).unwrap();
    assert!(path(&k24).is_none());
}

#[test]
fn block_cut() {
    let mut g = UGraph::<u32>::simple();
    for (a, b) in [(1, 2), (2, 3), (3, 1), (3, 4), (4, 5), (5, 3), (5, 6)] {
        g.add_edge_with_vertices(a, b).unwrap();
    }
    g.add_vertex(9);
    let t = block_cut_tree(&g);
    assert_eq!(t.blocks.len(), 4); // two triangles, the bridge, the isolated vertex
    let mut cuts = t.cut_vertices.clone();
    cuts.sort();
    assert_eq!(cuts, [3, 5]);
    assert_eq!(t.edges.len(), 4);
}

#[test]
fn approximate_and_exact_weighted_matching() {
    let mut rng = StdRng::seed_from_u64(12);
    for _ in 0..30 {
        let mut g = UGraph::<usize>::simple().weighted();
        GnpRandomGraphGenerator::new(10, 0.4, StdRng::seed_from_u64(rng.random()), false)
            .unwrap()
            .generate_indexed(&mut g)
            .unwrap();
        for e in g.edges().cloned().collect::<Vec<_>>() {
            g.set_edge_weight(&e, rng.random_range(1..10) as f64)
                .unwrap();
        }
        let exact = maximum_weight_matching_exact(&g).unwrap();
        assert!(is_matching(&g, &exact.edges));
        let pg = path_growing_matching(&g);
        assert!(is_matching(&g, &pg.edges));
        assert!(2.0 * pg.weight >= exact.weight - 1e-9 && pg.weight <= exact.weight + 1e-9);
        assert!(exact.len() <= edmonds_blossom(&g).len());
    }
}

#[test]
fn christofides_is_within_bound() {
    let mut rng = StdRng::seed_from_u64(13);
    for _ in 0..10 {
        let pts: Vec<(f64, f64)> = (0..9)
            .map(|_| (rng.random::<f64>() * 10.0, rng.random::<f64>() * 10.0))
            .collect();
        let mut g = UGraph::<usize>::simple().weighted();
        g.add_vertices(0..9);
        for i in 0..9 {
            for j in i + 1..9 {
                let w = ((pts[i].0 - pts[j].0).powi(2) + (pts[i].1 - pts[j].1).powi(2)).sqrt();
                g.add_weighted_edge(&i, &j, w).unwrap();
            }
        }
        let c = christofides_tour(&g).unwrap();
        assert!(c.is_valid(&g) && c.length() == 9);
        let opt = held_karp_tour(&g).unwrap();
        assert!(c.weight >= opt.weight - 1e-9 && c.weight <= 1.5 * opt.weight + 1e-9);
    }
}
