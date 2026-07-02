use std::{
    sync::{Arc, atomic::AtomicBool},
    time::Instant,
};

use thiserror::Error;

use crate::{
    EngineEvent, EngineLimits, GameState,
    clock::Clock,
    game::{GameStateError, MoveGenMode::All},
    history::HistoryHeuristic,
    notation::parse_algebraic,
    position::{Position, PositionStatus},
    search::iterative_deepening,
    transposition::TranspositionTable,
    zobrist::compute_hash,
};

#[derive(Error, Debug)]
pub enum GoldFishError {
    #[error("GameStateError: {0}")]
    GameStateError(#[from] GameStateError),
    #[error("Invalid move: {0}")]
    InvalidMove(String),
}

const STARTPOS: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

pub struct GoldFish {
    gamestate: GameState,
    tt: Option<TranspositionTable>,
    history: Option<HistoryHeuristic>,
    clock: Clock,
}

impl GoldFish {
    pub fn new() -> Result<Self, GoldFishError> {
        Ok(Self {
            gamestate: GameState::from_fen(STARTPOS)?,
            tt: None,
            history: None,
            clock: Clock::default(),
        })
    }

    pub fn set_option(&mut self, name: &str, value: &str) {
        match name {
            "Hash" => {
                let hash = value.parse().unwrap();
                self.tt = Some(TranspositionTable::new(hash));
            }
            _ => {}
        }
    }

    pub fn new_position_from_fen(&mut self, fen: &str) -> Result<(), GoldFishError> {
        match fen {
            "startpos" => self.gamestate = GameState::from_fen(STARTPOS)?,
            fen => self.gamestate = GameState::from_fen(fen)?,
        }
        Ok(())
    }

    pub fn new_game(&mut self) -> Result<(), GoldFishError> {
        self.history = None;
        self.tt = None;
        Ok(())
    }

    pub fn search<F>(
        &mut self,
        limits: EngineLimits,
        stop: &Arc<AtomicBool>,
        ponderhit: &Arc<AtomicBool>,
        callback: F,
    ) -> Result<(), GoldFishError>
    where
        F: FnMut(EngineEvent),
    {
        let gs = &mut self.gamestate;
        self.clock.load_clock(gs.turn, &limits);
        iterative_deepening(
            gs,
            limits,
            stop,
            ponderhit,
            &mut self.tt,
            &mut self.history,
            &mut self.clock,
            callback,
        );
        Ok(())
    }

    pub fn make_move(&mut self, m: String) -> Result<(), GoldFishError> {
        let gs = &mut self.gamestate;
        match parse_algebraic(gs, &m) {
            Some(m) => gs.make_move(m),
            None => return Err(GoldFishError::InvalidMove(m)),
        };
        Ok(())
    }

    pub fn debug<F>(&self, mut callback: F) -> Result<(), GoldFishError>
    where
        F: FnMut(EngineEvent),
    {
        let gs = &self.gamestate;
        let fen = gs.to_fen();
        callback(EngineEvent::Debug {
            string: format!(""),
        });
        callback(EngineEvent::Debug {
            string: format!("{}", gs.representation()),
        });
        callback(EngineEvent::Debug {
            string: format!("Fen: {fen}"),
        });
        callback(EngineEvent::Debug {
            string: format!("Key: {:0X}", gs.zobrist),
        });
        callback(EngineEvent::Debug {
            string: format!(
                "Checkers: {}",
                gs.checks(gs.turn)
                    .iter()
                    .flatten()
                    .map(|c| c.0.to_algebraic())
                    .collect::<Vec<_>>()
                    .join(" ")
            ),
        });
        Ok(())
    }

    fn perft(gs: &mut GameState, depth: u32) -> u64 {
        if depth == 0 {
            return 1;
        }
        let mut count = 0;

        let legal = gs.legal_moves(All);

        for m in legal {
            let undo = gs.make_move(m);
            debug_assert_eq!(gs.zobrist, compute_hash(gs));
            count += GoldFish::perft(gs, depth - 1);
            gs.unmake_move(undo);
        }
        count
    }

    pub fn divide<F>(&mut self, depth: u32, mut callback: F) -> Result<(), GoldFishError>
    where
        F: FnMut(EngineEvent),
    {
        let gs = &mut self.gamestate;
        let position = Position::analyse(gs);

        match position.status {
            PositionStatus::Ongoing => (),
            _ => {
                callback(EngineEvent::Debug {
                    string: ": 0".to_string(),
                });
                return Ok(());
            }
        }

        if depth == 0 {
            callback(EngineEvent::Debug {
                string: ": 0".to_string(),
            });
            return Ok(());
        }

        let legal = position.legal_moves.expect("No legal moves");
        let mut t_nodes = 0;
        let now = Instant::now();

        for m in legal {
            let undo = gs.make_move(m);
            let m = m.to_algebraic();
            let nodes = Self::perft(gs, depth - 1);

            callback(EngineEvent::Debug {
                string: format!("{}: {}", m, nodes),
            });

            gs.unmake_move(undo);
            t_nodes += nodes;
        }

        callback(EngineEvent::Debug {
            string: format!(
                "Nodes searched: {} took {:.2}s",
                t_nodes,
                now.elapsed().as_secs_f32()
            ),
        });

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        engine::GoldFish,
        game::{GameState, MoveGenMode},
        notation::{fmt_pgn_moves, parse_algebraic},
        types::*,
    };

    #[test]
    fn test_perft_start_position() {
        let mut gs =
            GameState::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1")
                .unwrap();

        assert_eq!(GoldFish::perft(&mut gs, 1), 20);
        assert_eq!(GoldFish::perft(&mut gs, 2), 400);
        assert_eq!(GoldFish::perft(&mut gs, 3), 8902);
        assert_eq!(GoldFish::perft(&mut gs, 4), 197281);
    }

