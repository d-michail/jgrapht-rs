#![allow(
    clippy::needless_range_loop,
    clippy::collapsible_if,
    clippy::type_complexity,
    clippy::manual_is_multiple_of,
    clippy::too_many_arguments,
    clippy::nonminimal_bool
)]
//! A Rust port of the [JGraphT](https://jgrapht.org) core library.
//!
//! Unlike the Java original, graphs are expressed as *traits*
//! ([`Graph`], [`DirectedGraph`], [`UndirectedGraph`], [`GraphMut`]) so that
//! algorithms are written against capabilities and views (subgraphs, reversed
//! graphs, ...) are plain wrapper types. Vertices and edges are user supplied
//! values (`Clone + Eq + Hash`).

pub mod alg;
pub mod error;
pub mod event;
pub mod generate;
pub mod graph;
pub mod graph_type;
pub mod traverse;
pub mod util;
pub mod view;
pub mod walk;

pub use error::{GraphError, Result};
pub use graph::{
    BaseGraph, DefaultEdge, DiGraph, DirectedGraph, Graph, GraphExt, GraphMut, GraphMutExt, UGraph,
    UndirectedGraph, graph_equals, graph_hash,
};
pub use graph_type::GraphType;
pub use view::{
    AsSubgraph, AsUndirected, AsUnweighted, AsWeighted, EdgeReversed, GraphUnion, MaskSubgraph,
    Unmodifiable,
};
pub use walk::{GraphMapping, GraphWalk};
