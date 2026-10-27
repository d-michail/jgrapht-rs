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

//! TSPLIB95 coordinate-instance and tour import support.
//!
//! The importer reads `NODE_COORD_SECTION` data into a complete weighted graph
//! and optionally maps a `TOUR_SECTION` to the created vertices. It supports
//! the coordinate distance types `EUC_2D`, `EUC_3D`, `MAX_2D`, `MAX_3D`,
//! `MAN_2D`, `MAN_3D`, `CEIL_2D`, `GEO`, and `ATT`. Other TSPLIB95 data
//! sections and explicit edge-weight matrices are not imported.

#![deny(missing_docs)]

use jgrapht::graph::IncidenceGraph;
use crate::{GraphImporter, IoError, Result};
use indexmap::IndexMap;
use jgrapht::{Graph, GraphMut};
use std::collections::{HashMap, HashSet};
use std::hash::Hash;
use std::io::Read;

const VALID_TYPES: &[&str] = &["TSP", "ATSP", "SOP", "HCP", "CVRP", "TOUR"];
const VALID_EDGE_WEIGHT_TYPES: &[&str] = &[
    "EXPLICIT", "EUC_2D", "EUC_3D", "MAX_2D", "MAX_3D", "MAN_2D", "MAN_3D", "CEIL_2D", "GEO",
    "ATT", "XRAY1", "XRAY2", "SPECIAL",
];
const VALID_EDGE_WEIGHT_FORMATS: &[&str] = &[
    "FUNCTION",
    "FULL_MATRIX",
    "UPPER_ROW",
    "LOWER_ROW",
    "UPPER_DIAG_ROW",
    "LOWER_DIAG_ROW",
    "UPPER_COL",
    "LOWER_COL",
    "UPPER_DIAG_COL",
    "LOWER_DIAG_COL",
];
const VALID_EDGE_DATA_FORMATS: &[&str] = &["EDGE_LIST", "ADJ_LIST"];
const VALID_NODE_COORD_TYPES: &[&str] = &["TWOD_COORDS", "THREED_COORDS", "NO_COORDS"];
const VALID_DISPLAY_DATA_TYPES: &[&str] = &["COORD_DISPLAY", "TWOD_DISPLAY", "NO_DISPLAY"];

#[derive(Debug, Clone, PartialEq)]
/// A numbered node read from a TSPLIB95 `NODE_COORD_SECTION`.
pub struct TsplibNode {
    /// The node number specified by the source file.
    pub number: i32,
    /// The node's two- or three-dimensional coordinates.
    pub coordinates: Vec<f64>,
}

#[derive(Debug, Clone)]
/// Metadata collected from an imported TSPLIB95 file.
///
/// The structure contains the supported specification fields, the mapping
/// between generated graph vertices and source nodes, and any imported tour.
pub struct TsplibMetadata<V> {
    /// The value of the `NAME` field.
    pub name: Option<String>,
    /// First comment, retained for compatibility with the initial Rust API.
    pub comment: Option<String>,
    /// All values of the repeatable `COMMENT` field, in source order.
    pub comments: Vec<String>,
    /// The value of the `TYPE` field.
    pub problem_type: Option<String>,
    /// The value of `EDGE_WEIGHT_TYPE`, or an empty string when absent.
    pub edge_weight_type: String,
    /// The value of `DIMENSION`, or zero when absent.
    pub dimension: usize,
    /// The value of the `CAPACITY` field.
    pub capacity: Option<i32>,
    /// The value of the `EDGE_WEIGHT_FORMAT` field.
    pub edge_weight_format: Option<String>,
    /// The value of the `EDGE_DATA_FORMAT` field.
    pub edge_data_format: Option<String>,
    /// The value of the `NODE_COORD_TYPE` field.
    pub node_coord_type: Option<String>,
    /// The value of the `DISPLAY_DATA_TYPE` field.
    pub display_data_type: Option<String>,
    /// Imported graph vertices mapped to their source nodes.
    pub nodes: IndexMap<V, TsplibNode>,
    /// The imported tour, when the source contained a `TOUR_SECTION`.
    pub tour: Option<Vec<V>>,
    distinct_node_locations: bool,
    distinct_neighbor_distances: bool,
}

impl<V> TsplibMetadata<V> {
    /// Returns whether all imported nodes have different coordinates.
    pub fn has_distinct_node_locations(&self) -> bool {
        self.distinct_node_locations
    }

    /// Returns whether each vertex's incident edges have distinct weights.
    ///
    /// A true value means that no two other imported locations have the same
    /// distance from any one location.
    pub fn has_distinct_neighbor_distances(&self) -> bool {
        self.distinct_neighbor_distances
    }
}

