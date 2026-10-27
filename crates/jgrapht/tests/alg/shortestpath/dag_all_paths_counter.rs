use jgrapht::alg::shortestpath::DagAllPathsCounter;
use jgrapht::{DiGraph, GraphMut, GraphMutExt};

fn graph(
    vertices: &[&'static str],
    edges: &[(&'static str, &'static str)],
) -> DiGraph<&'static str> {
    let mut graph = DiGraph::simple();
    graph.add_vertices(vertices.iter().copied());
    for &(source, target) in edges {
        graph.add_edge(&source, &target).unwrap();
    }
    graph
}

#[test]
fn test_diamond_two_paths() {
    let graph = graph(
        &["A", "B", "C", "D"],
        &[("A", "B"), ("A", "C"), ("B", "D"), ("C", "D")],
    );

    assert_eq!(
        DagAllPathsCounter::count_all_paths(&graph, &"A", &"D").unwrap(),
        2u8.into()
    );
}

#[test]
fn test_source_equals_target() {
    let graph = graph(&["X"], &[]);

    assert_eq!(
        DagAllPathsCounter::count_all_paths(&graph, &"X", &"X").unwrap(),
        1u8.into()
    );
}

#[test]
fn test_no_path() {
    let graph = graph(&["A", "B"], &[]);

    assert_eq!(
        DagAllPathsCounter::count_all_paths(&graph, &"A", &"B").unwrap(),
        0u8.into()
    );
}

#[test]
fn test_reject_cycle() {
    let graph = graph(&["A", "B"], &[("A", "B"), ("B", "A")]);

    assert!(DagAllPathsCounter::count_all_paths(&graph, &"A", &"B").is_err());
}

#[test]
fn test_big_integer_growth() {
    let graph = graph(
        &["S", "T", "U1", "U2", "U3", "V1", "V2", "V3", "V4"],
        &[
            ("S", "U1"),
            ("S", "U2"),
            ("S", "U3"),
            ("U1", "V1"),
            ("U1", "V2"),
            ("U1", "V3"),
            ("U1", "V4"),
            ("U2", "V1"),
            ("U2", "V2"),
            ("U2", "V3"),
            ("U2", "V4"),
            ("U3", "V1"),
            ("U3", "V2"),
            ("U3", "V3"),
            ("U3", "V4"),
            ("V1", "T"),
            ("V2", "T"),
            ("V3", "T"),
            ("V4", "T"),
        ],
    );

    assert_eq!(
        DagAllPathsCounter::count_all_paths(&graph, &"S", &"T").unwrap(),
        12u8.into()
    );
}
