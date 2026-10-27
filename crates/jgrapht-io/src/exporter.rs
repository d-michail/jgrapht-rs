use crate::{AttributeMap, IoError, Result};
use std::collections::HashMap;
use std::hash::Hash;

/// Assigns stable, sequential string identifiers in first-seen order.
#[derive(Debug, Clone)]
pub struct IntegerIdProvider<T> {
    next_id: i64,
    ids: HashMap<T, i64>,
}

impl<T> Default for IntegerIdProvider<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> IntegerIdProvider<T> {
    pub fn new() -> Self {
        Self::with_start(1)
    }

    pub fn with_start(next_id: i64) -> Self {
        Self {
            next_id,
            ids: HashMap::new(),
        }
    }
}

impl<T: Clone + Eq + Hash> IntegerIdProvider<T> {
    pub fn id(&mut self, value: &T) -> String {
        let id = self.ids.entry(value.clone()).or_insert_with(|| {
            let id = self.next_id;
            self.next_id += 1;
            id
        });
        id.to_string()
    }
}

type GraphIdProvider = Box<dyn FnMut() -> String>;
type GraphAttributeProvider = Box<dyn Fn() -> AttributeMap>;
type IdProvider<T> = Box<dyn FnMut(&T) -> String>;
type AttributeProvider<T> = Box<dyn Fn(&T) -> AttributeMap>;

/// Shared provider configuration used by text and XML exporters.
pub struct BaseExporter<V, E> {
    graph_id_provider: Option<GraphIdProvider>,
    graph_attribute_provider: Option<GraphAttributeProvider>,
    vertex_id_provider: IdProvider<V>,
    vertex_attribute_provider: Option<AttributeProvider<V>>,
    edge_id_provider: Option<IdProvider<E>>,
    edge_attribute_provider: Option<AttributeProvider<E>>,
}

impl<V, E> BaseExporter<V, E> {
    pub fn new(provider: impl FnMut(&V) -> String + 'static) -> Self {
        Self {
            graph_id_provider: None,
            graph_attribute_provider: None,
            vertex_id_provider: Box::new(provider),
            vertex_attribute_provider: None,
            edge_id_provider: None,
            edge_attribute_provider: None,
        }
    }

    pub fn set_graph_id_provider(&mut self, provider: impl FnMut() -> String + 'static) {
        self.graph_id_provider = Some(Box::new(provider));
    }

    pub fn has_graph_id_provider(&self) -> bool {
        self.graph_id_provider.is_some()
    }

    pub fn clear_graph_id_provider(&mut self) {
        self.graph_id_provider = None;
    }

    pub fn set_graph_attribute_provider(&mut self, provider: impl Fn() -> AttributeMap + 'static) {
        self.graph_attribute_provider = Some(Box::new(provider));
    }

    pub fn has_graph_attribute_provider(&self) -> bool {
        self.graph_attribute_provider.is_some()
    }

    pub fn clear_graph_attribute_provider(&mut self) {
        self.graph_attribute_provider = None;
    }

    pub fn set_vertex_id_provider(&mut self, provider: impl FnMut(&V) -> String + 'static) {
        self.vertex_id_provider = Box::new(provider);
    }

    pub fn set_vertex_attribute_provider(
        &mut self,
        provider: impl Fn(&V) -> AttributeMap + 'static,
    ) {
        self.vertex_attribute_provider = Some(Box::new(provider));
    }

    pub fn has_vertex_attribute_provider(&self) -> bool {
        self.vertex_attribute_provider.is_some()
    }

    pub fn clear_vertex_attribute_provider(&mut self) {
        self.vertex_attribute_provider = None;
    }

    pub fn set_edge_id_provider(&mut self, provider: impl FnMut(&E) -> String + 'static) {
        self.edge_id_provider = Some(Box::new(provider));
    }

    pub fn has_edge_id_provider(&self) -> bool {
        self.edge_id_provider.is_some()
    }

    pub fn clear_edge_id_provider(&mut self) {
        self.edge_id_provider = None;
    }

    pub fn set_edge_attribute_provider(&mut self, provider: impl Fn(&E) -> AttributeMap + 'static) {
        self.edge_attribute_provider = Some(Box::new(provider));
    }

    pub fn has_edge_attribute_provider(&self) -> bool {
        self.edge_attribute_provider.is_some()
    }

    pub fn clear_edge_attribute_provider(&mut self) {
        self.edge_attribute_provider = None;
    }

    pub fn graph_id(&mut self) -> Option<String> {
        self.graph_id_provider.as_mut().map(|provider| provider())
    }

    pub fn graph_attributes(&self) -> Option<AttributeMap> {
        self.graph_attribute_provider
            .as_ref()
            .map(|provider| provider())
    }

    pub fn graph_attribute(&self, key: &str) -> Option<crate::Attribute> {
        self.graph_attributes()?.shift_remove(key)
    }

    pub fn vertex_id(&mut self, vertex: &V) -> Result<String> {
        let id = (self.vertex_id_provider)(vertex);
        if id.is_empty() {
            Err(IoError::Export("vertex identifier cannot be empty".into()))
        } else {
            Ok(id)
        }
    }

    pub fn vertex_attributes(&self, vertex: &V) -> Option<AttributeMap> {
        self.vertex_attribute_provider
            .as_ref()
            .map(|provider| provider(vertex))
    }

    pub fn vertex_attribute(&self, vertex: &V, key: &str) -> Option<crate::Attribute> {
        self.vertex_attributes(vertex)?.shift_remove(key)
    }

    pub fn edge_id(&mut self, edge: &E) -> Option<String> {
        self.edge_id_provider
            .as_mut()
            .map(|provider| provider(edge))
    }

    pub fn edge_attributes(&self, edge: &E) -> Option<AttributeMap> {
        self.edge_attribute_provider
            .as_ref()
            .map(|provider| provider(edge))
    }

    pub fn edge_attribute(&self, edge: &E, key: &str) -> Option<crate::Attribute> {
        self.edge_attributes(edge)?.shift_remove(key)
    }
}

/// Java's `Double.toString` differs from Rust for integral finite values.
pub(crate) fn format_f64(value: f64) -> String {
    if value.is_nan() {
        return "NaN".into();
    }
    if value == f64::INFINITY {
        return "Infinity".into();
    }
    if value == f64::NEG_INFINITY {
        return "-Infinity".into();
    }
    let mut text = value.to_string();
    if value.is_finite() && value.fract() == 0.0 && !text.contains(['e', 'E']) {
        text.push_str(".0");
    }
    text
}