#[derive(Default)]
struct Specification {
    name: Option<String>,
    comments: Vec<String>,
    problem_type: Option<String>,
    dimension: Option<usize>,
    capacity: Option<i32>,
    edge_weight_type: Option<String>,
    edge_weight_format: Option<String>,
    edge_data_format: Option<String>,
    node_coord_type: Option<String>,
    display_data_type: Option<String>,
}

struct ParsedFile {
    specification: Specification,
    nodes: Option<Vec<TsplibNode>>,
    tour_numbers: Option<Vec<i32>>,
}

/// Imports coordinate-based TSPLIB95 instances and tours.
///
/// Graph imports require a weighted destination graph. A
/// `NODE_COORD_SECTION` creates one supplied vertex per node and a weighted
/// edge for every pair of imported vertices; a directed graph receives both
/// orientations. An empty destination graph is recommended.
///
/// Metadata for the most recently completed import is available through
/// [`Self::metadata`]. Starting an import clears the previous metadata, and a
/// failed import leaves it empty.
pub struct TsplibImporter<V> {
    metadata: Option<TsplibMetadata<V>>,
}

impl<V> TsplibImporter<V> {
    /// Creates a TSPLIB95 importer with no import metadata.
    pub fn new() -> Self {
        Self { metadata: None }
    }

    /// Returns metadata for the most recently completed import.
    pub fn metadata(&self) -> Option<&TsplibMetadata<V>> {
        self.metadata.as_ref()
    }

    /// Imports a tour using the node-number mapping from a prior graph import.
    ///
    /// The source should contain a `TOUR_SECTION`. Each listed node number is
    /// resolved through `reference`; an unknown number is an error. If no tour
    /// section is present, this returns `None`. Metadata parsed from the tour
    /// file becomes the importer's current metadata while the reference node
    /// mapping and distinctness results are retained.
    pub fn import_tour<R: Read>(
        &mut self,
        reference: &TsplibMetadata<V>,
        mut input: R,
    ) -> Result<Option<Vec<V>>>
    where
        V: Clone + Eq + Hash,
    {
        self.metadata = None;
        let mut text = String::new();
        input.read_to_string(&mut text)?;
        let parsed = parse_file(&text)?;
        let by_number = number_mapping(&reference.nodes);
        let tour = parsed
            .tour_numbers
            .map(|numbers| map_tour(numbers, &by_number))
            .transpose()?;
        let metadata = metadata_from_parts(
            parsed.specification,
            reference.nodes.clone(),
            tour.clone(),
            reference.distinct_node_locations,
            reference.distinct_neighbor_distances,
        );
        self.metadata = Some(metadata);
        Ok(tour)
    }

    /// Computes the integer TSPLIB95 `GEO` distance between two nodes.
    ///
    /// The first two coordinates of each node are interpreted as geographical
    /// latitude and longitude in TSPLIB's degree-minute representation.
    ///
    /// # Errors
    ///
    /// [`IoError::Import`] if a node has fewer than two coordinates.
    pub fn compute_2d_geographical_distance(a: &TsplibNode, b: &TsplibNode) -> Result<i32> {
        Ok(geographical_distance(a, b)? as i32)
    }
}

impl<V> Default for TsplibImporter<V> {
    fn default() -> Self {
        Self::new()
    }
}

fn first_token(value: &str) -> &str {
    value.split_whitespace().next().unwrap_or_default()
}

fn required_value<'a>(key: &str, value: Option<&'a str>) -> Result<&'a str> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| IoError::Import(format!("Missing value for key {key}")))
}

fn set_once<T>(slot: &mut Option<T>, key: &str, value: T) -> Result<()> {
    if slot.is_some() {
        return Err(IoError::Import(format!("Multiple values for key {key}")));
    }
    *slot = Some(value);
    Ok(())
}

fn validated(value: &str, key: &str, allowed: &[&str]) -> Result<String> {
    let value = first_token(value);
    allowed
        .iter()
        .find(|candidate| candidate.eq_ignore_ascii_case(value))
        .map(|value| (*value).to_owned())
        .ok_or_else(|| IoError::Import(format!("Invalid {key} value <{value}>")))
}

fn integer<T: std::str::FromStr>(value: &str, key: &str) -> Result<T> {
    let value = first_token(value);
    value
        .parse()
        .map_err(|_| IoError::Import(format!("Invalid {key} integer value <{value}>")))
}

