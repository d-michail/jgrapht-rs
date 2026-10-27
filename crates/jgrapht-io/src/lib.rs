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

//! Graph import and export support for [`jgrapht`].
//!
//! Format implementations will live in modules such as `dimacs`, `dot`, and
//! `graphml`. Keeping them in this companion crate lets graph algorithms and
//! storage types remain independent of parser dependencies.

#![forbid(unsafe_code)]

mod attribute;
pub mod csv;
pub mod dimacs;
pub mod dot;
mod error;
mod event;
mod exporter;
pub mod gexf;
pub mod gml;
pub mod graph6;
pub mod graphml;
pub mod json;
pub mod lemon;
pub mod matrix;
mod traits;
pub mod tsplib;

pub use attribute::{Attribute, AttributeMap, AttributeType, DefaultAttribute};
pub use error::{IoError, Result};
pub use event::ImportEvent;
pub use exporter::{BaseExporter, IntegerIdProvider};
pub use traits::{EventDrivenImporter, GraphExporter, GraphImporter};
