use jgrapht::alg::transitive::{TransitiveClosure, TransitiveReduction};
use jgrapht::alg::{chordal::*, connectivity::*, cycle::*, lca::Lca};
use jgrapht::generate::*;
use jgrapht::*;

fn ug(edges: &[(u32, u32)]) -> UGraph<u32> {
    let mut g = UGraph::<u32>::simple();
    for &(a, b) in edges {
        g.add_edge_with_vertices(a, b).unwrap();
    }
    g
}
fn dg(edges: &[(u32, u32)]) -> DiGraph<u32> {
    let mut g = DiGraph::<u32>::simple();
    for &(a, b) in edges {
        g.add_edge_with_vertices(a, b).unwrap();
    }
    g
}
fn sorted(mut cs: Vec<Vec<u32>>) -> Vec<Vec<u32>> {
    cs.iter_mut().for_each(|c| c.sort());
    cs.sort();
    cs
}

#[test]
fn components() {
    let g = ug(&[(1, 2), (2, 3), (4, 5)]);
    assert_eq!(connected_components(&g), [vec![1, 2, 3], vec![4, 5]]);
    assert_eq!(connected_component_of(&g, &5), Some(vec![4, 5]));
}

#[test]
fn scc() {
    let g = dg(&[(1, 2), (2, 3), (3, 1), (3, 4), (4, 5), (5, 4)]);
    let expect = vec![vec![1, 2, 3], vec![4, 5]];
    assert_eq!(sorted(strongly_connected_components(&g)), expect);
    assert_eq!(sorted(strongly_connected_components_kosaraju(&g)), expect);
    assert!(!is_strongly_connected(&g));
    let (dag, comps) = condensation(&g);
    assert_eq!(
        (dag.vertex_count(), dag.edge_count(), comps.len()),
        (2, 1, 2)
    );
    assert!(jgrapht::graph::props::is_dag(&dag));
}

#[test]
fn biconnected() {
    // two triangles sharing vertex 3, plus a pendant bridge 5-6
    let g = ug(&[(1, 2), (2, 3), (3, 1), (3, 4), (4, 5), (5, 3), (5, 6)]);
    let b = biconnectivity(&g);
    let mut art = b.articulation_points.clone();
    art.sort();
    assert_eq!(art, [3, 5]);
    assert_eq!(b.bridges.len(), 1);
    let mut sizes: Vec<_> = b.blocks.iter().map(Vec::len).collect();
    sizes.sort();
    assert_eq!(sizes, [1, 3, 3]);
    assert!(!is_biconnected(&g));
    let mut ring = UGraph::<usize>::simple();
    Ring(6).generate_indexed(&mut ring).unwrap();
    assert!(is_biconnected(&ring));
}

#[test]
fn cycles() {
    let g = dg(&[(1, 2), (2, 3), (3, 1), (2, 4), (4, 2)]);
    assert!(has_cycle(&g));
    assert_eq!(sorted(simple_cycles(&g)), [vec![1, 2, 3], vec![2, 4]]);
    assert!(!has_cycle(&dg(&[(1, 2), (2, 3)])));
    let k4 = {
        let mut g = UGraph::<usize>::simple();
        Complete(4).generate_indexed(&mut g).unwrap();
        g
    };
    assert_eq!(simple_cycles(&k4).len(), 7); // 4 triangles + 3 squares
    assert_eq!(cycle_basis(&k4).len(), 3); // m - n + 1
    assert!(has_cycle(&k4) && !has_cycle(&ug(&[(1, 2), (2, 3)])));
}

#[test]
fn eulerian() {
    let mut g = UGraph::<usize>::simple();
    Ring(5).generate_indexed(&mut g).unwrap();
    let w = HierholzerEulerianCycle::new().get_eulerian_cycle(&g).unwrap();
    assert_eq!(w.length(), 5);
    assert_eq!(w.start_vertex(), w.end_vertex());
    assert!(w.is_valid(&g));
    assert!(!HierholzerEulerianCycle::new().is_eulerian(&ug(&[(1, 2), (2, 3)])));
    let d = dg(&[(1, 2), (2, 3), (3, 1), (1, 4), (4, 1)]);
    let w = HierholzerEulerianCycle::new().get_eulerian_cycle(&d).unwrap();
    assert_eq!(w.length(), 5);
    assert!(w.is_valid(&d));
    assert!(!HierholzerEulerianCycle::new().is_eulerian(&ug(&[(1, 2), (3, 4)])));
}
#[test]
fn chordal() {
    let tri_fan = ug(&[(1, 2), (2, 3), (3, 1), (3, 4), (4, 2)]);
    assert!(is_chordal(&tri_fan));
    assert_eq!(chordal_chromatic_number(&tri_fan), Some(3));
    assert_eq!(chordal_maximal_cliques(&tri_fan).unwrap().len(), 2);
    let c4 = ug(&[(1, 2), (2, 3), (3, 4), (4, 1)]);
    assert!(!is_chordal(&c4));
    assert_eq!(chordal_maximum_independent_set(&tri_fan).unwrap().len(), 2);
}

#[test]
fn closure_reduction() {
    let mut g = dg(&[(1, 2), (2, 3), (3, 4)]);
    TransitiveClosure::INSTANCE
        .close_simple_directed_graph(&mut g)
        .unwrap();
    assert_eq!(g.edge_count(), 6);
    TransitiveReduction::INSTANCE.reduce(&mut g).unwrap();
    assert_eq!(g.edge_count(), 3);
}

#[test]
fn lca() {
    let t = dg(&[(1, 2), (1, 3), (2, 4), (2, 5), (3, 6), (5, 7)]);
    let l = Lca::new(&t, &1).unwrap();
    assert_eq!(l.lca(&4, &7), Some(&2));
    assert_eq!(l.lca(&7, &6), Some(&1));
    assert_eq!(l.lca(&5, &7), Some(&5));
    assert_eq!(l.depth(&7), Some(3));
    assert_eq!(l.lca(&4, &99), None);
}
