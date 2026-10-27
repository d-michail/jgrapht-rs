//! TSPLIB coordinate-instance importer.

use crate::{GraphImporter, IoError, Result};
use indexmap::IndexMap;
use jgrapht::GraphMut;
use std::collections::HashMap;
use std::hash::Hash;
use std::io::Read;

#[derive(Debug, Clone, PartialEq)]
pub struct TsplibNode {
    pub number: i32,
    pub coordinates: Vec<f64>,
}

#[derive(Debug, Clone)]
pub struct TsplibMetadata<V> {
    pub name: Option<String>,
    pub comment: Option<String>,
    pub problem_type: Option<String>,
    pub edge_weight_type: String,
    pub dimension: usize,
    pub nodes: IndexMap<V, TsplibNode>,
    pub tour: Option<Vec<V>>,
}

pub struct TsplibImporter<V> {
    metadata: Option<TsplibMetadata<V>>,
}

impl<V> TsplibImporter<V> {
    pub fn new() -> Self {
        Self { metadata: None }
    }
    pub fn metadata(&self) -> Option<&TsplibMetadata<V>> {
        self.metadata.as_ref()
    }
}
impl<V> Default for TsplibImporter<V> {
    fn default() -> Self {
        Self::new()
    }
}

fn dimension_for(kind: &str) -> Result<usize> {
    match kind {
        "EUC_2D" | "MAX_2D" | "MAN_2D" | "CEIL_2D" | "GEO" | "ATT" => Ok(2),
        "EUC_3D" | "MAX_3D" | "MAN_3D" => Ok(3),
        _ => Err(IoError::Import(format!(
            "unsupported TSPLIB EDGE_WEIGHT_TYPE {kind:?}"
        ))),
    }
}

fn nint(value: f64) -> f64 {
    (value + 0.5).floor()
}

fn distance(kind: &str, a: &TsplibNode, b: &TsplibNode) -> f64 {
    let differences: Vec<_> = a
        .coordinates
        .iter()
        .zip(&b.coordinates)
        .map(|(a, b)| (a - b).abs())
        .collect();
    match kind {
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
        "ATT" => {
            let raw = (differences.iter().map(|value| value * value).sum::<f64>() / 10.0).sqrt();
            let rounded = nint(raw);
            if rounded < raw {
                rounded + 1.0
            } else {
                rounded
            }
        }
        "GEO" => {
            fn radians(value: f64) -> f64 {
                let degrees = value.floor();
                let minutes = value - degrees;
                std::f64::consts::PI * (degrees + 5.0 * minutes / 3.0) / 180.0
            }
            let (lat_a, lon_a) = (radians(a.coordinates[0]), radians(a.coordinates[1]));
            let (lat_b, lon_b) = (radians(b.coordinates[0]), radians(b.coordinates[1]));
            let q1 = (lon_a - lon_b).cos();
            let q2 = (lat_a - lat_b).cos();
            let q3 = (lat_a + lat_b).cos();
            (6378.388 * (0.5 * ((1.0 + q1) * q2 - (1.0 - q1) * q3)).acos() + 1.0).floor()
        }
        _ => unreachable!(),
    }
}