fn parse_specification(specification: &mut Specification, line: &str) -> Result<bool> {
    let (raw_key, raw_value) = line
        .split_once(':')
        .map_or((line, None), |(key, value)| (key, Some(value)));
    let key = raw_key.trim().to_ascii_uppercase();
    let recognized = matches!(
        key.as_str(),
        "NAME"
            | "TYPE"
            | "COMMENT"
            | "DIMENSION"
            | "CAPACITY"
            | "EDGE_WEIGHT_TYPE"
            | "EDGE_WEIGHT_FORMAT"
            | "EDGE_DATA_FORMAT"
            | "NODE_COORD_TYPE"
            | "DISPLAY_DATA_TYPE"
    );
    if !recognized {
        return Ok(false);
    }
    let value = required_value(&key, raw_value)?;
    match key.as_str() {
        "NAME" => set_once(&mut specification.name, &key, value.to_owned())?,
        "TYPE" => {
            let value = validated(value, &key, VALID_TYPES)?;
            set_once(&mut specification.problem_type, &key, value)?;
        }
        "COMMENT" => specification.comments.push(value.to_owned()),
        "DIMENSION" => {
            let value = integer(value, &key)?;
            set_once(&mut specification.dimension, &key, value)?;
        }
        "CAPACITY" => {
            let value = integer(value, &key)?;
            set_once(&mut specification.capacity, &key, value)?;
        }
        "EDGE_WEIGHT_TYPE" => {
            let value = validated(value, &key, VALID_EDGE_WEIGHT_TYPES)?;
            set_once(&mut specification.edge_weight_type, &key, value)?;
        }
        "EDGE_WEIGHT_FORMAT" => {
            let value = validated(value, &key, VALID_EDGE_WEIGHT_FORMATS)?;
            set_once(&mut specification.edge_weight_format, &key, value)?;
        }
        "EDGE_DATA_FORMAT" => {
            let value = validated(value, &key, VALID_EDGE_DATA_FORMATS)?;
            set_once(&mut specification.edge_data_format, &key, value)?;
        }
        "NODE_COORD_TYPE" => {
            let value = validated(value, &key, VALID_NODE_COORD_TYPES)?;
            set_once(&mut specification.node_coord_type, &key, value)?;
        }
        "DISPLAY_DATA_TYPE" => {
            let value = validated(value, &key, VALID_DISPLAY_DATA_TYPES)?;
            set_once(&mut specification.display_data_type, &key, value)?;
        }
        _ => unreachable!(),
    }
    Ok(true)
}

fn dimension_for(kind: &str) -> Result<usize> {
    match kind {
        "EUC_2D" | "MAX_2D" | "MAN_2D" | "CEIL_2D" | "GEO" | "ATT" => Ok(2),
        "EUC_3D" | "MAX_3D" | "MAN_3D" => Ok(3),
        _ => Err(IoError::Import(format!(
            "Unsupported EDGE_WEIGHT_TYPE <{kind}>"
        ))),
    }
}

fn parse_file(text: &str) -> Result<ParsedFile> {
    let lines: Vec<_> = text.lines().map(str::trim).collect();
    let mut specification = Specification::default();
    let mut nodes = None;
    let mut tour_numbers = None;
    let mut index = 0;
    while index < lines.len() {
        let line = lines[index];
        index += 1;
        if line == "EOF" {
            break;
        }
        if line.is_empty() {
            continue;
        }
        if line.eq_ignore_ascii_case("NODE_COORD_SECTION") {
            if nodes.is_some() {
                return Err(IoError::Import(
                    "Multiple values for key NODE_COORD_SECTION".into(),
                ));
            }
            let kind = specification.edge_weight_type.as_deref().ok_or_else(|| {
                IoError::Import("Missing data to read <NODE_COORD_SECTION>".into())
            })?;
            let dimension = specification.dimension.ok_or_else(|| {
                IoError::Import("Missing data to read <NODE_COORD_SECTION>".into())
            })?;
            let coordinate_count = dimension_for(kind)?;
            let mut parsed_nodes = Vec::with_capacity(dimension);
            while parsed_nodes.len() < dimension && index < lines.len() {
                let node_line = lines[index];
                if node_line == "EOF"
                    || node_line.eq_ignore_ascii_case("TOUR_SECTION")
                    || node_line.eq_ignore_ascii_case("NODE_COORD_SECTION")
                {
                    break;
                }
                index += 1;
                if node_line.is_empty() {
                    continue;
                }
                let values: Vec<_> = node_line.split_whitespace().collect();
                if values.len() != coordinate_count + 1 {
                    return Err(IoError::Import(format!(
                        "Unexpected number of elements <{}> in line: {node_line}",
                        values.len()
                    )));
                }
                parsed_nodes.push(TsplibNode {
                    number: values[0]
                        .parse()
                        .map_err(|_| IoError::Import("invalid TSPLIB node number".into()))?,
                    coordinates: values[1..]
                        .iter()
                        .map(|value| {
                            value
                                .parse()
                                .map_err(|_| IoError::Import("invalid TSPLIB coordinate".into()))
                        })
                        .collect::<Result<_>>()?,
                });
            }
            nodes = Some(parsed_nodes);
            continue;
        }
        if line.eq_ignore_ascii_case("TOUR_SECTION") {
            if tour_numbers.is_some() {
                return Err(IoError::Import(
                    "Multiple values for key TOUR_SECTION".into(),
                ));
            }
            let mut tour = Vec::with_capacity(specification.dimension.unwrap_or(0));
            while index < lines.len() {
                let tour_line = lines[index];
                index += 1;
                if tour_line == "EOF" || tour_line == "-1" {
                    break;
                }
                if tour_line.is_empty() {
                    continue;
                }
                tour.push(
                    tour_line
                        .parse()
                        .map_err(|_| IoError::Import("invalid TSPLIB tour entry".into()))?,
                );
            }
            tour_numbers = Some(tour);
            continue;
        }
        parse_specification(&mut specification, line)?;
    }
    Ok(ParsedFile {
        specification,
        nodes,
        tour_numbers,
    })
}

