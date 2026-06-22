use shakmaty::{Board, CastlingMode, Chess, Color, FromSetup, Piece, Position, Rank, Role, Setup, Square};
use std::num::NonZeroU32;

/// Checks if the predecessor board is valid. 
/// A predecessor board is valid if `them` (the side that just moved to reach `prev_board`) 
/// did not leave their king in check.
fn is_valid_predecessor(board: &Board, us: Color, them: Color) -> bool {
    if let Some(them_king) = board.king_of(them) {
        let occupied = board.occupied();
        
        let pawn_attacks = shakmaty::attacks::pawn_attacks(them, them_king);
        if (pawn_attacks & board.pawns() & board.by_color(us)).any() { return false; }

        let knight_attacks = shakmaty::attacks::knight_attacks(them_king);
        if (knight_attacks & board.knights() & board.by_color(us)).any() { return false; }

        let king_attacks = shakmaty::attacks::king_attacks(them_king);
        if (king_attacks & board.kings() & board.by_color(us)).any() { return false; }

        let bishop_attacks = shakmaty::attacks::bishop_attacks(them_king, occupied);
        if (bishop_attacks & (board.bishops() | board.queens()) & board.by_color(us)).any() { return false; }

        let rook_attacks = shakmaty::attacks::rook_attacks(them_king, occupied);
        if (rook_attacks & (board.rooks() | board.queens()) & board.by_color(us)).any() { return false; }
    }
    true
}

