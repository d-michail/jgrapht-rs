// (C) Copyright 2003-2026, by John V Sichi and JGraphT Contributors.
// Rust port (C) Copyright 2026, by Dimitrios Michail and jgrapht-rs Contributors.
//
// See the CONTRIBUTORS.md file distributed with this work for additional
// information regarding copyright ownership.
//
// This program and the accompanying materials are made available under the
// terms of the Eclipse Public License 2.0 which is available at
// http://www.eclipse.org/legal/epl-2.0, or the GNU Lesser General Public
// License v2.1 or later which is available at
// http://www.gnu.org/licenses/old-licenses/lgpl-2.1-standalone.html.
//
// SPDX-License-Identifier: EPL-2.0 OR LGPL-2.1-or-later

//! # User guide
//!
//! A tour of the library, organised like the JGraphT user guide: what a graph is, how to create
//! and query one, views, traversals, algorithms, generators, import and export, and the places
//! where Rust differs from Java. Every example is compiled and run as a doctest.
//!
//! 1. [Concepts](#concepts)
//! 2. [Hello jgrapht](#hello-jgrapht)
//! 3. [Graph types](#graph-types)
//! 4. [Creating graphs](#creating-graphs)
//! 5. [Querying graphs](#querying-graphs)
//! 6. [Writing generic code](#writing-generic-code)
//! 7. [Views](#views)
//! 8. [Listening to changes](#listening-to-changes)
//! 9. [Traversals](#traversals)
//! 10. [Algorithms](#algorithms)
//! 11. [Generators](#generators)
//! 12. [Import and export](#import-and-export)
//! 13. [Serialization](#serialization)
//! 14. [Randomness](#randomness)
//! 15. [Errors](#errors)
//! 16. [Performance](#performance)
//! 17. [Coming from JGraphT](#coming-from-jgrapht)
//!
//! # Concepts
//!
//! A graph is a set of *vertices* and a set of *edges* that join them. Here both are plain values
//! of your own types, as long as they are `Clone + Eq + Hash`: an `u32`, a `&str`, a `String`,
//! a struct with an id. Two vertices are the same vertex when they compare equal.
//!
//! An edge is a value too. A graph created with [`UGraph::simple`](crate::UGraph) or
//! [`DiGraph::simple`](crate::DiGraph) uses [`DefaultEdge`](crate::DefaultEdge), a small unique id
//! that the graph hands out, but you can bring your own edge type. The *weight* of an edge is an
//! `f64` stored by the graph, not by the edge value; every edge of an unweighted graph has weight
//! `1.0`.
//!
//! The library is built around traits, so algorithms say what they need:
//!
//! | Trait | What it offers |
//! |---|---|
//! | [`Graph`](crate::Graph) | read access: vertices, edges, endpoints, weights, `edges_between` |
//! | [`GraphMut`](crate::GraphMut) | add and remove vertices and edges, set weights |
//! | [`DirectedGraph`](crate::DirectedGraph), [`UndirectedGraph`](crate::UndirectedGraph) | marker for the direction of every edge |
//! | [`OutgoingGraph`](crate::OutgoingGraph) | edges leaving a vertex, out-degree |
//! | [`IncomingGraph`](crate::IncomingGraph) | edges entering a vertex, in-degree |
//! | [`IncidenceGraph`](crate::IncidenceGraph) | both of the above, plus all incident edges and degree |
//!
//! [`GraphExt`](crate::GraphExt), [`DirectedExt`](crate::DirectedExt) and
//! [`GraphMutExt`](crate::GraphMutExt) add convenience methods to every graph (neighbour lists,
//! `add_vertices`, `add_weighted_edge`, topological order, ...).
//!
//! # Hello jgrapht
//!
//! ```
//! use jgrapht::alg::shortestpath::{Dijkstra, ShortestPathAlgorithm};
//! use jgrapht::*;
//!
//! // An undirected, simple (no loops, no parallel edges), weighted graph.
//! let mut g = UGraph::<&str>::simple().weighted();
//! g.add_vertices(["Amsterdam", "Berlin", "Copenhagen", "Dublin"]);
//! g.add_weighted_edge(&"Amsterdam", &"Berlin", 577.0)?;
//! g.add_weighted_edge(&"Berlin", &"Copenhagen", 354.0)?;
//! g.add_weighted_edge(&"Amsterdam", &"Copenhagen", 1200.0)?;
//!
//! assert_eq!(g.vertex_count(), 4);
//! assert_eq!(g.edge_count(), 3);
//! assert_eq!(g.degree_of(&"Berlin"), 2);
//!
//! // The cheapest route goes through Berlin; Dublin is not connected.
//! let route = Dijkstra::new(&g).path(&"Amsterdam", &"Copenhagen")?.unwrap();
//! assert_eq!(route.vertex_list(), ["Amsterdam", "Berlin", "Copenhagen"]);
//! assert_eq!(route.weight(), 931.0);
//! assert!(Dijkstra::new(&g).path(&"Amsterdam", &"Dublin")?.is_none());
//! # Ok::<(), GraphError>(())
//! ```
//!
//! # Graph types
//!
//! What a graph allows is described by a [`GraphType`](crate::GraphType): directed or undirected,
//! self-loops, parallel edges, weights, cycles. The direction is part of the Rust type
//! ([`UGraph`](crate::UGraph) or [`DiGraph`](crate::DiGraph)); the rest is chosen by the
//! constructor:
//!
//! | Constructor | Self-loops | Parallel edges | JGraphT class |
//! |---|---|---|---|
//! | `simple()` | no | no | `SimpleGraph`, `SimpleDirectedGraph` |
//! | `multi()` | no | yes | `Multigraph`, `DirectedMultigraph` |
//! | `with_loops()` | yes | no | `DefaultUndirectedGraph`, `DefaultDirectedGraph` |
//! | `pseudo()` | yes | yes | `Pseudograph`, `DirectedPseudograph` |
//! | `DiGraph::dag()` | no | no, and no cycles | `DirectedAcyclicGraph` |
//! | `.weighted()` | | | the `Weighted` variants |
//!
//! ```
//! use jgrapht::*;
//!
//! let mut multi = UGraph::<u32>::multi();
//! multi.add_vertices([1, 2]);
//! let first = multi.add_edge(&1, &2)?.unwrap();
//! let second = multi.add_edge(&1, &2)?.unwrap();
//! assert_ne!(first, second); // two parallel edges
//! assert!(multi.add_edge(&1, &1).is_err()); // no self-loops
//!
//! let mut simple = UGraph::<u32>::simple();
//! simple.add_vertices([1, 2]);
//! assert!(simple.add_edge(&1, &2)?.is_some());
//! assert!(simple.add_edge(&1, &2)?.is_none()); // already present: no change
//!
//! // A DAG refuses an edge that would close a cycle.
//! let mut dag = DiGraph::<u32>::dag();
//! dag.add_vertices([1, 2]);
//! dag.add_edge(&1, &2)?;
//! assert_eq!(dag.add_edge(&2, &1).unwrap_err(), GraphError::CycleProhibited);
//! # Ok::<(), GraphError>(())
//! ```
//!
//! Besides [`BaseGraph`](crate::BaseGraph) (the adjacency-map graph behind `UGraph` and
//! `DiGraph`) there are compact graphs for integer vertices
//! ([`graph::sparse`](crate::graph::sparse)) and map-backed variants
//! ([`graph::map`](crate::graph::map)); see [Performance](#performance).
//!
//! # Creating graphs
//!
//! Add vertices first, then edges. `add_vertex` returns whether the vertex was new; `add_edge`
//! returns `Ok(Some(edge))` for a new edge, `Ok(None)` when a simple graph already has it, and
//! `Err` when the graph refuses it (unknown vertex, forbidden loop, cycle in a DAG).
//!
//! ```
//! use jgrapht::graph::builder::GraphBuilder;
//! use jgrapht::*;
//!
//! // Step by step ...
//! let mut g = DiGraph::<&str>::simple();
//! g.add_vertex("a");
//! g.add_vertices(["b", "c"]);
//! let ab = g.add_edge(&"a", &"b")?.unwrap();
//! g.add_weighted_edge(&"b", &"c", 2.5).unwrap_err(); // the graph is not weighted
//!
//! // ... or with a builder that chains the calls and reports the first error at the end.
//! let h = GraphBuilder::new(DiGraph::<&str>::simple().weighted())
//!     .add_edge_chain(["a", "b", "c"])
//!     .add_weighted_edge("c", "a", 4.0)
//!     .build()?;
//! assert_eq!(h.edge_count(), 3);
//! assert!(g.contains_edge(&ab));
//! # Ok::<(), GraphError>(())
//! ```
//!
//! To use your own edge type, create the graph with an edge factory
//! ([`BaseGraph::with_edge_factory`](crate::BaseGraph::with_edge_factory)) or add every edge with
//! [`add_edge_with`](crate::GraphMut::add_edge_with). Vertex values can be created by the graph too:
//! install a factory with `with_vertex_factory` and call `add_new_vertex`.
//!
//! ```
//! use jgrapht::*;
//!
//! // Edges are the road names themselves.
//! let mut g = UGraph::<u32, String>::without_edge_factory(GraphType::simple());
//! g.add_vertices([1, 2, 3]);
//! g.add_edge_with(&1, &2, "A1".to_string())?;
//! g.add_edge_with(&2, &3, "N7".to_string())?;
//! assert_eq!(g.edge_source(&"N7".to_string()), Some(&2));
//! # Ok::<(), GraphError>(())
//! ```
//!
//! # Querying graphs
//!
//! ```
//! use jgrapht::*;
//!
//! let mut g = DiGraph::<u32>::simple().weighted();
//! g.add_vertices(1..=4);
//! for (u, v, w) in [(1, 2, 1.0), (1, 3, 4.0), (2, 3, 2.0), (3, 4, 1.0)] {
//!     g.add_weighted_edge(&u, &v, w)?;
//! }
//!
//! // Counts and membership.
//! assert_eq!((g.vertex_count(), g.edge_count()), (4, 4));
//! assert!(g.contains_vertex(&3));
//!
//! // Edges between two vertices, endpoints and weights of an edge.
//! let e = *g.get_edge(&1, &3).unwrap();
//! assert_eq!(g.edge_endpoints(&e), Some((&1, &3)));
//! assert_eq!(g.edge_weight(&e), Some(4.0));
//! assert_eq!(g.edges_between(&1, &3).count(), 1);
//!
//! // Degrees and neighbours.
//! assert_eq!(g.out_degree_of(&1), 2);
//! assert_eq!(g.in_degree_of(&3), 2);
//! assert_eq!(g.degree_of(&3), 3);
//! let mut next = g.successor_list_of(&1);
//! next.sort();
//! assert_eq!(next, [2, 3]);
//!
//! // Iterate over everything.
//! let total: f64 = g.edges_cloned().map(|e| g.weight_of(&e)).sum();
//! assert_eq!(total, 8.0);
//! assert_eq!(g.topological_order(), Some(vec![1, 2, 3, 4]));
//! # Ok::<(), GraphError>(())
//! ```
//!
//! A query about something that is not in the graph answers with `None` or an empty iterator, never
//! with a panic: `edge_weight` of an unknown edge is `None`, `edges_between` of an unknown vertex
//! is empty. Operations that need a vertex to exist (a shortest path from it, an edge added to it)
//! return `GraphError::VertexNotFound`.
//!
//! # Writing generic code
//!
//! Algorithms in this crate are written against the traits, so they work on every graph type, and
//! you can do the same. Ask for the narrowest trait you use.
//!
//! `Graph::vertices()` and `Graph::edges()` yield [`Graph::VertexRef`](crate::Graph) and
//! `Graph::EdgeRef` items. For a graph that stores its vertices that is a reference; for a graph
//! that computes them (the sparse integer graphs) it is the value. Both implement `Borrow`, so
//! generic code calls `.borrow()`, or uses `vertices_cloned()` to get owned values:
//!
//! ```
//! use jgrapht::graph::sparse::SparseIntDirectedGraph;
//! use jgrapht::*;
//! use std::borrow::Borrow;
//!
//! /// Number of vertices without outgoing edges.
//! fn sinks<G: OutgoingGraph>(g: &G) -> usize {
//!     g.vertices().filter(|v| g.out_degree_of((*v).borrow()) == 0).count()
//! }
//!
//! /// The same with owned vertices.
//! fn sink_list<G: OutgoingGraph>(g: &G) -> Vec<G::Vertex> {
//!     g.vertices_cloned().filter(|v| g.out_degree_of(v) == 0).collect()
//! }
//!
//! let mut base = DiGraph::<&str>::simple();
//! base.add_vertices(["a", "b"]);
//! base.add_edge(&"a", &"b")?;
//! assert_eq!(sinks(&base), 1);
//!
//! let sparse = SparseIntDirectedGraph::new(3, &[(0, 1), (1, 2)])?;
//! assert_eq!(sink_list(&sparse), [2]);
//! # Ok::<(), GraphError>(())
//! ```
//!
//! # Views
//!
//! A view is a graph that wraps another graph and shows it differently, without copying it. A view
//! *borrows* the graph it wraps, so the base cannot change while the view exists and the view can
//! never be stale.
//!
//! | View | Shows |
//! |---|---|
//! | [`AsSubgraph`](crate::AsSubgraph) | a chosen subset of the vertices and edges (with its own sets) |
//! | [`MaskSubgraph`](crate::MaskSubgraph) | the graph without the vertices and edges that two predicates hide |
//! | [`EdgeReversed`](crate::EdgeReversed) | a directed graph with every edge reversed |
//! | [`AsUndirected`](crate::AsUndirected) | a directed graph ignoring direction |
//! | [`AsUnweighted`](crate::AsUnweighted) | the same graph with every weight `1.0` |
//! | [`AsWeighted`](crate::AsWeighted) | weights from a map or function |
//! | [`GraphUnion`](crate::GraphUnion) | the union of two graphs over the same vertex and edge types |
//! | [`Unmodifiable`](crate::Unmodifiable) | the graph, read only |
//!
//! ```
//! use jgrapht::alg::shortestpath::{Dijkstra, ShortestPathAlgorithm};
//! use jgrapht::*;
//!
//! let mut g = DiGraph::<u32>::simple().weighted();
//! g.add_vertices(1..=4);
//! for (u, v, w) in [(1, 2, 1.0), (2, 3, 1.0), (3, 4, 1.0), (1, 4, 10.0)] {
//!     g.add_weighted_edge(&u, &v, w)?;
//! }
//!
//! // The graph without vertex 2: the direct edge is now the only way.
//! let without_2 = MaskSubgraph::new(&g, |v: &u32| *v == 2, |_| false);
//! assert_eq!(without_2.vertex_count(), 3);
//! assert_eq!(Dijkstra::new(&without_2).path_weight(&1, &4)?, Some(10.0));
//!
//! // Every edge reversed: paths now go from 4 back to 1.
//! let reversed = EdgeReversed(&g);
//! assert_eq!(Dijkstra::new(&reversed).path_weight(&4, &1)?, Some(3.0));
//! assert_eq!(Dijkstra::new(&reversed).path_weight(&1, &4)?, None);
//!
//! // Ignore the weights: the direct edge is the shortest path by number of edges.
//! let unit = AsUnweighted(&g);
//! assert_eq!(Dijkstra::new(&unit).path_weight(&1, &4)?, Some(1.0));
//! # Ok::<(), GraphError>(())
//! ```
//!
//! Pass `&mut graph` instead of `&graph` to a view to let it write through to the base graph where
//! the JGraphT view does; the unmodifiable views simply do not implement [`GraphMut`](crate::GraphMut).
//! [`AsSubgraph`](crate::AsSubgraph) copies its vertex and edge *sets* when it is created, so it does
//! not follow later changes of the base.
//!
//! # Listening to changes
//!
//! [`Listenable`](crate::event::Listenable) wraps a graph and reports every successful change to
//! [`GraphListener`](crate::event::GraphListener)s. Rust's borrowing rules replace most uses of
//! listeners in Java (views cannot go stale), so this is for caches and logs you maintain yourself.
//!
//! ```
//! use jgrapht::event::{GraphListener, Listenable};
//! use jgrapht::*;
//! use std::{cell::RefCell, rc::Rc};
//!
//! struct Log(Rc<RefCell<Vec<String>>>);
//! impl GraphListener<u32, DefaultEdge> for Log {
//!     fn vertex_added(&mut self, v: &u32) {
//!         self.0.borrow_mut().push(format!("+{v}"));
//!     }
//!     fn edge_added(&mut self, _: &DefaultEdge, s: &u32, t: &u32) {
//!         self.0.borrow_mut().push(format!("{s}->{t}"));
//!     }
//! }
//!
//! let log = Rc::new(RefCell::new(Vec::new()));
//! let mut g = Listenable::new(DiGraph::<u32>::simple());
//! g.add_listener(Log(log.clone()));
//! g.add_vertex(1);
//! g.add_vertex(2);
//! g.add_edge(&1, &2)?;
//! assert_eq!(*log.borrow(), ["+1", "+2", "1->2"]);
//! # Ok::<(), GraphError>(())
//! ```
//!
//! # Traversals
//!
//! [`traverse`](crate::traverse) has lazy iterators for breadth-first, depth-first, closest-first
//! (Dijkstra order), topological, lexicographic breadth-first, maximum-cardinality, degeneracy and
//! random-walk order. Constructors check their start vertex, so an unknown vertex is an error
//! before the first step; iterators that can fail while running yield `Result` items.
//!
//! ```
//! use jgrapht::traverse::{BreadthFirstIterator, DepthFirstIterator, TopologicalOrderIterator};
//! use jgrapht::*;
//!
//! let mut g = DiGraph::<&str>::simple();
//! g.add_vertices(["shirt", "tie", "jacket", "belt", "trousers"]);
//! for (u, v) in [("shirt", "tie"), ("tie", "jacket"), ("shirt", "belt"), ("trousers", "belt"), ("belt", "jacket")] {
//!     g.add_edge(&u, &v)?;
//! }
//!
//! // Visit what is reachable from "shirt", breadth first.
//! let bfs: Vec<_> = BreadthFirstIterator::from_start(&g, &"shirt")?.collect();
//! assert_eq!(bfs[0], "shirt");
//! assert_eq!(bfs.len(), 4);
//!
//! // Depth first over the whole graph.
//! assert_eq!(DepthFirstIterator::new(&g).count(), 5);
//!
//! // A dressing order: every item after the ones it depends on.
//! let order: Result<Vec<_>> = TopologicalOrderIterator::new(&g)?.collect();
//! let order = order?;
//! let at = |x| order.iter().position(|v| *v == x).unwrap();
//! assert!(at("shirt") < at("tie") && at("tie") < at("jacket") && at("belt") < at("jacket"));
//!
//! // An unknown start vertex is reported up front.
//! assert!(BreadthFirstIterator::from_start(&g, &"socks").is_err());
//! # Ok::<(), GraphError>(())
//! ```
//!
//! A [`TraversalListener`](crate::traverse::TraversalListener) receives the events of a traversal
//! (vertex visited, edge traversed, vertex finished, component started and finished) when you
//! need more than the visiting order.
//!
//! # Algorithms
//!
//! [`alg`](crate::alg) has one module per area. Results are plain values returned by the call
//! (`SpanningTree`, `Coloring`, `Matching`, `MaximumFlow`, `GraphWalk`, ...); nothing has to be
//! "computed" before you can read it.
//!
//! | Area | Module | Examples |
//! |---|---|---|
//! | Shortest paths | [`alg::shortestpath`](crate::alg::shortestpath) | Dijkstra, A*, bidirectional A*, Bellman-Ford, BFS, Floyd-Warshall, Johnson, delta-stepping, Yen, Eppstein, Suurballe, Bhandari, contraction hierarchies, transit node routing, multi-objective |
//! | Connectivity | [`alg::connectivity`](crate::alg::connectivity) | connected, strongly connected and biconnected components, block-cut tree, bridges, articulation points |
//! | Spanning trees | [`alg::spanning`](crate::alg::spanning) | Kruskal, Prim, Boruvka, arborescences, capacitated trees |
//! | Flows and cuts | [`alg::flow`](crate::alg::flow) | Edmonds-Karp, Dinic, push-relabel, Boykov-Kolmogorov, minimum-cost flow, Gomory-Hu trees, Stoer-Wagner |
//! | Matching | [`alg::matching`](crate::alg::matching) | greedy, path growing, Hopcroft-Karp, Kuhn-Munkres, Edmonds, Blossom V |
//! | Colouring | [`alg::color`](crate::alg::color) | greedy, largest-first, smallest-last, DSATUR, Brown, colour refinement |
//! | Cliques and covers | [`alg::clique`](crate::alg::clique), [`alg::cover`](crate::alg::cover), [`alg::independent_set`](crate::alg::independent_set) | Bron-Kerbosch, vertex covers, independent sets |
//! | Cycles | [`alg::cycle`](crate::alg::cycle) | simple cycles, cycle bases, Eulerian cycles, Chinese postman, chordality |
//! | Tours | [`alg::tour`](crate::alg::tour) | TSP heuristics, Held-Karp, Hamiltonian paths and cycles |
//! | Scoring | [`alg::scoring`](crate::alg::scoring) | betweenness, closeness, PageRank, eigenvector, Katz, clustering coefficient |
//! | Structure | [`alg::planar`](crate::alg::planar), [`alg::isomorphism`](crate::alg::isomorphism), [`alg::lca`](crate::alg::lca), [`alg::decomposition`](crate::alg::decomposition) | planarity, isomorphism, lowest common ancestors, tree decompositions |
//! | Other | [`alg::clustering`](crate::alg::clustering), [`alg::linkprediction`](crate::alg::linkprediction), [`alg::drawing`](crate::alg::drawing), [`alg::transform`](crate::alg::transform) | clustering, link prediction, layouts, line graphs |
//!
//! Shortest paths, spanning trees and flows:
//!
//! ```
//! use jgrapht::alg::flow::EdmondsKarpMFImpl;
//! use jgrapht::alg::shortestpath::{BellmanFord, Dijkstra, ShortestPathAlgorithm};
//! use jgrapht::alg::spanning::kruskal;
//! use jgrapht::*;
//!
//! let mut g = DiGraph::<char>::simple().weighted();
//! g.add_vertices(['s', 'a', 'b', 't']);
//! for (u, v, w) in [('s', 'a', 3.0), ('s', 'b', 2.0), ('a', 'b', 1.0), ('a', 't', 2.0), ('b', 't', 3.0)] {
//!     g.add_weighted_edge(&u, &v, w)?;
//! }
//!
//! // The result of a shortest path search is a GraphWalk: vertices, edges and total weight.
//! let walk = Dijkstra::new(&g).path(&'s', &'t')?.unwrap();
//! assert_eq!(walk.weight(), 5.0);
//! assert_eq!(walk.length(), walk.edge_list().len());
//!
//! // Single-source results answer many queries.
//! let paths = BellmanFord::new(&g).single_source(&'s')?;
//! assert_eq!(paths.weight(&'b'), Some(2.0));
//!
//! // The edge weights are capacities in a flow problem.
//! let flow = EdmondsKarpMFImpl::new(&g)?.calculate_maximum_flow(&'s', &'t')?;
//! assert_eq!(flow, 5.0);
//!
//! // Spanning trees treat the graph as undirected.
//! let tree = kruskal(&g);
//! assert_eq!(tree.edges.len(), 3);
//! # Ok::<(), GraphError>(())
//! ```
//!
//! Colouring, matching and components:
//!
//! ```
//! use jgrapht::alg::color::greedy_coloring;
//! use jgrapht::alg::connectivity::{connected_components, strongly_connected_components};
//! use jgrapht::*;
//!
//! let mut g = UGraph::<u32>::simple();
//! g.add_vertices(0..6);
//! for (u, v) in [(0, 1), (1, 2), (2, 0), (3, 4)] {
//!     g.add_edge(&u, &v)?;
//! }
//! assert_eq!(connected_components(&g).len(), 3); // {0,1,2}, {3,4}, {5}
//! let colors = greedy_coloring(&g);
//! assert_eq!(colors.count, 3); // the triangle needs three colours
//!
//! let mut d = DiGraph::<u32>::simple();
//! d.add_vertices(0..4);
//! for (u, v) in [(0, 1), (1, 0), (1, 2), (2, 3), (3, 2)] {
//!     d.add_edge(&u, &v)?;
//! }
//! assert_eq!(strongly_connected_components(&d).len(), 2);
//! # Ok::<(), GraphError>(())
//! ```
//!
//! The module documentation of each area lists its algorithms and says which graph capabilities
//! they need.
//!
//! # Generators
//!
//! [`generate`](crate::generate) builds well-known graphs into an existing graph: complete,
//! complete bipartite, empty, linear, ring, star, wheel, grid, hypercube, generalized Petersen,
//! windmill, named graphs, and random models (Erdős-Rényi G(n,p) and G(n,m), Barabási-Albert,
//! Watts-Strogatz, scale-free, random regular, Prüfer trees). Every generator implements
//! [`GraphGenerator`](crate::generate::GraphGenerator).
//!
//! ```
//! use jgrapht::generate::{Complete, GraphGenerator, Ring};
//! use jgrapht::*;
//!
//! let mut g = UGraph::<u32>::simple().with_vertex_factory(|i| i as u32);
//! Complete(5).generate_default(&mut g)?;
//! assert_eq!((g.vertex_count(), g.edge_count()), (5, 10));
//!
//! let mut ring = UGraph::<u32>::simple().with_vertex_factory(|i| i as u32);
//! Ring(6).generate_default(&mut ring)?;
//! assert_eq!(ring.edge_count(), 6);
//! # Ok::<(), GraphError>(())
//! ```
//!
//! The generators take the vertices from the vertex factory of the target graph. Random generators
//! take any [`rand::RngCore`](https://docs.rs/rand), so a seeded generator is reproducible.
//!
//! # Import and export
//!
//! The companion crate `jgrapht-io` reads and writes DIMACS, CSV (edge list, adjacency list,
//! matrix), graph6, sparse6, digraph6, JSON, GML, DOT, GraphML, GEXF, Lemon, TSPLIB and matrix
//! formats. Importers and exporters report malformed input and failed writes as `IoError`.
//!
//! # Serialization
//!
//! With the `serde` feature, graphs, sparse graphs, views, walks and result types implement
//! `Serialize` and `Deserialize` for any serde format. Reading goes through the normal mutation
//! rules, so a document that breaks the graph type (a loop in a simple graph, a cycle in a DAG,
//! the wrong direction) is an error. Function pointers (edge and vertex factories) are not written;
//! add edges to a deserialized graph with `add_edge_with`, or restore the default factory with
//! `with_default_edge_factory()`.
//!
//! # Randomness
//!
//! Randomised algorithms (random generators, random walks, randomised colouring and matching
//! heuristics) take any `rand::RngCore`; use `StdRng::seed_from_u64(7)` for reproducible runs.
//! Results for a given seed are valid but are *not* the same as JGraphT's for the same seed.
//!
//! # Errors
//!
//! See the crate-level documentation: unreachable is `None`, failure is `Err(GraphError)` with a typed
//! variant you can match, and graph capabilities are trait bounds, not run-time errors.
//!
//! # Performance
//!
//! * [`BaseGraph`](crate::BaseGraph) (`UGraph`, `DiGraph`) keeps vertices and edges in
//!   insertion-ordered hash maps. It is the right default: any vertex type, edges can be removed.
//!   Looking up the endpoints or weight of an edge costs a hash lookup.
//! * [`graph::sparse`](crate::graph::sparse) stores integer-vertex graphs in compressed sparse rows:
//!   24 bytes per edge for `NoIncoming` and 32 for `FullIncoming`, endpoints are array reads, and
//!   the graphs are immutable. Pick the incoming-edge mode your algorithms need; passing a
//!   `NoIncoming` graph to an algorithm that needs incoming edges does not compile.
//! * [`graph::map`](crate::graph::map) has map-backed graphs for integer vertices.
//! * In loops, prefer `edges()` and the incidence iterators over cloning; use `vertices_cloned()`
//!   only when you need owned values.
//! * `NeighborCache` ([`alg::util`](crate::alg::util)) caches neighbour sets for algorithms that ask
//!   for them repeatedly.
//! * Benchmarks live in `crates/jgrapht/benches`; run them with `cargo bench-all` or
//!   `scripts/bench.sh` (see the repository README).
//!
//! # Coming from JGraphT
//!
//! | JGraphT | jgrapht-rs |
//! |---|---|
//! | exceptions and `null` | `Result` and `Option` |
//! | `Double.POSITIVE_INFINITY` for "no path" | `None` |
//! | `getX()` valid only after `compute()` | the computation returns the result object |
//! | `UnsupportedOperationException` on some graphs | the capability trait is not implemented, so the call does not compile |
//! | `Graph<V, E>` interface | [`Graph`](crate::Graph), [`GraphMut`](crate::GraphMut) and the capability traits |
//! | `SimpleGraph`, `Multigraph`, `Pseudograph`, ... | [`UGraph`](crate::UGraph) and [`DiGraph`](crate::DiGraph) with `simple()`, `multi()`, `pseudo()`, ... |
//! | `Graphs` utility class | [`GraphExt`](crate::GraphExt), [`DirectedExt`](crate::DirectedExt), [`GraphMutExt`](crate::GraphMutExt) |
//! | `GraphTests` | [`graph::props`](crate::graph::props) |
//! | `GraphWalk` / `GraphPath` | [`GraphWalk`](crate::GraphWalk) |
//! | `AsSubgraph`, `EdgeReversedGraph`, ... | the view types, which borrow their base |
//! | `ListenableGraph` | [`event::Listenable`](crate::event::Listenable) |
//! | `java.io.Serializable` | the `serde` feature |
//! | `HashSet` iteration order in ties | insertion order |
//! | `Random` with a seed reproduces output | any `RngCore`; a seed gives a valid but different result |
//! | live views and listener-driven caches | borrowing views and explicit snapshots |