fn nint(value: f64) -> f64 {
    (value + 0.5).floor()
}

/// TSPLIB `GEO` distance; the first two coordinates of a node are its latitude and longitude.
fn geographical_distance(a: &TsplibNode, b: &TsplibNode) -> Result<f64> {
    let coordinates = |node: &TsplibNode| match node.coordinates[..] {
        [latitude, longitude, ..] => Ok((latitude, longitude)),
        _ => Err(IoError::Import(format!(
            "GEO distance needs two coordinates per node, node {} has {}",
            node.number,
            node.coordinates.len()
        ))),
    };
    let ((lat_a, lon_a), (lat_b, lon_b)) = (coordinates(a)?, coordinates(b)?);
    #[allow(clippy::approx_constant)] // Exact constant required by TSPLIB95.
    const PI: f64 = 3.141592;
    const RRR: f64 = 6378.388;
    fn radians(value: f64) -> f64 {
        let degrees = value.round();
        let minutes = value - degrees;
        PI * (degrees + 5.0 * minutes / 3.0) / 180.0
    }
    let (lat_a, lon_a) = (radians(lat_a), radians(lon_a));
    let (lat_b, lon_b) = (radians(lat_b), radians(lon_b));
    let q1 = (lon_a - lon_b).cos();
    let q2 = (lat_a - lat_b).cos();
    let q3 = (lat_a + lat_b).cos();
    Ok((RRR * (0.5 * ((1.0 + q1) * q2 - (1.0 - q1) * q3)).acos() + 1.0).floor())
}

fn distance(kind: &str, a: &TsplibNode, b: &TsplibNode) -> Result<f64> {
    let differences: Vec<_> = a
        .coordinates
        .iter()
        .zip(&b.coordinates)
        .map(|(a, b)| (a - b).abs())
        .collect();
    Ok(match kind {
        "EUC_2D" | "EUC_3D" => nint(
            differences
                .iter()
                .map(|value| value * value)
                .sum::<f64>()
                .sqrt(),
        ),
        "MAX_2D" | "MAX_3D" => nint(differences.into_iter().fold(0.0, f64::max)),
        "MAN_2D" | "MAN_3D" => nint(differences.into_iter().sum()),
        "CEIL_2D" => differences
            .iter()
            .map(|value| value * value)
            .sum::<f64>()
            .sqrt()
            .ceil(),
        "GEO" => geographical_distance(a, b)?,
        "ATT" => {
            let raw = (differences.iter().map(|value| value * value).sum::<f64>() / 10.0).sqrt();
            let rounded = nint(raw);
            if rounded < raw {
                rounded + 1.0
            } else {
                rounded
            }
        }
        _ => {
            return Err(IoError::Import(format!(
                "Unsupported EDGE_WEIGHT_TYPE <{kind}>"
            )));
        }
    })
}

fn number_mapping<V: Clone + Eq + Hash>(nodes: &IndexMap<V, TsplibNode>) -> HashMap<i32, V> {
    nodes
        .iter()
        .map(|(vertex, node)| (node.number, vertex.clone()))
        .collect()
}