    #[test]
    fn test_perft_kiwipete() {
        let mut gs = GameState::from_fen(
            "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
        )
        .unwrap();

        assert_eq!(GoldFish::perft(&mut gs, 1), 48);
        assert_eq!(GoldFish::perft(&mut gs, 2), 2039);
        assert_eq!(GoldFish::perft(&mut gs, 3), 97862);
    }

    #[test]
    fn test_perft_en_passant() {
        let mut gs = GameState::from_fen("8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1").unwrap();
        println!("{}", gs.representation());
        assert_eq!(GoldFish::perft(&mut gs, 1), 14);
        assert_eq!(GoldFish::perft(&mut gs, 2), 191);
        assert_eq!(GoldFish::perft(&mut gs, 3), 2812);
        assert_eq!(GoldFish::perft(&mut gs, 4), 43238);
    }

    #[test]
    fn test_perft_position1() {
        let mut gs =
            GameState::from_fen("rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8")
                .unwrap();
        assert_eq!(GoldFish::perft(&mut gs, 1), 44);
        assert_eq!(GoldFish::perft(&mut gs, 2), 1486);
        assert_eq!(GoldFish::perft(&mut gs, 3), 62379);
        assert_eq!(GoldFish::perft(&mut gs, 4), 2103487);
    }

    #[test]
    fn test_starting_position() {
        let gs = GameState::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1")
            .unwrap();
        assert_eq!(gs.turn, Color::White);
        assert_eq!(
            gs.board.get(Coordinate::new(0, 0)),
            (Square::Occupied {
                color: Color::White,
                piece: PieceType::Rook
            })
        );
        assert_eq!(
            gs.board.get(Coordinate::new(4, 0)),
            (Square::Occupied {
                color: Color::White,
                piece: PieceType::King
            })
        );
        assert_eq!(
            gs.board.get(Coordinate::new(4, 7)),
            (Square::Occupied {
                color: Color::Black,
                piece: PieceType::King
            })
        );
    }

    #[test]
    fn test_initial_legal_moves() {
        let mut gs =
            GameState::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1")
                .unwrap();
        let legal = gs.legal_moves(MoveGenMode::All);
        assert_eq!(legal.len(), 20);
    }

    #[test]
    fn test_algebraic_parsing() {
        let mut gs =
            GameState::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1")
                .unwrap();
        let m = parse_algebraic(&mut gs, "e2e4").unwrap();
        assert_eq!(m.from, Coordinate::new(4, 1));
        assert_eq!(m.to, Coordinate::new(4, 3));
    }

    #[test]
    fn test_fen_output() {
        let gs = GameState::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1")
            .unwrap();
        assert!(
            gs.to_fen()
                .starts_with("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w")
        );
    }

    #[test]
    fn test_e2e4() {
        let mut gs =
            GameState::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1")
                .unwrap();
        let m = parse_algebraic(&mut gs, "e2e4").unwrap();
        gs.make_move(m);
        assert_eq!(
            gs.board.get(Coordinate::new(4, 3)),
            (Square::Occupied {
                color: Color::White,
                piece: PieceType::Pawn
            })
        );
        assert_eq!(gs.board.get(Coordinate::new(4, 1)), Square::Empty);
        assert_eq!(gs.turn, Color::Black);
    }

    #[test]
    fn test_knight_moves() {
        let mut gs =
            GameState::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1")
                .unwrap();
        let m = parse_algebraic(&mut gs, "g1f3").unwrap();
        gs.make_move(m);
        assert_eq!(
            gs.board.get(Coordinate::new(5, 2)),
            (Square::Occupied {
                color: Color::White,
                piece: PieceType::Knight
            })
        );
    }

    #[test]
    fn test_en_passant() {
        let mut gs =
            GameState::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1")
                .unwrap();
        for mv_str in &["e2e4", "d7d5", "e4e5", "f7f5"] {
            let m = parse_algebraic(&mut gs, mv_str).unwrap();
            gs.make_move(m);
        }
        let m = parse_algebraic(&mut gs, "e5f6").unwrap();
        assert!(m.en_passant);
        gs.make_move(m);
        assert_eq!(
            gs.board.get(Coordinate::new(5, 5)),
            (Square::Occupied {
                color: Color::White,
                piece: PieceType::Pawn
            })
        );
        assert_eq!(gs.board.get(Coordinate::new(5, 4)), Square::Empty);
    }

    #[test]
    fn test_disambiguation() {
        // Two white rooks that can both go to d5: one on d1, one on a5
        // Position: K on e1, rooks on d1 and a5, black K on e8
        let gs = GameState::from_fen("4k3/8/8/R2b4/8/8/8/3RK3 w - - 0 1").unwrap();
        let m = Move {
            from: Coordinate::new(3, 0),
            to: Coordinate::new(3, 4),
            promotion: None,
            en_passant: false,
            captured: Some(PieceType::Bishop),
            piece: PieceType::Rook,
            castle: None,
        };
        let m = &fmt_pgn_moves(&gs, &[m])[0];
        assert_eq!(m, "Rdxd5");
    }

    #[test]
    fn test_scholars_mate() {
        let mut gs =
            GameState::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1")
                .unwrap();
        let moves = ["e2e4", "e7e5", "f1c4", "b8c6", "d1h5", "g8f6", "h5f7"];
        for mv_str in &moves {
            let m = parse_algebraic(&mut gs, mv_str).unwrap();
            gs.make_move(m);
        }
        assert!(gs.in_check(gs.turn));
        assert!(gs.legal_moves(MoveGenMode::All).is_empty());
    }
}
