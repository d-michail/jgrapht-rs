//! Port of `org.jgrapht.graph.ListenableGraphTest`.

use jgrapht::event::{GraphListener, Listenable};
use jgrapht::*;
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Default)]
struct State {
    last_added_edge: Option<DefaultEdge>,
    last_removed_edge: Option<DefaultEdge>,
    last_added_vertex: Option<String>,
    last_removed_vertex: Option<String>,
    last_weight_update: Option<f64>,
}

#[derive(Clone)]
struct MyGraphListener(Rc<RefCell<State>>);

impl GraphListener<String, DefaultEdge> for MyGraphListener {
    fn vertex_added(&mut self, v: &String) {
        self.0.borrow_mut().last_added_vertex = Some(v.clone());
    }
    fn vertex_removed(&mut self, v: &String) {
        self.0.borrow_mut().last_removed_vertex = Some(v.clone());
    }
    fn edge_added(&mut self, e: &DefaultEdge, _: &String, _: &String) {
        self.0.borrow_mut().last_added_edge = Some(*e);
    }
    fn edge_removed(&mut self, e: &DefaultEdge, _: &String, _: &String) {
        self.0.borrow_mut().last_removed_edge = Some(*e);
    }
    fn edge_weight_changed(&mut self, _: &DefaultEdge, _: f64, new: f64) {
        self.0.borrow_mut().last_weight_update = Some(new);
    }
}

/// Only the vertex callbacks; the edge ones keep their no-op defaults (a `VertexSetListener`).
struct VertexOnly(Rc<RefCell<State>>);
impl GraphListener<String, DefaultEdge> for VertexOnly {
    fn vertex_added(&mut self, v: &String) {
        self.0.borrow_mut().last_added_vertex = Some(v.clone());
    }
    fn vertex_removed(&mut self, v: &String) {
        self.0.borrow_mut().last_removed_vertex = Some(v.clone());
    }
}

fn s(x: &str) -> String {
    x.to_string()
}

fn init(state: &Rc<RefCell<State>>) {
    *state.borrow_mut() = State::default();
}

fn run_listener_scenario(
    g: &mut Listenable<UGraph<String>>,
    state: &Rc<RefCell<State>>,
    listener_id: usize,
    edges_notified: bool,
) {
    let (v1, v2) = (s("v1"), s("v2"));
    g.add_vertex(v1.clone());
    assert_eq!(state.borrow().last_added_vertex.as_deref(), Some("v1"));
    assert_eq!(state.borrow().last_removed_vertex, None);

    init(state);
    g.remove_vertex(&v1);
    assert_eq!(state.borrow().last_removed_vertex.as_deref(), Some("v1"));
    assert_eq!(state.borrow().last_added_vertex, None);

    g.add_vertex(v1.clone());
    g.add_vertex(v2.clone());
    init(state);
    let e = g.add_edge(&v1, &v2).unwrap().unwrap();
    assert_eq!(state.borrow().last_added_edge, edges_notified.then_some(e));
    assert_eq!(state.borrow().last_removed_edge, None);

    init(state);
    assert!(g.remove_edge(&e));
    assert_eq!(
        state.borrow().last_removed_edge,
        edges_notified.then_some(e)
    );
    assert_eq!(state.borrow().last_added_edge, None);

    g.remove_vertex(&v1);
    g.remove_vertex(&v2);
    assert!(g.remove_listener(listener_id));

    init(state);
    g.add_vertex(v1.clone());
    g.add_vertex(v2.clone());
    let e = g.add_edge(&v1, &v2).unwrap().unwrap();
    g.remove_edge(&e);
    let st = state.borrow();
    assert!(st.last_added_edge.is_none() && st.last_added_vertex.is_none());
    assert!(st.last_removed_edge.is_none() && st.last_removed_vertex.is_none());
}

#[test]
fn test_graph_listener() {
    let state = Rc::new(RefCell::new(State::default()));
    let mut g = Listenable::new(UGraph::<String>::simple());
    let id = g.add_listener(MyGraphListener(state.clone()));
    run_listener_scenario(&mut g, &state, id, true);
}