fn map_tour<V: Clone>(numbers: Vec<i32>, by_number: &HashMap<i32, V>) -> Result<Vec<V>> {
    numbers
        .into_iter()
        .map(|number| {
            by_number
                .get(&number)
                .cloned()
                .ok_or_else(|| IoError::Import(format!("Missing vertex with number {number}")))
        })
        .collect()
}

fn distinct_locations<V>(nodes: &IndexMap<V, TsplibNode>) -> bool {
    nodes.values().enumerate().all(|(index, node)| {
        nodes
            .values()
            .skip(index + 1)
            .all(|other| node.coordinates != other.coordinates)
    })
}

fn distinct_neighbor_distances<G: IncidenceGraph>(graph: &G) -> Result<bool> {
    for vertex in graph.vertices() {
        let mut weights = HashSet::new();
        for edge in graph.edges_of(vertex) {
            if !weights.insert(crate::exporter::edge_weight(graph, edge)?.to_bits()) {
                return Ok(false);
            }
        }
    }
    Ok(true)
}

fn metadata_from_parts<V>(
    specification: Specification,
    nodes: IndexMap<V, TsplibNode>,
    tour: Option<Vec<V>>,
    distinct_node_locations: bool,
    distinct_neighbor_distances: bool,
) -> TsplibMetadata<V> {
    let comment = specification.comments.first().cloned();
    TsplibMetadata {
        name: specification.name,
        comment,
        comments: specification.comments,
        problem_type: specification.problem_type,
        edge_weight_type: specification.edge_weight_type.unwrap_or_default(),
        dimension: specification.dimension.unwrap_or_default(),
        capacity: specification.capacity,
        edge_weight_format: specification.edge_weight_format,
        edge_data_format: specification.edge_data_format,
        node_coord_type: specification.node_coord_type,
        display_data_type: specification.display_data_type,
        nodes,
        tour,
        distinct_node_locations,
        distinct_neighbor_distances,
    }
}

impl<V, G> GraphImporter<G> for TsplibImporter<V>
where
    G: GraphMut<Vertex = V> + IncidenceGraph,
    V: Clone + Eq + Hash,
{
    fn import_graph<R: Read>(&mut self, graph: &mut G, mut input: R) -> Result<()> {
        self.metadata = None;
        if !graph.graph_type().weighted {
            return Err(IoError::Import("Graph must be weighted".into()));
        }
        let mut text = String::new();
        input.read_to_string(&mut text)?;
        let parsed = parse_file(&text)?;
        let kind = parsed.specification.edge_weight_type.clone();
        let parsed_nodes = parsed.nodes.unwrap_or_default();
        let mut vertices = Vec::with_capacity(parsed_nodes.len());
        let mut nodes = IndexMap::new();
        for node in &parsed_nodes {
            let vertex = graph.add_new_vertex()?;
            nodes.insert(vertex.clone(), node.clone());
            vertices.push(vertex);
        }
        if !parsed_nodes.is_empty() {
            let kind = kind.as_deref().ok_or_else(|| {
                IoError::Import("Missing data to read <NODE_COORD_SECTION>".into())
            })?;
            if graph.graph_type().directed {
                for source in 0..vertices.len() {
                    for target in 0..vertices.len() {
                        if source != target {
                            let edge = graph
                                .add_edge(&vertices[source], &vertices[target])?
                                .ok_or_else(|| {
                                    IoError::Import("graph rejected TSPLIB edge".into())
                                })?;
                            graph.set_edge_weight(
                                &edge,
                                distance(kind, &parsed_nodes[source], &parsed_nodes[target])?,
                            )?;
                        }
                    }
                }
            } else {
                for source in 0..vertices.len() {
                    for target in source + 1..vertices.len() {
                        let edge = graph
                            .add_edge(&vertices[source], &vertices[target])?
                            .ok_or_else(|| IoError::Import("graph rejected TSPLIB edge".into()))?;
                        graph.set_edge_weight(
                            &edge,
                            distance(kind, &parsed_nodes[source], &parsed_nodes[target])?,
                        )?;
                    }
                }
            }
        }
        let by_number = number_mapping(&nodes);
        let tour = parsed
            .tour_numbers
            .map(|numbers| map_tour(numbers, &by_number))
            .transpose()?;
        let locations_are_distinct = distinct_locations(&nodes);
        let distances_are_distinct = distinct_neighbor_distances(graph)?;
        self.metadata = Some(metadata_from_parts(
            parsed.specification,
            nodes,
            tour,
            locations_are_distinct,
            distances_are_distinct,
        ));
        Ok(())
    }
}
