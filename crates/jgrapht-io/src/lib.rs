//! Graph import and export support for [`jgrapht`].
//!
//! Format implementations will live in modules such as `dimacs`, `dot`, and
//! `graphml`. Keeping them in this companion crate lets graph algorithms and
//! storage types remain independent of parser dependencies.

#![forbid(unsafe_code)]
