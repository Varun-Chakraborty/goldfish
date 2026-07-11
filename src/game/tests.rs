use crate::{
    game::{
        Direction::{Horizontal, LeftDiagonal, RightDiagonal, Vertical},
        GameState,
        MoveGenMode::All,
    },
    notation::parse_algebraic,
    types::{
        Color::{Black, White},
        Coordinate, Move,
        PieceType::{Bishop, King, Knight, Pawn, Queen, Rook},
    },
};

#[test]
fn test_fen() {
    let mut gs =
        GameState::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1").unwrap();
    assert_eq!(
        gs.to_fen(),
        "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1"
    );
    let m = parse_algebraic(&mut gs, "e4").unwrap();
    gs.make_move(m);
    assert_eq!(
        gs.to_fen(),
        "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1"
    );
}

#[test]
fn test_legal_moves() {
    let mut gs =
        GameState::from_fen("4k2r/1p3ppp/p7/2bPp3/5B2/5P2/PP4PP/R2nRrK1 w - - 10 38").unwrap();
    let moves = gs.legal_moves(All);
    assert!(moves.contains(&Move::new_capture(
        Coordinate::new(6, 0),
        Coordinate::new(5, 0),
        King,
        Rook
    )));
}

#[test]
fn test_from_fen_startpos() {
    let gs =
        GameState::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1").unwrap();

    assert_eq!(gs.wp, 0x_00_00_00_00_00_00_FF_00);
    assert_eq!(gs.bp, 0x_00_FF_00_00_00_00_00_00);
    assert_eq!(
        gs.to_fen(),
        "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1"
    );
}

#[test]
fn test_from_fen_position1() {
    let gs = GameState::from_fen("rnb1kbnr/ppp1pppp/5q2/3pP3/8/8/PPPP1PPP/RNBQKBNR w KQkq - 0 1")
        .unwrap();

    assert_eq!(gs.wp, 0x_00_00_00_10_00_00_EF_00);
    assert_eq!(gs.bp, 0x_00_F7_00_08_00_00_00_00);
    assert_eq!(gs.wb, 0x_00_00_00_00_00_00_00_24);
    assert_eq!(gs.bb, 0x_24_00_00_00_00_00_00_00);
    assert_eq!(
        gs.to_fen(),
        "rnb1kbnr/ppp1pppp/5q2/3pP3/8/8/PPPP1PPP/RNBQKBNR w KQkq - 0 1"
    );
}

#[test]
fn test_make_move_startpos() {
    let mut gs =
        GameState::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1").unwrap();
    let m = parse_algebraic(&mut gs, "e4").unwrap();
    let material_count = gs.material_count;
    let undo = gs.make_move(m);
    assert_eq!(gs.wp, 0x_00_00_00_00_10_00_EF_00);
    gs.unmake_move(undo);
    assert_eq!(gs.wp, 0x_00_00_00_00_00_00_FF_00);
    assert_eq!(gs.material_count, material_count);
}

#[test]
fn test_make_move_capture() {
    let mut gs =
        GameState::from_fen("rnb1kbnr/ppp1pppp/5b2/3pP3/8/8/PPPP1PPP/RNBQKBNR w KQkq d6 0 1")
            .unwrap();
    let m = parse_algebraic(&mut gs, "exd6").unwrap();
    let undo = gs.make_move(m);
    assert_eq!(gs.wp, 0x_00_00_08_00_00_00_EF_00);
    assert_eq!(gs.bp, 0x_00_F7_00_00_00_00_00_00);
    assert_eq!(gs.bb, 0x_24_00_20_00_00_00_00_00);
    gs.unmake_move(undo);
    assert_eq!(gs.wp, 0x_00_00_00_10_00_00_EF_00);
    assert_eq!(gs.bp, 0x_00_F7_00_08_00_00_00_00);
    assert_eq!(gs.bb, 0x_24_00_20_00_00_00_00_00);
    let m = parse_algebraic(&mut gs, "exf6").unwrap();
    let undo = gs.make_move(m);
    assert_eq!(gs.wp, 0x_00_00_20_00_00_00_EF_00);
    assert_eq!(gs.bp, 0x_00_F7_00_08_00_00_00_00);
    assert_eq!(gs.bb, 0x_24_00_00_00_00_00_00_00);
    gs.unmake_move(undo);
    assert_eq!(gs.wp, 0x_00_00_00_10_00_00_EF_00);
    assert_eq!(gs.bp, 0x_00_F7_00_08_00_00_00_00);
    assert_eq!(gs.bb, 0x_24_00_20_00_00_00_00_00);
}

