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

use crate::{GraphError, Result};

/// Compressed sparse row boolean matrix (`CSRBooleanMatrix`): row offsets plus the column index
/// of every non-zero entry, ordered by row and then by column.
#[derive(Debug, Clone)]
pub struct CsrBooleanMatrix {
    columns: usize,
    row_offsets: Vec<usize>,
    column_indices: Vec<usize>,
}

impl CsrBooleanMatrix {
    /// Builds the matrix from `(row, column)` entries. Entries are sorted lexicographically, as
    /// in Java, in `O(k log k)`; the offsets come from a prefix sum over per-row counts.
    ///
    /// # Errors
    ///
    /// Returns [`GraphError::InvalidParameter`] if a parameter is out of range. Errors of the graph
    /// operations it performs are propagated.
    pub fn new(rows: usize, columns: usize, entries: &[(usize, usize)]) -> Result<Self> {
        let mut sorted = entries.to_vec();
        sorted.sort_unstable();

        let mut row_offsets = vec![0; rows + 1];
        let mut column_indices = Vec::with_capacity(sorted.len());
        for &(row, column) in &sorted {
            if column >= columns {
                return Err(GraphError::InvalidParameter {
                    name: "column",
                    requirement: crate::ParameterRequirement::InBounds,
                });
            }
            if row >= rows {
                return Err(GraphError::InvalidParameter {
                    name: "row",
                    requirement: crate::ParameterRequirement::InBounds,
                });
            }
            column_indices.push(column);
            row_offsets[row + 1] += 1;
        }
        for i in 1..row_offsets.len() {
            row_offsets[i] += row_offsets[i - 1];
        }
        Ok(Self {
            columns,
            row_offsets,
            column_indices,
        })
    }

    /// Number of columns.
    pub fn columns(&self) -> usize {
        self.columns
    }

    /// Number of rows.
    pub fn rows(&self) -> usize {
        self.row_offsets.len() - 1
    }

    /// Number of non-zero entries in `row`.
    pub fn non_zeros(&self, row: usize) -> usize {
        self.row_offsets[row + 1] - self.row_offsets[row]
    }

    /// Column indices of the non-zero entries of `row`, in increasing order.
    pub fn non_zeros_of(&self, row: usize) -> &[usize] {
        &self.column_indices[self.row_offsets[row]..self.row_offsets[row + 1]]
    }
}
