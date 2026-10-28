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

//! Graph, subgraph and tree isomorphism.
//!
//! The JGraphT inspectors (`VF2GraphIsomorphismInspector`, the AHU tree inspectors, ...).

use crate::graph::IncidenceGraph;

mod ahu;
mod mapping;
mod ordering;
mod refinement;
mod vf2;

pub use ahu::{
    AhuForestIsomorphismInspector, AhuRootedTreeIsomorphismInspector,
    AhuUnrootedTreeIsomorphismInspector,
};
pub use mapping::{IsomorphicGraphMapping, NULL_NODE};
pub use ordering::GraphOrdering;
pub use refinement::{ColorRefinementIsomorphismInspector, IsomorphismUndecidable};
pub use vf2::{
    EdgeComparator, VF2GraphIsomorphismInspector, VF2SubgraphIsomorphismInspector,
    VertexComparator, Vf2MappingIterator,
};

/// General interface for graph and subgraph isomorphism.
///
/// # Rust notes
///
/// `IsomorphismInspector`: enumerates mappings from the first graph to the second.
pub trait IsomorphismInspector<'g, G: crate::graph::Graph + 'g + IncidenceGraph> {
    /// All mappings found (lazily for VF2, at most one for the other inspectors).
    fn mappings(&self) -> Box<dyn Iterator<Item = IsomorphicGraphMapping<'g, G>> + '_>;
    /// Whether at least one mapping exists.
    fn isomorphism_exists(&self) -> bool {
        self.mappings().next().is_some()
    }
}