#[test]
fn test_unmake_move_kingside_castle() {
    let mut gs =
        GameState::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQK2R w KQkq - 0 1").unwrap();
    let m = parse_algebraic(&mut gs, "o-o").unwrap();
    let material_count = gs.material_count;
    let undo = gs.make_move(m);
    assert_eq!(
        gs.to_fen(),
        "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQ1RK1 b kq - 0 1"
    );
    gs.unmake_move(undo);
    assert_eq!(
        gs.to_fen(),
        "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQK2R w KQkq - 0 1"
    );
    assert_eq!(gs.material_count, material_count);
}

#[test]
fn test_unmake_move_queenside_castle() {
    let mut gs =
        GameState::from_fen("r3kbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR b KQkq - 0 1").unwrap();
    let m = parse_algebraic(&mut gs, "o-o-o").unwrap();
    let material_count = gs.material_count;
    let undo = gs.make_move(m);
    assert_eq!(
        gs.to_fen(),
        "2kr1bnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQ - 0 2"
    );
    gs.unmake_move(undo);
    assert_eq!(
        gs.to_fen(),
        "r3kbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR b KQkq - 0 1"
    );
    assert_eq!(gs.material_count, material_count);
}

#[test]
fn test_unmake_move_rook_move_kingside() {
    let mut gs =
        GameState::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQK2R w KQkq - 0 1").unwrap();
    let m = parse_algebraic(&mut gs, "Rg1").unwrap();
    let material_count = gs.material_count;
    let undo = gs.make_move(m);
    assert_eq!(
        gs.to_fen(),
        "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQK1R1 b Qkq - 1 1"
    );
    gs.unmake_move(undo);
    assert_eq!(
        gs.to_fen(),
        "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQK2R w KQkq - 0 1"
    );
    assert_eq!(gs.material_count, material_count);
}

#[test]
fn test_unmake_move_rook_move_queenside() {
    let mut gs =
        GameState::from_fen("r3kbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR b KQkq - 0 1").unwrap();
    let m = parse_algebraic(&mut gs, "Rb8").unwrap();
    let material_count = gs.material_count;
    let undo = gs.make_move(m);
    assert_eq!(
        gs.to_fen(),
        "1r2kbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQk - 1 2"
    );
    gs.unmake_move(undo);
    assert_eq!(
        gs.to_fen(),
        "r3kbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR b KQkq - 0 1"
    );
    assert_eq!(gs.material_count, material_count);
}

#[test]
fn test_unmake_move_capture() {
    let mut gs =
        GameState::from_fen("rnbqkbnr/ppp1pppp/8/3p4/4P3/8/PPPP1PPP/RNBQKBNR w KQkq - 0 1")
            .unwrap();
    let m = parse_algebraic(&mut gs, "exd5").unwrap();
    let material_count = gs.material_count;
    let undo = gs.make_move(m);
    assert_eq!(
        gs.to_fen(),
        "rnbqkbnr/ppp1pppp/8/3P4/8/8/PPPP1PPP/RNBQKBNR b KQkq - 0 1"
    );
    gs.unmake_move(undo);
    assert_eq!(
        gs.to_fen(),
        "rnbqkbnr/ppp1pppp/8/3p4/4P3/8/PPPP1PPP/RNBQKBNR w KQkq - 0 1"
    );
    assert_eq!(gs.material_count, material_count);
}

