use shakmaty::{fen::Fen, CastlingMode, Chess, Position, Setup, FromSetup, Color, Square};
use std::str::FromStr;

fn main() {
    let fen_str = "8/8/8/8/8/8/5q2/5K1k w - - 0 1";
    let pos: Chess = Fen::from_str(fen_str).unwrap().into_position(CastlingMode::Standard).unwrap();
    let board = pos.board();
    let is_empty = board.piece_at(Square::E4).is_none();
    println!("Empty: {}", is_empty);
}
