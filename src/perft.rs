use crate::{
    GameState,
    notation::fmt_pgn_moves,
    game::MoveGenMode,
    zobrist::compute_hash,
};

#[allow(dead_code)]
pub fn perft(gs: &mut GameState, depth: u32) -> u64 {
    if depth == 0 {
        return 1;
    }
    let mut count = 0;

    let legal = gs.legal_moves(MoveGenMode::All);

    for m in legal {
        let undo = gs.make_move(m);
        debug_assert_eq!(gs.zobrist, compute_hash(gs));
        count += perft(gs, depth - 1);
        gs.unmake_move(undo);
    }
    count
}

#[allow(dead_code)]
pub fn divide(gs: &mut GameState, depth: u32) {
    if depth == 0 {
        println!(": 0");
        return;
    }

    let legal = gs.legal_moves(MoveGenMode::All);

    for m in legal {
        let san = &fmt_pgn_moves(gs, &[m])[0];
        let undo = gs.make_move(m);

        println!("{}: {}", san, perft(gs, depth - 1));

        gs.unmake_move(undo);
    }
}

#[cfg(test)]
mod tests {
    use crate::{game::GameState, perft::perft};

    #[test]
    fn test_perft_start_position() {
        let mut gs =
            GameState::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1")
                .unwrap();

        assert_eq!(perft(&mut gs, 1), 20);
        assert_eq!(perft(&mut gs, 2), 400);
        assert_eq!(perft(&mut gs, 3), 8902);
        assert_eq!(perft(&mut gs, 4), 197281);
    }

    #[test]
    fn test_perft_kiwipete() {
        let mut gs = GameState::from_fen(
            "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
        )
        .unwrap();

        assert_eq!(perft(&mut gs, 1), 48);
        assert_eq!(perft(&mut gs, 2), 2039);
        assert_eq!(perft(&mut gs, 3), 97862);
    }

    #[test]
    fn test_perft_en_passant() {
        let mut gs = GameState::from_fen("8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1").unwrap();
        assert_eq!(perft(&mut gs, 1), 14);
        assert_eq!(perft(&mut gs, 2), 191);
        assert_eq!(perft(&mut gs, 3), 2812);
        assert_eq!(perft(&mut gs, 4), 43238);
    }

    #[test]
    fn test_perft_position1() {
        let mut gs =
            GameState::from_fen("rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8")
                .unwrap();
        assert_eq!(perft(&mut gs, 1), 44);
        assert_eq!(perft(&mut gs, 2), 1486);
        assert_eq!(perft(&mut gs, 3), 62379);
        assert_eq!(perft(&mut gs, 4), 2103487);
    }
}