#[test]
fn test_unmake_move_promotion() {
    let mut gs = GameState::from_fen("8/7P/8/8/8/8/8/K1k5 w - - 0 1").unwrap();
    let m = parse_algebraic(&mut gs, "h8=Q").unwrap();
    let material_count = gs.material_count;
    let undo = gs.make_move(m);
    assert_eq!(gs.to_fen(), "7Q/8/8/8/8/8/8/K1k5 b - - 0 1");
    gs.unmake_move(undo);
    assert_eq!(gs.to_fen(), "8/7P/8/8/8/8/8/K1k5 w - - 0 1");
    assert_eq!(gs.material_count, material_count);
}

#[test]
fn test_unmake_move_promotion_capture() {
    let mut gs = GameState::from_fen("6r1/7P/8/8/8/8/8/K1k5 w - - 0 1").unwrap();
    let m = parse_algebraic(&mut gs, "hxg8=Q").unwrap();
    let material_count = gs.material_count;
    let undo = gs.make_move(m);
    assert_eq!(gs.to_fen(), "6Q1/8/8/8/8/8/8/K1k5 b - - 0 1");
    gs.unmake_move(undo);
    assert_eq!(gs.to_fen(), "6r1/7P/8/8/8/8/8/K1k5 w - - 0 1");
    assert_eq!(gs.material_count, material_count);
}

#[test]
fn test_unmake_move_enpassant() {
    let mut gs =
        GameState::from_fen("rnbqkbnr/ppp1pppp/8/3pP3/8/8/PPPP1PPP/RNBQKBNR w KQkq d6 0 1")
            .unwrap();
    let m = parse_algebraic(&mut gs, "exd6").unwrap();
    let material_count = gs.material_count;
    let undo = gs.make_move(m);
    assert_eq!(
        gs.to_fen(),
        "rnbqkbnr/ppp1pppp/3P4/8/8/8/PPPP1PPP/RNBQKBNR b KQkq - 0 1"
    );
    gs.unmake_move(undo);
    assert_eq!(
        gs.to_fen(),
        "rnbqkbnr/ppp1pppp/8/3pP3/8/8/PPPP1PPP/RNBQKBNR w KQkq d6 0 1"
    );
    assert_eq!(gs.material_count, material_count);
}

#[test]
fn test_count_material_capture_promotion() {
    let mut gs = GameState::from_fen("7r/6P1/8/8/8/8/8/k1K5 w KQkq - 0 1").unwrap();
    let m = parse_algebraic(&mut gs, "gxh8=Q+").unwrap();
    let undo = gs.make_move(m);
    let material_count = gs.material_count;
    assert_eq!(material_count[gs.material_idx(Queen, White)], 1);
    assert_eq!(material_count[gs.material_idx(Queen, Black)], 0);
    assert_eq!(material_count[gs.material_idx(Rook, White)], 0);
    assert_eq!(material_count[gs.material_idx(Rook, Black)], 0);
    assert_eq!(material_count[gs.material_idx(Bishop, White)], 0);
    assert_eq!(material_count[gs.material_idx(Bishop, Black)], 0);
    assert_eq!(material_count[gs.material_idx(Knight, White)], 0);
    assert_eq!(material_count[gs.material_idx(Knight, Black)], 0);
    assert_eq!(material_count[gs.material_idx(Pawn, Black)], 0);
    assert_eq!(material_count[gs.material_idx(Pawn, White)], 0);
    gs.unmake_move(undo);

    let material_count = gs.material_count;
    assert_eq!(material_count[gs.material_idx(Queen, White)], 0);
    assert_eq!(material_count[gs.material_idx(Queen, Black)], 0);
    assert_eq!(material_count[gs.material_idx(Rook, White)], 0);
    assert_eq!(material_count[gs.material_idx(Rook, Black)], 1);
    assert_eq!(material_count[gs.material_idx(Bishop, White)], 0);
    assert_eq!(material_count[gs.material_idx(Bishop, Black)], 0);
    assert_eq!(material_count[gs.material_idx(Knight, White)], 0);
    assert_eq!(material_count[gs.material_idx(Knight, Black)], 0);
    assert_eq!(material_count[gs.material_idx(Pawn, White)], 1);
    assert_eq!(material_count[gs.material_idx(Pawn, Black)], 0);
}

