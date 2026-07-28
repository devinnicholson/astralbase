//! Queue-based propagation over inverse chess moves.

use crate::retrograde;
use shakmaty::{Chess, Position};
use std::collections::{HashMap, HashSet, VecDeque};

/// A proved bounded-retrograde result for one side-to-move position.
///
/// Distances count plies (half-moves) from the represented position to a
/// caller-declared terminal seed. `Loss` distances maximize over every proved
/// winning child. A `Win` distance follows the first loss child that proves the
/// parent, so v0.1 does not claim a globally minimal distance-to-mate value.
/// Draw propagation lies outside Astralbase v0.1, so this type has no draw
/// variant.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GameValue {
    /// A discovered move reaches a proved loss along a path of this many plies.
    Win(u32),
    /// The side to move loses after this many plies against delaying play.
    Loss(u32),
    /// The position is stored, but no win/loss proof has been established.
    ///
    /// `Unknown` is epistemic. Draws, stalemate, dead positions, and claims
    /// about forced results require separate rules evidence.
    Unknown,
}

/// The result of querying an engine table.
///
/// This makes the distinction between a stored [`GameValue::Unknown`] and a
/// position that was never inserted explicit at the API boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ProbeResult {
    /// The position has a stored bounded-retrograde value.
    Present(GameValue),
    /// The position has no row in this engine's in-memory table.
    ///
    /// Absence records only that this engine has no stored row.
    Absent,
}

/// A bounded, in-memory retrograde work queue.
///
/// The engine begins only from caller-supplied seeds. It generates legal
/// predecessor candidates, propagates `Win` from any `Loss` child, and
/// propagates `Loss` only after all legal children have been proved `Win`.
/// Persistence, draw/repetition state, exhaustive enumeration, and
/// completeness certificates lie outside this engine.
pub struct RetrogradeEngine {
    /// Stored positions and their current bounded-retrograde values.
    ///
    /// This field remains public for v0.1 source compatibility. Prefer
    /// [`RetrogradeEngine::probe`] for reads and [`RetrogradeEngine::add_seed`]
    /// for writes so queue state stays consistent.
    pub tablebase: HashMap<Chess, GameValue>,
    /// Remaining unproved legal children for positions seen during propagation.
    ///
    /// This field is exposed for v0.1 diagnostics. Mutating it can invalidate
    /// propagation invariants.
    pub unresolved_children: HashMap<Chess, usize>,
    queue: VecDeque<Chess>,
    maximum_winning_child_distance: HashMap<Chess, u32>,
}

impl Default for RetrogradeEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl RetrogradeEngine {
    /// Creates an empty engine with no seeds or inferred positions.
    #[must_use]
    pub fn new() -> Self {
        Self {
            tablebase: HashMap::new(),
            unresolved_children: HashMap::new(),
            queue: VecDeque::new(),
            maximum_winning_child_distance: HashMap::new(),
        }
    }

    /// Inserts a caller-declared seed and schedules it for propagation.
    ///
    /// The caller supplies terminality and `value`. Release validation must
    /// establish both with a rules oracle.
    pub fn add_seed(&mut self, position: Chess, value: GameValue) {
        self.tablebase.insert(position.clone(), value);
        self.queue.push_back(position);
    }

    /// Inserts a caller-declared terminal seed.
    ///
    /// This source-compatible alias for [`RetrogradeEngine::add_seed`] does not
    /// itself validate terminality.
    pub fn add_terminal(&mut self, position: Chess, value: GameValue) {
        self.add_seed(position, value);
    }

    /// Queries a position without conflating a stored unknown with no row.
    #[must_use]
    pub fn probe(&self, position: &Chess) -> ProbeResult {
        self.tablebase
            .get(position)
            .copied()
            .map_or(ProbeResult::Absent, ProbeResult::Present)
    }

