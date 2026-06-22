pub mod retrograde;

use shakmaty::{fen::Fen, CastlingMode, Chess, Position, FromSetup};
use std::collections::{HashMap, VecDeque};
use std::str::FromStr;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GameValue {
    Win(u32),   // Win in N half-moves
    Loss(u32),  // Loss in N half-moves
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

        while let Some(current_pos) = self.queue.pop_front() {
            expanded += 1;
            if expanded > max_expansions {
                println!("Reached {max_expansions} expansions. Stopping.");
                break;
            }

            let current_value = *self.tablebase.get(&current_pos).unwrap();
            retrograde::inverse_moves(&current_pos, &mut buffer);

            for parent in &buffer {
                if self.tablebase.contains_key(parent) {
                    continue;
                }

                match current_value {
                    GameValue::Loss(n) => {
                        self.tablebase.insert(parent.clone(), GameValue::Win(n + 1));
                        self.queue.push_back(parent.clone());
                        println!("Found Win({}): {}", n+1, Fen::from_position(parent.clone(), shakmaty::EnPassantMode::Legal));
                    }
                    GameValue::Win(n) => {
                        let moves = parent.legal_moves().len();
                        let count = self.unresolved_children.entry(parent.clone()).or_insert(moves);
                        *count -= 1;
                        if *count == 0 {
                            self.tablebase.insert(parent.clone(), GameValue::Loss(n + 1));
                            self.queue.push_back(parent.clone());
                            println!("Found Loss({}): {}", n+1, Fen::from_position(parent.clone(), shakmaty::EnPassantMode::Legal));
                        }
                    }
                    _ => {}
                }
            }
        }
        expanded
    }
}

fn main() {
    println!("🚀 AstralBase Generator starting...");
    println!("Initializing Retrograde Analysis prototype...");
    
    let fen_str = "8/8/8/8/8/8/5q2/5K1k w - - 0 1"; 
    let setup = Fen::from_str(fen_str).unwrap().into_setup();
    let pos = Chess::from_setup(setup, CastlingMode::Standard).unwrap();

    let mut engine = RetrogradeEngine::new();
    engine.add_terminal(pos, GameValue::Loss(0));

    println!("Starting backward from checkmate: {fen_str}");
    let expanded = engine.solve(100);

    println!("Finished backpropagation prototype. Total expanded: {expanded}");
}
