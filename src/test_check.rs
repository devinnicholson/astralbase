use shakmaty::{Board, Setup, Color, Square, Bitboard, attacks};

fn test(board: &Board, sq: Square, occupied: Bitboard) {
    let _ = board.attacks_from(sq); // doesn't exist?
}