    /// Processes at most `max_expansions` queued positions.
    ///
    /// Returns the number of positions removed from the queue. A parent with a
    /// move to `Loss(n)` becomes `Win(n + 1)`. A parent becomes `Loss` only when
    /// every legal child has been observed as `Win`; its distance is one plus
    /// the maximum winning-child distance so the result is independent of queue
    /// order and models a losing side delaying the terminal seed.
    ///
    /// Reaching the budget with unresolved or absent positions proves nothing
    /// about draws or the existence of a forced result.
    pub fn solve(&mut self, max_expansions: usize) -> usize {
        let mut expanded = 0;
        let mut buffer = Vec::with_capacity(64);

        while expanded < max_expansions {
            let Some(current_position) = self.queue.pop_front() else {
                break;
            };
            expanded += 1;

            let current_value = *self
                .tablebase
                .get(&current_position)
                .expect("queued positions must have a stored value");
            retrograde::inverse_moves(&current_position, &mut buffer);
            let mut seen_parents = HashSet::with_capacity(buffer.len());

            for parent in &buffer {
                if !seen_parents.insert(parent) || self.tablebase.contains_key(parent) {
                    continue;
                }

                match current_value {
                    GameValue::Loss(distance) => {
                        self.tablebase
                            .insert(parent.clone(), GameValue::Win(distance.saturating_add(1)));
                        self.queue.push_back(parent.clone());
                    }
                    GameValue::Win(distance) => {
                        let legal_child_count = parent.legal_moves().len();
                        let remaining = self
                            .unresolved_children
                            .entry(parent.clone())
                            .or_insert(legal_child_count);
                        let maximum_distance = self
                            .maximum_winning_child_distance
                            .entry(parent.clone())
                            .or_insert(distance);
                        record_winning_child_distance(maximum_distance, distance);
                        *remaining = remaining.saturating_sub(1);

                        if *remaining == 0 {
                            let loss_distance = maximum_distance.saturating_add(1);
                            self.tablebase
                                .insert(parent.clone(), GameValue::Loss(loss_distance));
                            self.queue.push_back(parent.clone());
                        }
                    }
                    GameValue::Unknown => {}
                }
            }
        }
        expanded
    }
}

fn record_winning_child_distance(maximum_distance: &mut u32, distance: u32) {
    *maximum_distance = (*maximum_distance).max(distance);
}

#[cfg(test)]
mod tests {
    use super::*;
    use shakmaty::{CastlingMode, EnPassantMode, fen::Fen};
    use std::str::FromStr;

    fn sample_mate() -> Chess {
        Fen::from_str("rnb1kbnr/pppp1ppp/8/4p3/6Pq/5P2/PPPPP2P/RNBQKBNR w KQkq - 1 3")
            .unwrap()
            .into_position(CastlingMode::Standard)
            .unwrap()
    }

    #[test]
    fn solve_respects_zero_expansion_limit() {
        let mut engine = RetrogradeEngine::new();
        engine.add_terminal(sample_mate(), GameValue::Loss(0));

        assert_eq!(engine.solve(0), 0);
        assert_eq!(engine.solve(1), 1);
    }

    #[test]
    fn inverse_parents_can_legally_reach_child() {
        let child = sample_mate();
        let mut parents = Vec::new();

        retrograde::inverse_moves(&child, &mut parents);

        assert!(!parents.is_empty());
        for parent in parents {
            let round_trips = parent
                .legal_moves()
                .iter()
                .any(|mv| parent.clone().play(mv).is_ok_and(|next| next == child));

            assert!(
                round_trips,
                "generated parent cannot legally reach child: {}",
                Fen::from_position(parent, EnPassantMode::Legal)
            );
        }
    }

    #[test]
    fn probe_distinguishes_absent_and_stored_unknown() {
        let position = sample_mate();
        let mut engine = RetrogradeEngine::new();
        assert_eq!(engine.probe(&position), ProbeResult::Absent);

        engine.add_seed(position.clone(), GameValue::Unknown);
        assert_eq!(
            engine.probe(&position),
            ProbeResult::Present(GameValue::Unknown)
        );
    }

    #[test]
    fn a04_delaying_loss_distance_is_order_independent() {
        for distances in [[2, 4], [4, 2]] {
            let mut maximum = distances[0];
            for distance in distances {
                record_winning_child_distance(&mut maximum, distance);
            }
            assert_eq!(maximum.saturating_add(1), 5);
        }
    }
}