#[test]
fn test_count_material_en_passant() {
    let mut gs = GameState::from_fen("8/8/8/3pP3/8/8/8/k1K5 w KQkq d6 0 1").unwrap();
    let m = parse_algebraic(&mut gs, "exd6").unwrap();
    let undo = gs.make_move(m);
    let material_count = gs.material_count;
    assert_eq!(material_count[gs.material_idx(Queen, White)], 0);
    assert_eq!(material_count[gs.material_idx(Queen, Black)], 0);
    assert_eq!(material_count[gs.material_idx(Rook, White)], 0);
    assert_eq!(material_count[gs.material_idx(Rook, Black)], 0);
    assert_eq!(material_count[gs.material_idx(Bishop, White)], 0);
    assert_eq!(material_count[gs.material_idx(Bishop, Black)], 0);
    assert_eq!(material_count[gs.material_idx(Knight, White)], 0);
    assert_eq!(material_count[gs.material_idx(Knight, Black)], 0);
    assert_eq!(material_count[gs.material_idx(Pawn, Black)], 0);
    assert_eq!(material_count[gs.material_idx(Pawn, White)], 1);
    gs.unmake_move(undo);

    let material_count = gs.material_count;
    assert_eq!(material_count[gs.material_idx(Queen, White)], 0);
    assert_eq!(material_count[gs.material_idx(Queen, Black)], 0);
    assert_eq!(material_count[gs.material_idx(Rook, White)], 0);
    assert_eq!(material_count[gs.material_idx(Rook, Black)], 0);
    assert_eq!(material_count[gs.material_idx(Bishop, White)], 0);
    assert_eq!(material_count[gs.material_idx(Bishop, Black)], 0);
    assert_eq!(material_count[gs.material_idx(Knight, White)], 0);
    assert_eq!(material_count[gs.material_idx(Knight, Black)], 0);
    assert_eq!(material_count[gs.material_idx(Pawn, White)], 1);
    assert_eq!(material_count[gs.material_idx(Pawn, Black)], 1);
}

#[test]
fn test_pinned_direction1() {
    let gs = GameState::from_fen("7k/8/8/4r3/8/8/8/QK6 b - - 0 1").unwrap();
    let pin_map = gs.compute_pins();
    let pinned = pin_map.get(Coordinate::new(4, 4));
    println!("{:?}", pinned);
    assert!(pinned.is_some_and(|dir| dir == RightDiagonal));
}

#[test]
fn test_pinned_direction2() {
    let gs = GameState::from_fen("k7/8/8/3r4/8/8/8/6KQ b - - 0 1").unwrap();
    let pin_map = gs.compute_pins();
    let pinned = pin_map.get(Coordinate::new(3, 4));
    println!("{:?}", pinned);
    assert!(pinned.is_some_and(|dir| dir == LeftDiagonal));
}

#[test]
fn test_pinned_direction3() {
    let gs = GameState::from_fen("7k/8/8/7r/8/8/8/6KQ b - - 0 1").unwrap();
    let pin_map = gs.compute_pins();
    let pinned = pin_map.get(Coordinate::new(7, 4));
    println!("{:?}", pinned);
    assert!(pinned.is_some_and(|dir| dir == Vertical));
}

#[test]
fn test_pinned_direction4() {
    let gs = GameState::from_fen("KQ3r1k/8/8/8/8/8/8/8 b - - 0 1").unwrap();
    let pin_map = gs.compute_pins();
    let pinned = pin_map.get(Coordinate::new(5, 7));
    println!("{:?}", pinned);
    assert!(pinned.is_some_and(|dir| dir == Horizontal));
}
