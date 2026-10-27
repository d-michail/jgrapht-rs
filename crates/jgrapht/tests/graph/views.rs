use jgrapht::event::{GraphListener, Listenable};
use jgrapht::graph::DirectedExt;
use jgrapht::*;
use std::cell::RefCell;
use std::rc::Rc;

fn path_digraph() -> DiGraph<u32> {
    let mut g = DiGraph::<u32>::simple().weighted();
    for (a, b, w) in [(1, 2, 1.0), (2, 3, 2.0), (3, 4, 3.0)] {
        g.add_edge_with_vertices(a, b).unwrap();
        let e = *g.get_edge(&a, &b).unwrap();
        g.set_edge_weight(&e, w).unwrap();
    }
    g
}

#[test]
fn reversed_and_undirected() {
    let g = path_digraph();
    let r = EdgeReversed(&g);
    assert!(r.contains_edge_between(&2, &1) && !r.contains_edge_between(&1, &2));
    assert_eq!(r.successors_of(&3).collect::<Vec<_>>(), [&2]);
    let u = AsUndirected(&g);
    assert!(u.contains_edge_between(&2, &1) && u.contains_edge_between(&1, &2));
    assert_eq!(u.degree_of(&2).unwrap(), 2);
    assert!(!u.graph_type().directed);
}

#[test]
fn subgraphs() {
    let g = path_digraph();
    let s = AsSubgraph::induced(&g, [1, 2, 3, 99]);
    assert_eq!((s.vertex_count(), s.edge_count()), (3, 2));
    assert!(!s.contains_vertex(&4));
    assert_eq!(s.out_edges_of(&3).unwrap().count(), 0);
    let m = MaskSubgraph::new(&g, |v: &u32| !v.is_multiple_of(2), |_: &DefaultEdge| false);
    assert_eq!((m.vertex_count(), m.edge_count()), (2, 0));
}

#[test]
fn weights_and_union() {
    let g = path_digraph();
    let uw = AsUnweighted(&g);
    assert_eq!(uw.total_weight(), 3.0);
    let w = AsWeighted::from_function(&uw, |_| 5.0, false, false).unwrap();
    assert_eq!(w.total_weight(), 15.0);
    let mut h = DiGraph::<u32>::simple();
    h.add_edge_with_vertices(4, 5).unwrap();
    let u = GraphUnion::new(&g, &h);
    assert_eq!(u.vertex_count(), 5);
    assert_eq!(u.edge_count(), 4); // default edges are unique across graphs
}

#[derive(Default)]
struct Log(Rc<RefCell<Vec<String>>>);
impl GraphListener<u32, DefaultEdge> for Log {
    fn vertex_added(&mut self, v: &u32) {
        self.0.borrow_mut().push(format!("+v{v}"));
    }
    fn vertex_removed(&mut self, v: &u32) {
        self.0.borrow_mut().push(format!("-v{v}"));
    }
    fn edge_added(&mut self, _: &DefaultEdge, s: &u32, t: &u32) {
        self.0.borrow_mut().push(format!("+e{s}{t}"));
    }
    fn edge_removed(&mut self, _: &DefaultEdge, s: &u32, t: &u32) {
        self.0.borrow_mut().push(format!("-e{s}{t}"));
    }
}

#[test]
fn listenable_events() {
    let log = Rc::new(RefCell::new(vec![]));
    let mut g = Listenable::new(UGraph::<u32>::simple());
    g.add_listener(Log(log.clone()));
    g.add_edge_with_vertices(1, 2).unwrap();
    g.remove_vertex(&1);
    assert_eq!(*log.borrow(), ["+v1", "+v2", "+e12", "-e12", "-v1"]);
}

#[test]
fn walks() {
    let g = path_digraph();
    let w = GraphWalk::from_vertices(&g, vec![1, 2, 3, 4]).unwrap();
    assert_eq!((w.length(), w.weight), (3, 6.0));
    assert!(w.is_valid(&g));
    let a = GraphWalk::from_vertices(&g, vec![1, 2, 3]).unwrap();
    let b = GraphWalk::from_vertices(&g, vec![3, 4]).unwrap();
    assert_eq!(a.concat_sum(&b).unwrap(), w);
    assert!(GraphWalk::from_vertices(&g, vec![1, 3]).is_none());
}
