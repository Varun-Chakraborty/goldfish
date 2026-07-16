mod board;
mod eval;
mod game;
mod notation;
mod perft;
mod position;
mod search;
mod transposition;
mod types;
mod zobrist;

use std::sync::{Arc, atomic::AtomicBool, mpsc};
use thiserror::Error;

use crate::{game::GameStateError, notation::parse_algebraic, search::search};

pub use crate::{
    game::GameState,
    notation::fmt_pgn_moves,
    search::{EngineEvent, EngineLimits, Score},
};

pub enum EngineCommand {
    Init {
        fen: String,
        moves: Option<Vec<String>>,
    },
    Start {
        limits: EngineLimits,
    },
    Quit,
}

pub enum UCIMessage {
    Command(String),
    Event(EngineEvent),
}

pub struct EngineWorker {
    engine: GoldFish,
    cmd_receiver: mpsc::Receiver<EngineCommand>,
    event_sender: mpsc::Sender<UCIMessage>,
    stop: Arc<AtomicBool>,
}

impl EngineWorker {
    pub fn new(
        cmd_receiver: mpsc::Receiver<EngineCommand>,
        event_sender: mpsc::Sender<UCIMessage>,
        stop: Arc<AtomicBool>,
    ) -> Self {
        Self {
            engine: GoldFish::new_engine(),
            cmd_receiver,
            event_sender,
            stop,
        }
    }

    pub fn run(&mut self) {
        loop {
            match self.cmd_receiver.recv() {
                Ok(cmd) => match cmd {
                    EngineCommand::Init { fen, moves } => {
                        if let Err(e) = self.engine.new_position_from_fen(&fen) {
                            eprintln!("Init error: {e}");
                            continue;
                        }
                        if let Some(moves) = moves {
                            for m in moves {
                                if let Err(e) = self.engine.make_move(m) {
                                    eprintln!("Move error: {e}");
                                    break;
                                }
                            }
                        }
                    }
                    EngineCommand::Start { limits } => {
                        let result = self.engine.search(limits, &self.stop, |event| {
                            if self.event_sender.send(UCIMessage::Event(event)).is_err() {
                                eprintln!("Failed to send event");
                            }
                        });
                        if let Err(e) = result {
                            eprintln!("Search error: {e}");
                        }
                    }
                    EngineCommand::Quit => break,
                },
                Err(e) => {
                    eprintln!("Command channel error: {e}");
                    break;
                }
            }
        }
    }
}

#[derive(Error, Debug)]
enum GoldFishError {
    #[error("GameStateError: {0}")]
    GameStateError(#[from] GameStateError),
    #[error("Initialize the board first")]
    GameStateNotInitialised,
    #[error("Invalid move: {0}")]
    InvalidMove(String),
}

struct GoldFish {
    gamestate: Option<GameState>,
    tt: Option<transposition::TranspositionTable>
}

impl GoldFish {
    fn new_engine() -> Self {
        Self { gamestate: None, tt: None }
    }

    fn new_position_from_fen(&mut self, fen: &str) -> Result<(), GoldFishError> {
        self.gamestate = Some(GameState::from_fen(fen)?);
        Ok(())
    }

    fn search<F>(
        &mut self,
        limits: EngineLimits,
        stop: &Arc<AtomicBool>,
        callback: F,
    ) -> Result<(), GoldFishError>
    where
        F: FnMut(EngineEvent),
    {
        match &mut self.gamestate {
            Some(gs) => search(gs, limits, stop, &mut self.tt, callback),
            None => return Err(GoldFishError::GameStateNotInitialised),
        };
        Ok(())
    }

    fn make_move(&mut self, m: String) -> Result<(), GoldFishError> {
        match &mut self.gamestate {
            Some(gs) => match parse_algebraic(gs, &m) {
                Some(m) => gs.make_move(m),
                None => return Err(GoldFishError::InvalidMove(m)),
            },
            None => return Err(GoldFishError::GameStateNotInitialised),
        };
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        game::{GameState, MoveGenMode},
        notation::{fmt_pgn_moves, parse_algebraic},
        types::*,
    };

    #[test]
    fn test_starting_position() {
        let gs = GameState::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1")
            .unwrap();
        assert_eq!(gs.turn, Color::White);
        assert_eq!(
            gs.board.get(Coordinate::new_coordinate(0, 0)),
            (Square::Occupied {
                color: Color::White,
                piece: PieceType::Rook
            })
        );
        assert_eq!(
            gs.board.get(Coordinate::new_coordinate(4, 0)),
            (Square::Occupied {
                color: Color::White,
                piece: PieceType::King
            })
        );
        assert_eq!(
            gs.board.get(Coordinate::new_coordinate(4, 7)),
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
        assert_eq!(m.from, Coordinate::new_coordinate(4, 1));
        assert_eq!(m.to, Coordinate::new_coordinate(4, 3));
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
            gs.board.get(Coordinate::new_coordinate(4, 3)),
            (Square::Occupied {
                color: Color::White,
                piece: PieceType::Pawn
            })
        );
        assert_eq!(
            gs.board.get(Coordinate::new_coordinate(4, 1)),
            Square::Empty
        );
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
            gs.board.get(Coordinate::new_coordinate(5, 2)),
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
            gs.board.get(Coordinate::new_coordinate(5, 5)),
            (Square::Occupied {
                color: Color::White,
                piece: PieceType::Pawn
            })
        );
        assert_eq!(
            gs.board.get(Coordinate::new_coordinate(5, 4)),
            Square::Empty
        );
    }

    #[test]
    fn test_disambiguation() {
        // Two white rooks that can both go to d5: one on d1, one on a5
        // Position: K on e1, rooks on d1 and a5, black K on e8
        let mut gs = GameState::from_fen("4k3/8/8/R2b4/8/8/8/3RK3 w - - 0 1").unwrap();
        let m = Move {
            from: Coordinate::new_coordinate(3, 0),
            to: Coordinate::new_coordinate(3, 4),
            promotion: None,
            en_passant: false,
            captured: Some(PieceType::Bishop),
            piece: PieceType::Rook,
            castle: None,
        };
        let m = &fmt_pgn_moves(&mut gs, &[m])[0];
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
        assert!(gs.in_check(Color::Black));
        assert!(gs.legal_moves(MoveGenMode::All).is_empty());
    }
}
