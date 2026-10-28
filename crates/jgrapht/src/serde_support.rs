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

//! Helpers for the optional `serde` feature.
//!
//! Maps keyed by vertices or edges are written as sequences of `(key, value)` pairs, not as
//! serde maps: JSON and some other formats only allow string keys, while vertices and edges can be
//! any type. A sequence of pairs round-trips in every format.

use indexmap::IndexMap;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::HashMap;
use std::hash::Hash;

/// `#[serde(with = "...")]` for a [`HashMap`] field.
pub(crate) mod hash_pairs {
    use super::*;

    pub(crate) fn serialize<S, K, V>(map: &HashMap<K, V>, s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
        K: Serialize,
        V: Serialize,
    {
        s.collect_seq(map.iter())
    }

    pub(crate) fn deserialize<'de, D, K, V>(d: D) -> Result<HashMap<K, V>, D::Error>
    where
        D: Deserializer<'de>,
        K: Deserialize<'de> + Eq + Hash,
        V: Deserialize<'de>,
    {
        Ok(Vec::<(K, V)>::deserialize(d)?.into_iter().collect())
    }
}

/// `#[serde(with = "...")]` for an [`IndexMap`] field (the order is kept).
pub(crate) mod index_pairs {
    use super::*;

    pub(crate) fn serialize<S, K, V>(map: &IndexMap<K, V>, s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
        K: Serialize,
        V: Serialize,
    {
        s.collect_seq(map.iter())
    }

    pub(crate) fn deserialize<'de, D, K, V>(d: D) -> Result<IndexMap<K, V>, D::Error>
    where
        D: Deserializer<'de>,
        K: Deserialize<'de> + Eq + Hash,
        V: Deserialize<'de>,
    {
        Ok(Vec::<(K, V)>::deserialize(d)?.into_iter().collect())
    }
}