#[test]
fn test_vertex_set_listener() {
    let state = Rc::new(RefCell::new(State::default()));
    let mut g = Listenable::new(UGraph::<String>::simple());
    let id = g.add_listener(VertexOnly(state.clone()));
    run_listener_scenario(&mut g, &state, id, false);
}

#[test]
fn test_with_vertex_supplier() {
    let wrapped = UGraph::<String>::simple().with_vertex_factory(|i| (i + 15).to_string());
    let mut g = Listenable::new(wrapped);
    let state = Rc::new(RefCell::new(State::default()));
    g.add_listener(VertexOnly(state.clone()));
    g.add_new_vertex().unwrap();
    assert_eq!(state.borrow().last_added_vertex.as_deref(), Some("15"));
    g.add_vertex(s("other"));
    assert_eq!(state.borrow().last_added_vertex.as_deref(), Some("other"));
    g.add_new_vertex().unwrap();
    assert_eq!(state.borrow().last_added_vertex.as_deref(), Some("16"));
}

#[test]
fn test_listenable_directed_weighted_graph() {
    let state = Rc::new(RefCell::new(State::default()));
    let mut g = Listenable::new(DiGraph::<String>::with_loops().weighted());
    g.add_listener(MyGraphListenerDi(state.clone()));
    let (v1, v2) = (s("v1"), s("v2"));
    g.add_vertex(v1.clone());
    assert_eq!(state.borrow().last_added_vertex.as_deref(), Some("v1"));
    assert_eq!(state.borrow().last_removed_vertex, None);
    g.add_vertex(v2.clone());
    init(&state);
    let e = g.add_edge(&v1, &v2).unwrap().unwrap();
    g.set_edge_weight(&e, 10.0).unwrap();
    assert_eq!(g.edge_weight(&e), Some(10.0));
    assert_eq!(state.borrow().last_added_edge, Some(e));
    assert_eq!(state.borrow().last_removed_edge, None);
}

struct MyGraphListenerDi(Rc<RefCell<State>>);
impl GraphListener<String, DefaultEdge> for MyGraphListenerDi {
    fn vertex_added(&mut self, v: &String) {
        self.0.borrow_mut().last_added_vertex = Some(v.clone());
    }
    fn edge_added(&mut self, e: &DefaultEdge, _: &String, _: &String) {
        self.0.borrow_mut().last_added_edge = Some(*e);
    }
    fn edge_weight_changed(&mut self, _: &DefaultEdge, _: f64, new: f64) {
        self.0.borrow_mut().last_weight_update = Some(new);
    }
}

#[test]
fn test_listenable_directed_weighted_graph_with_custom_edge() {
    // `DefaultEdge` on a weighted graph: same scenario with weight-update notifications
    let state = Rc::new(RefCell::new(State::default()));
    let mut g = Listenable::new(DiGraph::<String>::with_loops().weighted());
    g.add_listener(MyGraphListenerDi(state.clone()));
    let (v1, v2) = (s("v1"), s("v2"));
    g.add_vertex(v1.clone());
    g.add_vertex(v2.clone());
    init(&state);
    let e = g.add_edge(&v1, &v2).unwrap().unwrap();
    g.set_edge_weight(&e, 10.0).unwrap();
    assert_eq!(g.edge_weight(&e), Some(10.0));
    assert_eq!(state.borrow().last_added_edge, Some(e));
    assert_eq!(state.borrow().last_removed_edge, None);

    init(&state);
    g.set_edge_weight(&e, 5.5).unwrap();
    assert!((g.edge_weight(&e).unwrap() - 5.5).abs() < 1e-9);
    assert_eq!(state.borrow().last_added_edge, None);
    assert_eq!(state.borrow().last_removed_edge, None);
    assert_eq!(state.borrow().last_weight_update, Some(5.5));
    g.set_edge_weight(&e, 20.5).unwrap();
    assert!((g.edge_weight(&e).unwrap() - 20.5).abs() < 1e-9);
    assert_eq!(state.borrow().last_weight_update, Some(20.5));
}
