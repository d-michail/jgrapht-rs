//! Graph import and export support for [`jgrapht`].
//!
//! Format implementations will live in modules such as `dimacs`, `dot`, and
//! `graphml`. Keeping them in this companion crate lets graph algorithms and
//! storage types remain independent of parser dependencies.

#![forbid(unsafe_code)]

mod attribute;
pub mod csv;
pub mod dimacs;
mod error;
mod event;
mod exporter;
pub mod gml;
pub mod graph6;
pub mod graphml;
pub mod json;
pub mod lemon;
pub mod matrix;
mod traits;

pub use attribute::{Attribute, AttributeMap, AttributeType, DefaultAttribute};
pub use error::{IoError, Result};
pub use event::ImportEvent;
pub use exporter::{BaseExporter, IntegerIdProvider};
pub use traits::{EventDrivenImporter, GraphExporter, GraphImporter};
