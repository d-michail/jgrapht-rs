// (C) Copyright 2003-2026, by Barak Naveh and JGraphT Contributors.
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

//! Typed attributes shared by all import and export formats.

#![deny(missing_docs)]

use indexmap::IndexMap;
use std::fmt;
use std::str::FromStr;

/// The declared type of an imported or exported attribute.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum AttributeType {
    /// Explicit null value.
    Null,
    /// Boolean value.
    Boolean,
    /// 32-bit integer value.
    Int,
    /// 64-bit integer value.
    Long,
    /// Single-precision floating-point value.
    Float,
    /// Double-precision floating-point value.
    Double,
    /// Plain string value.
    String,
    /// HTML-formatted string value.
    Html,
    /// Type was not declared or recognized by the source format.
    #[default]
    Unknown,
    /// Identifier value, distinct from display text in formats that support it.
    Identifier,
}

impl AttributeType {
    /// Returns the canonical lowercase spelling used by the text formats.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Null => "null",
            Self::Boolean => "boolean",
            Self::Int => "int",
            Self::Long => "long",
            Self::Float => "float",
            Self::Double => "double",
            Self::String => "string",
            Self::Html => "html",
            Self::Unknown => "unknown",
            Self::Identifier => "identifier",
        }
    }
}

impl fmt::Display for AttributeType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for AttributeType {
    type Err = String;

    fn from_str(value: &str) -> std::result::Result<Self, Self::Err> {
        Ok(match value {
            "null" => Self::Null,
            "boolean" => Self::Boolean,
            "int" => Self::Int,
            "long" => Self::Long,
            "float" => Self::Float,
            "double" => Self::Double,
            "string" => Self::String,
            "html" => Self::Html,
            "unknown" => Self::Unknown,
            "identifier" => Self::Identifier,
            _ => return Err(format!("attribute type {value} is unknown")),
        })
    }
}

/// An attribute value together with its declared type.
///
/// The lexical value is retained instead of normalising it so that importers can
/// round-trip files without losing representation details.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Attribute {
    value: String,
    attribute_type: AttributeType,
}

impl Attribute {
    /// Creates an attribute from its lexical value and declared type.
    pub fn new(value: impl Into<String>, attribute_type: AttributeType) -> Self {
        Self {
            value: value.into(),
            attribute_type,
        }
    }

    /// Creates an explicit null attribute.
    pub fn null() -> Self {
        Self::new("null", AttributeType::Null)
    }

    /// Returns the attribute's original lexical value.
    pub fn value(&self) -> &str {
        &self.value
    }

    /// The declared type of the attribute (`string`, `int`, ...); the value itself stays a string.
    pub const fn attribute_type(&self) -> AttributeType {
        self.attribute_type
    }
}

impl fmt::Display for Attribute {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.value)
    }
}

impl From<bool> for Attribute {
    fn from(value: bool) -> Self {
        Self::new(value.to_string(), AttributeType::Boolean)
    }
}

macro_rules! numeric_attribute {
    ($ty:ty, $kind:expr) => {
        impl From<$ty> for Attribute {
            fn from(value: $ty) -> Self {
                Self::new(value.to_string(), $kind)
            }
        }
    };
}

numeric_attribute!(i32, AttributeType::Int);
numeric_attribute!(i64, AttributeType::Long);
numeric_attribute!(f32, AttributeType::Float);
numeric_attribute!(f64, AttributeType::Double);

impl From<String> for Attribute {
    fn from(value: String) -> Self {
        Self::new(value, AttributeType::String)
    }
}

impl From<&str> for Attribute {
    fn from(value: &str) -> Self {
        Self::new(value, AttributeType::String)
    }
}

/// Compatibility name corresponding to JGraphT's concrete attribute class.
pub type DefaultAttribute = Attribute;

/// Insertion-ordered attribute collection used by all formats.
pub type AttributeMap = IndexMap<String, Attribute>;