impl<V, G> GraphImporter<G> for TsplibImporter<V>
where
    G: GraphMut<Vertex = V>,
    V: Clone + Eq + Hash,
{
    fn import_graph<R: Read>(&mut self, graph: &mut G, mut input: R) -> Result<()> {
        if !graph.graph_type().weighted {
            return Err(IoError::Import("TSPLIB requires a weighted graph".into()));
        }
        let mut text = String::new();
        input.read_to_string(&mut text)?;
        let lines: Vec<_> = text
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .collect();
        let mut fields = HashMap::<String, String>::new();
        let mut section = None;
        let mut node_lines = Vec::new();
        let mut tour_numbers = Vec::new();
        for line in lines {
            if line.eq_ignore_ascii_case("NODE_COORD_SECTION") {
                section = Some("nodes");
                continue;
            }
            if line.eq_ignore_ascii_case("TOUR_SECTION") {
                section = Some("tour");
                continue;
            }
            if line.eq_ignore_ascii_case("EOF") {
                break;
            }
            match section {
                Some("nodes") => node_lines.push(line.to_owned()),
                Some("tour") => {
                    let value = line
                        .parse::<i32>()
                        .map_err(|_| IoError::Import("invalid TSPLIB tour entry".into()))?;
                    if value == -1 {
                        section = None;
                    } else {
                        tour_numbers.push(value);
                    }
                }
                _ => {
                    let (key, value) = line.split_once(':').ok_or_else(|| {
                        IoError::Import(format!("invalid TSPLIB specification line {line:?}"))
                    })?;
                    fields.insert(
                        key.trim().to_ascii_uppercase(),
                        value
                            .split('#')
                            .next()
                            .unwrap_or_default()
                            .trim()
                            .to_owned(),
                    );
                }
            }
        }
        let dimension = fields
            .get("DIMENSION")
            .ok_or_else(|| IoError::Import("TSPLIB DIMENSION is missing".into()))?
            .parse::<usize>()
            .map_err(|_| IoError::Import("invalid TSPLIB DIMENSION".into()))?;
        let kind = fields
            .get("EDGE_WEIGHT_TYPE")
            .ok_or_else(|| IoError::Import("TSPLIB EDGE_WEIGHT_TYPE is missing".into()))?
            .clone();
        let coordinate_count = dimension_for(&kind)?;
        if node_lines.len() < dimension {
            return Err(IoError::Import(
                "TSPLIB NODE_COORD_SECTION is incomplete".into(),
            ));
        }
        let mut nodes = Vec::with_capacity(dimension);
        for line in node_lines.into_iter().take(dimension) {
            let values: Vec<_> = line.split_whitespace().collect();
            if values.len() != coordinate_count + 1 {
                return Err(IoError::Import(format!(
                    "wrong coordinate count in TSPLIB line {line:?}"
                )));
            }
            nodes.push(TsplibNode {
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
        let mut vertices = Vec::with_capacity(dimension);
        let mut mapping = IndexMap::new();
        let mut by_number = HashMap::new();
        for node in &nodes {
            let vertex = graph.add_new_vertex()?;
            by_number.insert(node.number, vertex.clone());
            mapping.insert(vertex.clone(), node.clone());
            vertices.push(vertex);
        }
        if graph.graph_type().directed {
            for i in 0..vertices.len() {
                for j in 0..vertices.len() {
                    if i != j {
                        let edge = graph
                            .add_edge(&vertices[i], &vertices[j])?
                            .ok_or_else(|| IoError::Import("graph rejected TSPLIB edge".into()))?;
                        graph.set_edge_weight(&edge, distance(&kind, &nodes[i], &nodes[j]))?;
                    }
                }
            }
        } else {
            for i in 0..vertices.len() {
                for j in i + 1..vertices.len() {
                    let edge = graph
                        .add_edge(&vertices[i], &vertices[j])?
                        .ok_or_else(|| IoError::Import("graph rejected TSPLIB edge".into()))?;
                    graph.set_edge_weight(&edge, distance(&kind, &nodes[i], &nodes[j]))?;
                }
            }
        }
        let tour = if tour_numbers.is_empty() {
            None
        } else {
            Some(
                tour_numbers
                    .into_iter()
                    .map(|number| {
                        by_number.get(&number).cloned().ok_or_else(|| {
                            IoError::Import(format!("TSPLIB tour references missing node {number}"))
                        })
                    })
                    .collect::<Result<_>>()?,
            )
        };
        self.metadata = Some(TsplibMetadata {
            name: fields.remove("NAME"),
            comment: fields.remove("COMMENT"),
            problem_type: fields.remove("TYPE"),
            edge_weight_type: kind,
            dimension,
            nodes: mapping,
            tour,
        });
        Ok(())
    }
}
