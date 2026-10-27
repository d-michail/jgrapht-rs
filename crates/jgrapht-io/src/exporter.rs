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

//! Shared identifier and attribute-provider support for graph exporters.

#![deny(missing_docs)]

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
    /// Creates a provider whose first identifier is `"1"`.
    pub fn new() -> Self {
        Self::with_start(1)
    }

    /// Creates a provider whose first identifier is the decimal form of `next_id`.
    pub fn with_start(next_id: i64) -> Self {
        Self {
            next_id,
            ids: HashMap::new(),
        }
    }
}

impl<T: Clone + Eq + Hash> IntegerIdProvider<T> {
    /// Returns the stable identifier assigned to `value`, allocating one on first use.
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
    /// Creates shared exporter state with the required vertex identifier provider.
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

    /// Sets the provider used to identify the graph itself.
    pub fn set_graph_id_provider(&mut self, provider: impl FnMut() -> String + 'static) {
        self.graph_id_provider = Some(Box::new(provider));
    }

    /// Returns whether a graph identifier provider is configured.
    pub fn has_graph_id_provider(&self) -> bool {
        self.graph_id_provider.is_some()
    }

    /// Removes the graph identifier provider.
    pub fn clear_graph_id_provider(&mut self) {
        self.graph_id_provider = None;
    }

    /// Sets the provider for graph-level attributes.
    pub fn set_graph_attribute_provider(&mut self, provider: impl Fn() -> AttributeMap + 'static) {
        self.graph_attribute_provider = Some(Box::new(provider));
    }

    /// Returns whether a graph attribute provider is configured.
    pub fn has_graph_attribute_provider(&self) -> bool {
        self.graph_attribute_provider.is_some()
    }

    /// Removes the graph attribute provider.
    pub fn clear_graph_attribute_provider(&mut self) {
        self.graph_attribute_provider = None;
    }

    /// Replaces the required vertex identifier provider.
    pub fn set_vertex_id_provider(&mut self, provider: impl FnMut(&V) -> String + 'static) {
        self.vertex_id_provider = Box::new(provider);
    }

    /// Sets the provider for per-vertex attributes.
    pub fn set_vertex_attribute_provider(
        &mut self,
        provider: impl Fn(&V) -> AttributeMap + 'static,
    ) {
        self.vertex_attribute_provider = Some(Box::new(provider));
    }

    /// Returns whether a vertex attribute provider is configured.
    pub fn has_vertex_attribute_provider(&self) -> bool {
        self.vertex_attribute_provider.is_some()
    }

    /// Removes the vertex attribute provider.
    pub fn clear_vertex_attribute_provider(&mut self) {
        self.vertex_attribute_provider = None;
    }

    /// Sets the optional edge identifier provider.
    pub fn set_edge_id_provider(&mut self, provider: impl FnMut(&E) -> String + 'static) {
        self.edge_id_provider = Some(Box::new(provider));
    }

    /// Returns whether an edge identifier provider is configured.
    pub fn has_edge_id_provider(&self) -> bool {
        self.edge_id_provider.is_some()
    }

    /// Removes the edge identifier provider.
    pub fn clear_edge_id_provider(&mut self) {
        self.edge_id_provider = None;
    }

    /// Sets the provider for per-edge attributes.
    pub fn set_edge_attribute_provider(&mut self, provider: impl Fn(&E) -> AttributeMap + 'static) {
        self.edge_attribute_provider = Some(Box::new(provider));
    }

    /// Returns whether an edge attribute provider is configured.
    pub fn has_edge_attribute_provider(&self) -> bool {
        self.edge_attribute_provider.is_some()
    }

    /// Removes the edge attribute provider.
    pub fn clear_edge_attribute_provider(&mut self) {
        self.edge_attribute_provider = None;
    }

    /// Invokes the graph identifier provider, or returns `None` when none is configured.
    pub fn graph_id(&mut self) -> Option<String> {
        self.graph_id_provider.as_mut().map(|provider| provider())
    }

    /// Invokes the graph attribute provider, or returns `None` when none is configured.
    pub fn graph_attributes(&self) -> Option<AttributeMap> {
        self.graph_attribute_provider
            .as_ref()
            .map(|provider| provider())
    }

    /// Returns one graph attribute by key from a freshly provided attribute map.
    pub fn graph_attribute(&self, key: &str) -> Option<crate::Attribute> {
        self.graph_attributes()?.shift_remove(key)
    }

    /// Returns the identifier for `vertex`.
    ///
    /// # Errors
    ///
    /// Returns [`IoError::Export`] if the provider returns an empty identifier.
    pub fn vertex_id(&mut self, vertex: &V) -> Result<String> {
        let id = (self.vertex_id_provider)(vertex);
        if id.is_empty() {
            Err(IoError::Export("vertex identifier cannot be empty".into()))
        } else {
            Ok(id)
        }
    }

    /// Invokes the vertex attribute provider for `vertex`, if configured.
    pub fn vertex_attributes(&self, vertex: &V) -> Option<AttributeMap> {
        self.vertex_attribute_provider
            .as_ref()
            .map(|provider| provider(vertex))
    }

    /// Returns one attribute for `vertex` by key from a freshly provided attribute map.
    pub fn vertex_attribute(&self, vertex: &V, key: &str) -> Option<crate::Attribute> {
        self.vertex_attributes(vertex)?.shift_remove(key)
    }

    /// Invokes the edge identifier provider for `edge`, if configured.
    pub fn edge_id(&mut self, edge: &E) -> Option<String> {
        self.edge_id_provider
            .as_mut()
            .map(|provider| provider(edge))
    }

    /// Invokes the edge attribute provider for `edge`, if configured.
    pub fn edge_attributes(&self, edge: &E) -> Option<AttributeMap> {
        self.edge_attribute_provider
            .as_ref()
            .map(|provider| provider(edge))
    }

    /// Returns one attribute for `edge` by key from a freshly provided attribute map.
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