/// Finds all positions `P'` such that a legal move from `P'` leads to `pos`.
pub fn inverse_moves(pos: &Chess, parents: &mut Vec<Chess>) {
    parents.clear();
    
    let us = pos.turn().other();
    let them = pos.turn();

    // If the side that just moved (us) is in check in the resulting position, the position is impossible
    if pos.clone().swap_turn().map(|p| p.is_check()).unwrap_or(false) {
        return; 
    }

    let board = pos.board();
    let occupied = board.occupied();

    let mut prev_board = board.clone();

    for sq in board.by_color(us) {
        let role = board.role_at(sq).unwrap();

        // Handle promotions (straight and capture)
        if role != Role::King && role != Role::Pawn {
            let promo_rank = if us == Color::White { Rank::Eighth } else { Rank::First };
            if sq.rank() == promo_rank {
                let direction = if us == Color::White { -8 } else { 8 };
                // 1. Straight promotion
                if let Some(src) = sq.offset(direction)
                    && board.piece_at(src).is_none() {
                        prev_board.discard_piece_at(sq);
                        prev_board.set_piece_at(src, Piece { role: Role::Pawn, color: us });

                        if is_valid_predecessor(&prev_board, us, them)
                            && let Ok(prev_pos) = Chess::from_setup(
                                Setup {
                                    board: prev_board.clone(),
                                    promoted: pos.promoted().without(sq),
                                    pockets: pos.pockets().copied(),
                                    turn: us,
                                    castling_rights: pos.castles().castling_rights(),
                                    ep_square: None,
                                    remaining_checks: pos.remaining_checks().copied(),
                                    halfmoves: 0,
                                    fullmoves: if us == Color::Black { pos.fullmoves() } else { NonZeroU32::new(pos.fullmoves().get().saturating_sub(1).max(1)).unwrap() },
                                },
                                CastlingMode::Standard,
                            ) {
                                parents.push(prev_pos);
                            }
                        
                        // Revert
                        prev_board.discard_piece_at(src);
                        prev_board.set_piece_at(sq, Piece { role, color: us });
                    }
                
                // 2. Capture promotion
                let capture_offsets = if us == Color::White { [-7, -9] } else { [7, 9] };
                for off in capture_offsets {
                    if let Some(src) = sq.offset(off)
                        && board.piece_at(src).is_none() && src.distance(sq) == 1 {
                            for captured_role in [Role::Queen, Role::Rook, Role::Bishop, Role::Knight, Role::Pawn] {
                                prev_board.discard_piece_at(sq);
                                prev_board.set_piece_at(src, Piece { role: Role::Pawn, color: us });
                                prev_board.set_piece_at(sq, Piece { role: captured_role, color: them });
                                
                                if is_valid_predecessor(&prev_board, us, them)
                                    && let Ok(prev_pos_cap) = Chess::from_setup(
                                        Setup {
                                            board: prev_board.clone(),
                                            promoted: pos.promoted().without(sq),
                                            pockets: pos.pockets().copied(),
                                            turn: us,
                                            castling_rights: pos.castles().castling_rights(),
                                            ep_square: None,
                                            remaining_checks: pos.remaining_checks().copied(),
                                            halfmoves: 0,
                                            fullmoves: if us == Color::Black { pos.fullmoves() } else { NonZeroU32::new(pos.fullmoves().get().saturating_sub(1).max(1)).unwrap() },
                                        },
                                        CastlingMode::Standard,
                                    ) {
                                        parents.push(prev_pos_cap);
                                    }
                                
                                // Revert inside loop is just overwritten, but after loop we must restore
                            }
                            // Revert after captures
                            prev_board.discard_piece_at(src);
                            prev_board.set_piece_at(sq, Piece { role, color: us });
                        }
                }
            }
        }

        // Unmoves for standard pieces (King, Queen, Rook, Bishop, Knight)
        if role == Role::Pawn {
            // Unmoves for Pawns
            let direction = if us == Color::White { -8 } else { 8 };
            
            // Single push
            if let Some(src) = sq.offset(direction)
                && board.piece_at(src).is_none() {
                    let piece = Piece { role: Role::Pawn, color: us };
                    prev_board.discard_piece_at(sq);
                    prev_board.set_piece_at(src, piece);
                    
                    if is_valid_predecessor(&prev_board, us, them)
                        && let Ok(prev_pos) = Chess::from_setup(
                            Setup {
                                board: prev_board.clone(),
                                promoted: pos.promoted(),
                                pockets: pos.pockets().copied(),
                                turn: us,
                                castling_rights: pos.castles().castling_rights(),
                                ep_square: None,
                                remaining_checks: pos.remaining_checks().copied(),
                                halfmoves: pos.halfmoves().saturating_sub(1),
                                fullmoves: if us == Color::Black { pos.fullmoves() } else { NonZeroU32::new(pos.fullmoves().get().saturating_sub(1).max(1)).unwrap() },
                            },
                            CastlingMode::Standard,
                        ) {
                            parents.push(prev_pos);
                        }

                    // Double push
                    if src.rank() == if us == Color::White { Rank::Third } else { Rank::Sixth }
                        && let Some(src_dbl) = src.offset(direction)
                            && board.piece_at(src_dbl).is_none() {
                                prev_board.discard_piece_at(src);
                                prev_board.set_piece_at(src_dbl, piece);
                                
                                if is_valid_predecessor(&prev_board, us, them)
                                    && let Ok(prev_pos_dbl) = Chess::from_setup(
                                        Setup {
                                            board: prev_board.clone(),
                                            promoted: pos.promoted(),
                                            pockets: pos.pockets().copied(),
                                            turn: us,
                                            castling_rights: pos.castles().castling_rights(),
                                            ep_square: Some(src),
                                            remaining_checks: pos.remaining_checks().copied(),
                                            halfmoves: pos.halfmoves().saturating_sub(1),
                                            fullmoves: if us == Color::Black { pos.fullmoves() } else { NonZeroU32::new(pos.fullmoves().get().saturating_sub(1).max(1)).unwrap() },
                                        },
                                        CastlingMode::Standard,
                                    ) {
                                        parents.push(prev_pos_dbl);
                                    }
                                
                                // Revert double push
                                prev_board.discard_piece_at(src_dbl);
                                prev_board.set_piece_at(src, piece);
                            }
                    
                    // Revert single push
                    prev_board.discard_piece_at(src);
                    prev_board.set_piece_at(sq, piece);
                }

            // Pawn Uncaptures
            let capture_offsets = if us == Color::White { [-7, -9] } else { [7, 9] };
            for off in capture_offsets {
                if let Some(src) = sq.offset(off)
                    && src.distance(sq) == 1 {
                        if board.piece_at(src).is_none() {
                            let piece = Piece { role: Role::Pawn, color: us };
                            prev_board.discard_piece_at(sq);
                            prev_board.set_piece_at(src, piece);
                            
                            for captured_role in [Role::Queen, Role::Rook, Role::Bishop, Role::Knight, Role::Pawn] {
                                if captured_role == Role::Pawn && (sq.rank() == Rank::First || sq.rank() == Rank::Eighth) {
                                    continue;
                                }

                                prev_board.set_piece_at(sq, Piece { role: captured_role, color: them });
                                
                                if is_valid_predecessor(&prev_board, us, them)
                                    && let Ok(prev_pos_cap) = Chess::from_setup(
                                        Setup {
                                            board: prev_board.clone(),
                                            promoted: pos.promoted(),
                                            pockets: pos.pockets().copied(),
                                            turn: us,
                                            castling_rights: pos.castles().castling_rights(),
                                            ep_square: None,
                                            remaining_checks: pos.remaining_checks().copied(),
                                            halfmoves: 0,
                                            fullmoves: if us == Color::Black { pos.fullmoves() } else { NonZeroU32::new(pos.fullmoves().get().saturating_sub(1).max(1)).unwrap() },
                                        },
                                        CastlingMode::Standard,
                                    ) {
                                        parents.push(prev_pos_cap);
                                    }
                            }
                            
                            // Revert uncapture
                            prev_board.discard_piece_at(src);
                            prev_board.set_piece_at(sq, piece);
                        }

                        // En Passant Uncapture
                        if let Some(captured_sq) = sq.offset(direction)
                            && board.piece_at(src).is_none() && board.piece_at(captured_sq).is_none() {
                                let ep_rank = if us == Color::White { Rank::Sixth } else { Rank::Third };
                                if sq.rank() == ep_rank {
                                    prev_board.discard_piece_at(sq);
                                    prev_board.set_piece_at(src, Piece { role: Role::Pawn, color: us });
                                    prev_board.set_piece_at(captured_sq, Piece { role: Role::Pawn, color: them });

                                    if is_valid_predecessor(&prev_board, us, them)
                                        && let Ok(prev_pos_ep) = Chess::from_setup(
                                            Setup {
                                                board: prev_board.clone(),
                                                promoted: pos.promoted(),
                                                pockets: pos.pockets().copied(),
                                                turn: us,
                                                castling_rights: pos.castles().castling_rights(),
                                                ep_square: Some(sq),
                                                remaining_checks: pos.remaining_checks().copied(),
                                                halfmoves: 0,
                                                fullmoves: if us == Color::Black { pos.fullmoves() } else { NonZeroU32::new(pos.fullmoves().get().saturating_sub(1).max(1)).unwrap() },
                                            },
                                            CastlingMode::Standard,
                                        ) {
                                            parents.push(prev_pos_ep);
                                        }
                                    
                                    // Revert EP
                                    prev_board.discard_piece_at(src);
                                    prev_board.discard_piece_at(captured_sq);
                                    prev_board.set_piece_at(sq, Piece { role: Role::Pawn, color: us });
                                }
                            }
                    }
            }
        } else {
            let attacks = shakmaty::attacks::attacks(sq, Piece { role, color: us }, occupied);

            for src in attacks {
                if board.piece_at(src).is_none() {
                    let piece = Piece { role, color: us };
                    
                    // Quiet unmove
                    prev_board.discard_piece_at(sq);
                    prev_board.set_piece_at(src, piece);

                    if is_valid_predecessor(&prev_board, us, them) {
                        let cr = pos.castles().castling_rights();
                        if let Ok(prev_pos) = Chess::from_setup(
                            Setup {
                                board: prev_board.clone(),
                                promoted: pos.promoted(),
                                pockets: pos.pockets().copied(),
                                turn: us,
                                castling_rights: cr,
                                ep_square: None,
                                remaining_checks: pos.remaining_checks().copied(),
                                halfmoves: pos.halfmoves().saturating_sub(1),
                                fullmoves: if us == Color::Black { pos.fullmoves() } else { NonZeroU32::new(pos.fullmoves().get().saturating_sub(1).max(1)).unwrap() },
                            },
                            CastlingMode::Standard,
                        ) {
                            parents.push(prev_pos);
                        }
                    }

                    // Uncaptures
                    for captured_role in [Role::Queen, Role::Rook, Role::Bishop, Role::Knight, Role::Pawn] {
                        if captured_role == Role::Pawn && (sq.rank() == Rank::First || sq.rank() == Rank::Eighth) {
                            continue;
                        }

                        prev_board.set_piece_at(sq, Piece { role: captured_role, color: them });
                        
                        if is_valid_predecessor(&prev_board, us, them) {
                            let cr = pos.castles().castling_rights();
                            if let Ok(prev_pos_cap) = Chess::from_setup(
                                Setup {
                                    board: prev_board.clone(),
                                    promoted: pos.promoted(),
                                    pockets: pos.pockets().copied(),
                                    turn: us,
                                    castling_rights: cr,
                                    ep_square: None,
                                    remaining_checks: pos.remaining_checks().copied(),
                                    halfmoves: 0,
                                    fullmoves: if us == Color::Black { pos.fullmoves() } else { NonZeroU32::new(pos.fullmoves().get().saturating_sub(1).max(1)).unwrap() },
                                },
                                CastlingMode::Standard,
                            ) {
                                parents.push(prev_pos_cap);
                            }
                        }
                    }
                    
                    // Revert to original
                    prev_board.discard_piece_at(src);
                    prev_board.set_piece_at(sq, piece);
                }
            }
        }
    }

    // Un-castling
    let castling_unmoves = if us == Color::White {
        [
            (Square::G1, Square::F1, Square::E1, Square::H1, Role::Rook),
            (Square::C1, Square::D1, Square::E1, Square::A1, Role::Rook),
        ]
    } else {
        [
            (Square::G8, Square::F8, Square::E8, Square::H8, Role::Rook),
            (Square::C8, Square::D8, Square::E8, Square::A8, Role::Rook),
        ]
    };

    for (k_sq, r_sq, k_src, r_src, _) in castling_unmoves {
        if board.role_at(k_sq) == Some(Role::King) && board.color_at(k_sq) == Some(us) &&
           board.role_at(r_sq) == Some(Role::Rook) && board.color_at(r_sq) == Some(us)
            
            && board.piece_at(k_src).is_none() && board.piece_at(r_src).is_none() {
                prev_board.discard_piece_at(k_sq);
                prev_board.discard_piece_at(r_sq);
                prev_board.set_piece_at(k_src, Piece { role: Role::King, color: us });
                prev_board.set_piece_at(r_src, Piece { role: Role::Rook, color: us });

                // For castling to be valid, us king must not be in check, 
                // and the passed square must not be attacked by them.
                // But `from_setup` + `CastlingMode::Standard` does not verify the path of castling historically if we just supply setup.
                // Actually `Chess::from_setup` doesn't know about the forward move being Castling!
                // Wait! If the un-move is castling, the resulting `Chess` will just have castling rights.
                // Does `is_valid_predecessor` ensure castling was legal?
                // The actual legality of castling requires that K_SRC, K_PASS, K_DEST were not attacked.
                // We MUST check if K_SRC or K_PASS are attacked by `them`. (K_DEST is K_SQ, which is in `pos`, so it's not attacked since `us` is not in check in `pos`).
                
                let mut castling_legal = is_valid_predecessor(&prev_board, us, them);
                if castling_legal {
                    // Check if k_src or k_pass (which is r_sq for kingside, or D1 for queenside) is attacked by them
                    // r_sq is F1 or D1.
                    let k_pass = r_sq;
                    
                    let mut attacked = false;
                    for check_sq in [k_src, k_pass] {
                        let occupied = prev_board.occupied();
                        let pawn_attacks = shakmaty::attacks::pawn_attacks(us, check_sq);
                        if (pawn_attacks & prev_board.pawns() & prev_board.by_color(them)).any() { attacked = true; break; }

                        let knight_attacks = shakmaty::attacks::knight_attacks(check_sq);
                        if (knight_attacks & prev_board.knights() & prev_board.by_color(them)).any() { attacked = true; break; }

                        let king_attacks = shakmaty::attacks::king_attacks(check_sq);
                        if (king_attacks & prev_board.kings() & prev_board.by_color(them)).any() { attacked = true; break; }

                        let bishop_attacks = shakmaty::attacks::bishop_attacks(check_sq, occupied);
                        if (bishop_attacks & (prev_board.bishops() | prev_board.queens()) & prev_board.by_color(them)).any() { attacked = true; break; }

                        let rook_attacks = shakmaty::attacks::rook_attacks(check_sq, occupied);
                        if (rook_attacks & (prev_board.rooks() | prev_board.queens()) & prev_board.by_color(them)).any() { attacked = true; break; }
                    }
                    if attacked {
                        castling_legal = false;
                    }
                }

                if castling_legal {
                    let mut new_cr = pos.castles().castling_rights();
                    new_cr.add(r_src);

                    if let Ok(prev_pos) = Chess::from_setup(
                        Setup {
                            board: prev_board.clone(),
                            promoted: pos.promoted(),
                            pockets: pos.pockets().copied(),
                            turn: us,
                            castling_rights: new_cr,
                            ep_square: None,
                            remaining_checks: pos.remaining_checks().copied(),
                            halfmoves: pos.halfmoves().saturating_sub(1),
                            fullmoves: if us == Color::Black { pos.fullmoves() } else { NonZeroU32::new(pos.fullmoves().get().saturating_sub(1).max(1)).unwrap() },
                        },
                        CastlingMode::Standard,
                    ) {
                        parents.push(prev_pos);
                    }
                }
                
                // Revert
                prev_board.discard_piece_at(k_src);
                prev_board.discard_piece_at(r_src);
                prev_board.set_piece_at(k_sq, Piece { role: Role::King, color: us });
                prev_board.set_piece_at(r_sq, Piece { role: Role::Rook, color: us });
            }
    }
}
