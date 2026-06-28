pub mod dataset_label;
pub mod domain;
pub mod retrograde;

use shakmaty::{Chess, Position};
use std::collections::{HashMap, HashSet, VecDeque};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GameValue {
    Win(u32),  // Win in N half-moves
    Loss(u32), // Loss in N half-moves
    Unknown,
}

pub struct RetrogradeEngine {
    pub tablebase: HashMap<Chess, GameValue>,
    pub unresolved_children: HashMap<Chess, usize>,
    queue: VecDeque<Chess>,
}

impl Default for RetrogradeEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl RetrogradeEngine {
    #[must_use]
    pub fn new() -> Self {
        Self {
            tablebase: HashMap::new(),
            unresolved_children: HashMap::new(),
            queue: VecDeque::new(),
        }
    }

    pub fn add_terminal(&mut self, pos: Chess, value: GameValue) {
        self.tablebase.insert(pos.clone(), value);
        self.queue.push_back(pos);
    }

    pub fn solve(&mut self, max_expansions: usize) -> usize {
        let mut expanded = 0;
        let mut buffer = Vec::with_capacity(64);

        while expanded < max_expansions {
            let Some(current_pos) = self.queue.pop_front() else {
                break;
            };
            expanded += 1;

            let current_value = *self.tablebase.get(&current_pos).unwrap();
            retrograde::inverse_moves(&current_pos, &mut buffer);
            let mut seen_parents = HashSet::with_capacity(buffer.len());

            for parent in &buffer {
                if !seen_parents.insert(parent) {
                    continue;
                }
                if self.tablebase.contains_key(parent) {
                    continue;
                }

                match current_value {
                    GameValue::Loss(n) => {
                        self.tablebase.insert(parent.clone(), GameValue::Win(n + 1));
                        self.queue.push_back(parent.clone());
                    }
                    GameValue::Win(n) => {
                        let moves = parent.legal_moves().len();
                        let count = self
                            .unresolved_children
                            .entry(parent.clone())
                            .or_insert(moves);
                        *count = count.saturating_sub(1);
                        if *count == 0 {
                            self.tablebase
                                .insert(parent.clone(), GameValue::Loss(n + 1));
                            self.queue.push_back(parent.clone());
                        }
                    }
                    _ => {}
                }
            }
        }
        expanded
    }
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
}
