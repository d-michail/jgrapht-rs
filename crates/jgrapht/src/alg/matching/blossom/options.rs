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

use std::fmt;

/// Strategy used to initialize the matching before Blossom V's main phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InitializationType {
    /// Start with a greedy integral matching.
    Greedy,
    /// Start with an empty matching.
    None,
    /// Solve the fractional matching relaxation first.
    Fractional,
}

impl fmt::Display for InitializationType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Greedy => "Greedy initialization",
            Self::None => "None",
            Self::Fractional => "Fractional matching initializations",
        })
    }
}

/// Strategy used for global Blossom V dual updates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DualUpdateStrategy {
    /// Update all trees by one fixed delta.
    MultipleTreeFixedDelta,
    /// Update connected components of the auxiliary tree graph.
    MultipleTreeConnectedComponents,
}

impl fmt::Display for DualUpdateStrategy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::MultipleTreeFixedDelta => "Multiple tree fixed delta",
            Self::MultipleTreeConnectedComponents => "Multiple tree connected components",
        })
    }
}

/// Configuration for Kolmogorov's Blossom V algorithm.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BlossomVOptions {
    initialization_type: InitializationType,
    dual_update_strategy: DualUpdateStrategy,
    update_duals_before: bool,
    update_duals_after: bool,
}

impl BlossomVOptions {
    /// Constructs a custom option set.
    pub const fn new(
        initialization_type: InitializationType,
        dual_update_strategy: DualUpdateStrategy,
        update_duals_before: bool,
        update_duals_after: bool,
    ) -> Self {
        Self {
            initialization_type,
            dual_update_strategy,
            update_duals_before,
            update_duals_after,
        }
    }

    /// Constructs the default option set with a different initialization strategy.
    pub const fn with_initialization(initialization_type: InitializationType) -> Self {
        Self::new(
            initialization_type,
            DualUpdateStrategy::MultipleTreeFixedDelta,
            true,
            false,
        )
    }

    /// Returns every option combination used by the upstream parameterized tests.
    ///
    /// Entries 15 and 23 deliberately repeat `update_duals_after = true`, matching Java.
    pub fn all_options() -> [Self; 24] {
        use DualUpdateStrategy::{
            MultipleTreeConnectedComponents as Connected, MultipleTreeFixedDelta as Fixed,
        };
        use InitializationType::{Fractional, Greedy, None};
        [
            Self::new(None, Connected, true, true),
            Self::new(None, Connected, true, false),
            Self::new(None, Connected, false, true),
            Self::new(None, Connected, false, false),
            Self::new(None, Fixed, true, true),
            Self::new(None, Fixed, true, false),
            Self::new(None, Fixed, false, true),
            Self::new(None, Fixed, false, false),
            Self::new(Greedy, Connected, true, true),
            Self::new(Greedy, Connected, true, false),
            Self::new(Greedy, Connected, false, true),
            Self::new(Greedy, Connected, false, false),
            Self::new(Greedy, Fixed, true, true),
            Self::new(Greedy, Fixed, true, false),
            Self::new(Greedy, Fixed, false, true),
            Self::new(Greedy, Fixed, false, true),
            Self::new(Fractional, Connected, true, true),
            Self::new(Fractional, Connected, true, false),
            Self::new(Fractional, Connected, false, true),
            Self::new(Fractional, Connected, false, false),
            Self::new(Fractional, Fixed, true, true),
            Self::new(Fractional, Fixed, true, false),
            Self::new(Fractional, Fixed, false, true),
            Self::new(Fractional, Fixed, false, true),
        ]
    }

    /// Whether duals are updated before growing the current tree.
    pub const fn update_duals_before(self) -> bool {
        self.update_duals_before
    }

    /// Whether duals are updated after growing the current tree.
    pub const fn update_duals_after(self) -> bool {
        self.update_duals_after
    }

    /// Returns the global dual-update strategy.
    pub const fn dual_update_strategy(self) -> DualUpdateStrategy {
        self.dual_update_strategy
    }

    /// Returns the initialization strategy.
    pub const fn initialization_type(self) -> InitializationType {
        self.initialization_type
    }
}

impl Default for BlossomVOptions {
    fn default() -> Self {
        Self::with_initialization(InitializationType::Fractional)
    }
}

impl fmt::Display for BlossomVOptions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "BlossomVOptions{{initializationType={}, dualUpdateStrategy={}, updateDualsBefore={}, updateDualsAfter={}}}",
            self.initialization_type,
            self.dual_update_strategy,
            self.update_duals_before,
            self.update_duals_after
        )
    }
}

/// Whether a weighted matching objective is minimized or maximized.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ObjectiveSense {
    /// Minimize the total matching weight.
    Minimize,
    /// Maximize the total matching weight.
    #[default]
    Maximize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_and_java_option_matrix() {
        let options = BlossomVOptions::default();
        assert_eq!(
            options.initialization_type(),
            InitializationType::Fractional
        );
        assert_eq!(
            options.dual_update_strategy(),
            DualUpdateStrategy::MultipleTreeFixedDelta
        );
        assert!(options.update_duals_before());
        assert!(!options.update_duals_after());

        let all = BlossomVOptions::all_options();
        assert_eq!(all.len(), 24);
        assert_eq!(all[15], all[14]);
        assert_eq!(all[23], all[22]);
    }
}
