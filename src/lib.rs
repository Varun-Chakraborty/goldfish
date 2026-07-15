mod board;
mod clock;
mod engine;
mod game;
mod history;
mod notation;
mod opening_book;
mod position;
mod search;
mod transposition;
mod types;
mod zobrist;

use std::sync::{Arc, atomic::AtomicBool, mpsc};

use crate::{engine::GoldFish, search::search_types::IterationInfo, types::Move};
pub use crate::{
    game::GameState,
    notation::fmt_pgn_moves,
    search::search_types::{EngineLimits, Score},
};

pub enum EngineCommand {
    NewGame,
    Init {
        fen: String,
        moves: Option<Vec<String>>,
    },
    SetOption {
        name: String,
        value: String,
    },
    Start {
        limits: EngineLimits,
    },
    Perft {
        depth: u32,
    },
    Debug,
    Quit,
}

pub enum EngineEvent {
    IterationInfo(IterationInfo),
    SearchFinished {
        best_move: Option<Move>,
        ponder: Option<Move>,
    },
    CurrentMove {
        depth: u32,
        move_: Move,
        number: usize,
    },
    Debug {
        string: String,
    },
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
    ponderhit: Arc<AtomicBool>,
}

impl EngineWorker {
    pub fn new(
        cmd_receiver: mpsc::Receiver<EngineCommand>,
        event_sender: mpsc::Sender<UCIMessage>,
        stop: Arc<AtomicBool>,
        ponderhit: Arc<AtomicBool>,
    ) -> Self {
        let engine = match GoldFish::new() {
            Ok(engine) => engine,
            Err(e) => {
                eprintln!("Error creating engine: {e}");
                std::process::exit(1);
            }
        };
        Self {
            engine,
            cmd_receiver,
            event_sender,
            stop,
            ponderhit,
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
                    EngineCommand::NewGame => {
                        if let Err(e) = self.engine.new_game() {
                            eprintln!("New game error: {e}");
                        }
                    }
                    EngineCommand::SetOption { name, value } => {
                        if let Err(e) = self.engine.set_option(&name, &value) {
                            eprintln!("Set option error: {e}");
                        }
                    }
                    EngineCommand::Start { limits } => {
                        let result =
                            self.engine
                                .search(limits, &self.stop, &self.ponderhit, |event| {
                                    if let Err(e) = self.event_sender.send(UCIMessage::Event(event))
                                    {
                                        eprintln!("Failed to send event: {e}");
                                    }
                                });
                        if let Err(e) = result {
                            eprintln!("Search error: {e}");
                        }
                    }
                    EngineCommand::Perft { depth } => {
                        let result = self.engine.divide(depth, |event| {
                            if let Err(e) = self.event_sender.send(UCIMessage::Event(event)) {
                                eprintln!("Failed to send perft result: {e}");
                            }
                        });
                        if let Err(e) = result {
                            eprintln!("Perft error: {e}");
                        }
                    }
                    EngineCommand::Debug => {
                        let result = self.engine.debug(|event| {
                            if let Err(e) = self.event_sender.send(UCIMessage::Event(event)) {
                                eprintln!("Failed to send debug event: {e}");
                            }
                        });
                        if let Err(e) = result {
                            eprintln!("Engine error: {e}")
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
