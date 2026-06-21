use shakmaty::{Chess, Color, Position, Role, Setup, Square, Move, FromSetup, CastlingMode, Rank, Piece};
use std::num::NonZeroU32;

/// Finds all positions `P'` such that a legal move from `P'` leads to `pos`.
pub fn inverse_moves(pos: &Chess) -> Vec<Chess> {
    let mut parents = Vec::new();
    
    let us = pos.turn().other();
    let them = pos.turn();

    if pos.clone().swap_turn().map(|p| p.is_check()).unwrap_or(false) {
        return parents; 
    }

    let board = pos.board();

    for sq in board.by_color(us) {
        let role = board.role_at(sq).unwrap();

        // Handle promotions (straight and capture)
        if role != Role::King && role != Role::Pawn {
            let promo_rank = if us == Color::White { Rank::Eighth } else { Rank::First };
            if sq.rank() == promo_rank {
                let direction = if us == Color::White { -8 } else { 8 };
                // 1. Straight promotion
                if let Some(src) = sq.offset(direction) {
                    if board.piece_at(src).is_none() {
                        let mut prev_board = board.clone();
                        prev_board.discard_piece_at(sq);
                        prev_board.set_piece_at(src, Piece { role: Role::Pawn, color: us });

                        if let Ok(prev_pos) = Chess::from_setup(
                            Setup {
                                board: prev_board.clone(),
                                promoted: pos.promoted().without(sq),
                                pockets: pos.pockets().cloned(),
                                turn: us,
                                castling_rights: pos.castles().castling_rights(),
                                ep_square: None,
                                remaining_checks: pos.remaining_checks().copied(),
                                halfmoves: 0,
                                fullmoves: if us == Color::Black { pos.fullmoves() } else { NonZeroU32::new(pos.fullmoves().get().saturating_sub(1).max(1)).unwrap() },
                            },
                            CastlingMode::Standard,
                        ) {
                            let fwd_move = Move::Normal { role: Role::Pawn, from: src, to: sq, capture: None, promotion: Some(role) };
                            if prev_pos.is_legal(&fwd_move) {
                                parents.push(prev_pos);
                            }
                        }
                    }
                }
                
                // 2. Capture promotion
                let capture_offsets = if us == Color::White { vec![-7, -9] } else { vec![7, 9] };
                for off in capture_offsets {
                    if let Some(src) = sq.offset(off) {
                        if board.piece_at(src).is_none() && src.distance(sq) == 1 {
                            for captured_role in [Role::Queen, Role::Rook, Role::Bishop, Role::Knight, Role::Pawn] {
                                let mut uncap_board = board.clone();
                                uncap_board.discard_piece_at(sq);
                                uncap_board.set_piece_at(src, Piece { role: Role::Pawn, color: us });
                                uncap_board.set_piece_at(sq, Piece { role: captured_role, color: them });
                                
                                if let Ok(prev_pos_cap) = Chess::from_setup(
                                    Setup {
                                        board: uncap_board,
                                        promoted: pos.promoted().without(sq),
                                        pockets: pos.pockets().cloned(),
                                        turn: us,
                                        castling_rights: pos.castles().castling_rights(),
                                        ep_square: None,
                                        remaining_checks: pos.remaining_checks().copied(),
                                        halfmoves: 0,
                                        fullmoves: if us == Color::Black { pos.fullmoves() } else { NonZeroU32::new(pos.fullmoves().get().saturating_sub(1).max(1)).unwrap() },
                                    },
                                    CastlingMode::Standard,
                                ) {
                                    let fwd_cap = Move::Normal { role: Role::Pawn, from: src, to: sq, capture: Some(captured_role), promotion: Some(role) };
                                    if prev_pos_cap.is_legal(&fwd_cap) {
                                        parents.push(prev_pos_cap);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // 1. Unmoves for standard pieces (King, Queen, Rook, Bishop, Knight)
        if role != Role::Pawn {
            let attacks = shakmaty::attacks::attacks(sq, Piece { role, color: us }, board.occupied());

            for src in attacks {
                if board.piece_at(src).is_none() {
                    let mut prev_board = board.clone();
                    let piece = board.piece_at(sq).unwrap();
                    prev_board.discard_piece_at(sq);
                    prev_board.set_piece_at(src, piece);

                    let cr = pos.castles().castling_rights();

                    // Quiet unmove
                    if let Ok(prev_pos) = Chess::from_setup(
                        Setup {
                            board: prev_board.clone(),
                            promoted: pos.promoted().clone(),
                            pockets: pos.pockets().cloned(),
                            turn: us,
                            castling_rights: cr,
                            ep_square: None,
                            remaining_checks: pos.remaining_checks().copied(),
                            halfmoves: pos.halfmoves().saturating_sub(1),
                            fullmoves: if us == Color::Black { pos.fullmoves() } else { NonZeroU32::new(pos.fullmoves().get().saturating_sub(1).max(1)).unwrap() },
                        },
                        CastlingMode::Standard,
                    ) {
                        let fwd_move = Move::Normal { role, from: src, to: sq, capture: None, promotion: None };
                        if prev_pos.is_legal(&fwd_move) {
                            parents.push(prev_pos);
                        }
                    }

                    // Uncaptures
                    for captured_role in [Role::Queen, Role::Rook, Role::Bishop, Role::Knight, Role::Pawn] {
                        let mut uncap_board = prev_board.clone();
                        uncap_board.set_piece_at(sq, Piece { role: captured_role, color: them });
                        
                        if captured_role == Role::Pawn && (sq.rank() == Rank::First || sq.rank() == Rank::Eighth) {
                            continue;
                        }

                        if let Ok(prev_pos_cap) = Chess::from_setup(
                            Setup {
                                board: uncap_board,
                                promoted: pos.promoted().clone(),
                                pockets: pos.pockets().cloned(),
                                turn: us,
                                castling_rights: cr,
                                ep_square: None,
                                remaining_checks: pos.remaining_checks().copied(),
                                halfmoves: 0,
                                fullmoves: if us == Color::Black { pos.fullmoves() } else { NonZeroU32::new(pos.fullmoves().get().saturating_sub(1).max(1)).unwrap() },
                            },
                            CastlingMode::Standard,
                        ) {
                            let fwd_cap = Move::Normal { role, from: src, to: sq, capture: Some(captured_role), promotion: None };
                            if prev_pos_cap.is_legal(&fwd_cap) {
                                parents.push(prev_pos_cap);
                            }
                        }
                    }
                }
            }
        } else {
            // Unmoves for Pawns
            let direction = if us == Color::White { -8 } else { 8 };
            
            // Single push
            if let Some(src) = sq.offset(direction) {
                if board.piece_at(src).is_none() {
                    let mut prev_board = board.clone();
                    let piece = board.piece_at(sq).unwrap();
                    prev_board.discard_piece_at(sq);
                    prev_board.set_piece_at(src, piece);
                    
                    if let Ok(prev_pos) = Chess::from_setup(
                        Setup {
                            board: prev_board.clone(),
                            promoted: pos.promoted().clone(),
                            pockets: pos.pockets().cloned(),
                            turn: us,
                            castling_rights: pos.castles().castling_rights(),
                            ep_square: None,
                            remaining_checks: pos.remaining_checks().copied(),
                            halfmoves: pos.halfmoves().saturating_sub(1),
                            fullmoves: if us == Color::Black { pos.fullmoves() } else { NonZeroU32::new(pos.fullmoves().get().saturating_sub(1).max(1)).unwrap() },
                        },
                        CastlingMode::Standard,
                    ) {
                        let fwd_move = Move::Normal { role: Role::Pawn, from: src, to: sq, capture: None, promotion: None };
                        if prev_pos.is_legal(&fwd_move) {
                            parents.push(prev_pos);
                        }
                    }

                    // Double push
                    if src.rank() == if us == Color::White { Rank::Third } else { Rank::Sixth } {
                        if let Some(src_dbl) = src.offset(direction) {
                            if board.piece_at(src_dbl).is_none() {
                                let mut prev_board_dbl = board.clone();
                                let piece = board.piece_at(sq).unwrap();
                                prev_board_dbl.discard_piece_at(sq);
                                prev_board_dbl.set_piece_at(src_dbl, piece);
                                
                                if let Ok(prev_pos_dbl) = Chess::from_setup(
                                    Setup {
                                        board: prev_board_dbl.clone(),
                                        promoted: pos.promoted().clone(),
                                        pockets: pos.pockets().cloned(),
                                        turn: us,
                                        castling_rights: pos.castles().castling_rights(),
                                        ep_square: Some(src),
                                        remaining_checks: pos.remaining_checks().copied(),
                                        halfmoves: pos.halfmoves().saturating_sub(1),
                                        fullmoves: if us == Color::Black { pos.fullmoves() } else { NonZeroU32::new(pos.fullmoves().get().saturating_sub(1).max(1)).unwrap() },
                                    },
                                    CastlingMode::Standard,
                                ) {
                                    let fwd_move = Move::Normal { role: Role::Pawn, from: src_dbl, to: sq, capture: None, promotion: None };
                                    if prev_pos_dbl.is_legal(&fwd_move) {
                                        parents.push(prev_pos_dbl);
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Pawn Uncaptures
            let capture_offsets = if us == Color::White { vec![-7, -9] } else { vec![7, 9] };
            for off in capture_offsets {
                if let Some(src) = sq.offset(off) {
                    if src.distance(sq) == 1 {
                        if board.piece_at(src).is_none() {
                            for captured_role in [Role::Queen, Role::Rook, Role::Bishop, Role::Knight, Role::Pawn] {
                                let mut uncap_board = board.clone();
                                let piece = board.piece_at(sq).unwrap();
                                uncap_board.discard_piece_at(sq);
                                uncap_board.set_piece_at(src, piece);
                                uncap_board.set_piece_at(sq, Piece { role: captured_role, color: them });
                                
                                if captured_role == Role::Pawn && (sq.rank() == Rank::First || sq.rank() == Rank::Eighth) {
                                    continue;
                                }

                                if let Ok(prev_pos_cap) = Chess::from_setup(
                                    Setup {
                                        board: uncap_board.clone(),
                                        promoted: pos.promoted().clone(),
                                        pockets: pos.pockets().cloned(),
                                        turn: us,
                                        castling_rights: pos.castles().castling_rights(),
                                        ep_square: None,
                                        remaining_checks: pos.remaining_checks().copied(),
                                        halfmoves: 0,
                                        fullmoves: if us == Color::Black { pos.fullmoves() } else { NonZeroU32::new(pos.fullmoves().get().saturating_sub(1).max(1)).unwrap() },
                                    },
                                    CastlingMode::Standard,
                                ) {
                                    let fwd_cap = Move::Normal { role: Role::Pawn, from: src, to: sq, capture: Some(captured_role), promotion: None };
                                    if prev_pos_cap.is_legal(&fwd_cap) {
                                        parents.push(prev_pos_cap);
                                    }
                                }
                            }
                        }

                        // En Passant Uncapture
                        if let Some(captured_sq) = sq.offset(direction) {
                            if board.piece_at(src).is_none() && board.piece_at(captured_sq).is_none() {
                                let ep_rank = if us == Color::White { Rank::Sixth } else { Rank::Third };
                                if sq.rank() == ep_rank {
                                    let mut ep_board = board.clone();
                                    ep_board.discard_piece_at(sq);
                                    ep_board.set_piece_at(src, Piece { role: Role::Pawn, color: us });
                                    ep_board.set_piece_at(captured_sq, Piece { role: Role::Pawn, color: them });

                                    if let Ok(prev_pos_ep) = Chess::from_setup(
                                        Setup {
                                            board: ep_board,
                                            promoted: pos.promoted().clone(),
                                            pockets: pos.pockets().cloned(),
                                            turn: us,
                                            castling_rights: pos.castles().castling_rights(),
                                            ep_square: Some(sq),
                                            remaining_checks: pos.remaining_checks().copied(),
                                            halfmoves: 0,
                                            fullmoves: if us == Color::Black { pos.fullmoves() } else { NonZeroU32::new(pos.fullmoves().get().saturating_sub(1).max(1)).unwrap() },
                                        },
                                        CastlingMode::Standard,
                                    ) {
                                        let fwd_ep = Move::EnPassant { from: src, to: sq };
                                        if prev_pos_ep.is_legal(&fwd_ep) {
                                            parents.push(prev_pos_ep);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Un-castling
    let castling_unmoves = if us == Color::White {
        vec![
            (Square::G1, Square::F1, Square::E1, Square::H1, Role::Rook),
            (Square::C1, Square::D1, Square::E1, Square::A1, Role::Rook),
        ]
    } else {
        vec![
            (Square::G8, Square::F8, Square::E8, Square::H8, Role::Rook),
            (Square::C8, Square::D8, Square::E8, Square::A8, Role::Rook),
        ]
    };

    for (k_sq, r_sq, k_src, r_src, _) in castling_unmoves {
        if board.role_at(k_sq) == Some(Role::King) && board.color_at(k_sq) == Some(us) &&
           board.role_at(r_sq) == Some(Role::Rook) && board.color_at(r_sq) == Some(us) {
            
            if board.piece_at(k_src).is_none() && board.piece_at(r_src).is_none() {
                let mut prev_board = board.clone();
                prev_board.discard_piece_at(k_sq);
                prev_board.discard_piece_at(r_sq);
                prev_board.set_piece_at(k_src, Piece { role: Role::King, color: us });
                prev_board.set_piece_at(r_src, Piece { role: Role::Rook, color: us });

                let mut new_cr = pos.castles().castling_rights();
                new_cr.add(r_src);

                if let Ok(prev_pos) = Chess::from_setup(
                    Setup {
                        board: prev_board,
                        promoted: pos.promoted().clone(),
                        pockets: pos.pockets().cloned(),
                        turn: us,
                        castling_rights: new_cr,
                        ep_square: None,
                        remaining_checks: pos.remaining_checks().copied(),
                        halfmoves: pos.halfmoves().saturating_sub(1),
                        fullmoves: if us == Color::Black { pos.fullmoves() } else { NonZeroU32::new(pos.fullmoves().get().saturating_sub(1).max(1)).unwrap() },
                    },
                    CastlingMode::Standard,
                ) {
                    let fwd_castle = Move::Castle { king: k_src, rook: r_src };
                    if prev_pos.is_legal(&fwd_castle) {
                        parents.push(prev_pos);
                    }
                }
            }
        }
    }

    parents
}
