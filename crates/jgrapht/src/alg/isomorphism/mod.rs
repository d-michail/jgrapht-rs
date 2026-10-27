//! Graph, subgraph and tree isomorphism.
//!
//! The JGraphT inspectors (`VF2GraphIsomorphismInspector`, the AHU tree inspectors, ...).

mod ahu;
mod mapping;
mod ordering;
mod refinement;
mod vf2;

pub use ahu::{
    AHUForestIsomorphismInspector, AHURootedTreeIsomorphismInspector,
    AHUUnrootedTreeIsomorphismInspector,
};
pub use mapping::{IsomorphicGraphMapping, NULL_NODE};
pub use ordering::GraphOrdering;
pub use refinement::{ColorRefinementIsomorphismInspector, IsomorphismUndecidable};
pub use vf2::{
    EdgeComparator, VF2GraphIsomorphismInspector, VF2MappingIterator,
    VF2SubgraphIsomorphismInspector, VertexComparator,
};

/// `IsomorphismInspector`: enumerates mappings from the first graph to the second.
pub trait IsomorphismInspector<'g, G: crate::graph::Graph + 'g> {
    /// All mappings found (lazily for VF2, at most one for the other inspectors).
    fn mappings(&self) -> Box<dyn Iterator<Item = IsomorphicGraphMapping<'g, G>> + '_>;
    /// Whether at least one mapping exists.
    fn isomorphism_exists(&self) -> bool {
        self.mappings().next().is_some()
    }
}
