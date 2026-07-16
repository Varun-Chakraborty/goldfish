use std::{
    io::{self, BufRead},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread,
};

use goldfish::{EngineCommand, EngineEvent, EngineLimits, EngineWorker, Score, UCIMessage};

fn main() {
    let (cmd_sender, cmd_receiver) = mpsc::channel();
    let (ucimsg_sender, ucimsg_receiver) = mpsc::channel();
    let stop = Arc::new(AtomicBool::new(false));

    let mut worker = EngineWorker::new(cmd_receiver, ucimsg_sender.clone(), stop.clone());

    thread::spawn(move || worker.run());
    thread::spawn(move || {
        let mut line = String::new();
        let mut stdin_lock = io::stdin().lock();
        loop {
            line.clear();
            match stdin_lock.read_line(&mut line) {
                Err(_) | Ok(0) => break,
                Ok(_) => {}
            }
            if line.trim().is_empty() {
                continue;
            }
            if ucimsg_sender
                .send(UCIMessage::Command(line.trim().to_string()))
                .is_err()
            {
                break;
            }
        }
    });

    println!("Goldfish Chess Engine");

    loop {
        match ucimsg_receiver.recv() {
            Ok(msg) => match msg {
                UCIMessage::Command(line) => {
                    let mut args = line.split_whitespace();

                    match args.next() {
                        Some("uci") => {
                            println!("id name GoldFish\nid author Varun\nuciok");
                        }
                        Some("isready") => println!("readyok"),
                        Some("q") | Some("quit") => {
                            if let Err(e) = cmd_sender.send(EngineCommand::Quit) {
                                println!("{e}");
                            }
                            println!("Goodbye!");
                            break;
                        }
                        Some("position") => match args.next() {
                            Some("startpos") => {
                                if let Err(e) = cmd_sender.send(EngineCommand::Init {
                                    fen: "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1"
                                        .to_string(),
                                    moves: None,
                                }) {
                                    println!("{e}");
                                }
                            }
                            Some("fen") => {
                                let fen = args.by_ref().take(6).collect::<Vec<_>>().join(" ");
                                if args.next() == Some("moves") {
                                    let moves = args.map(|m| m.to_string()).collect();
                                    if let Err(e) = cmd_sender.send(EngineCommand::Init {
                                        fen,
                                        moves: Some(moves),
                                    }) {
                                        println!("{e}");
                                    }
                                } else if let Err(e) = cmd_sender.send(EngineCommand::Init {
                                    fen,
                                    moves: None,
                                }) {
                                    println!("{e}");
                                }
                            }
                            _ => {
                                println!("Unsure what you mean. Type 'uci' to get started.")
                            }
                        },
                        Some("go") => {
                            stop.store(false, Ordering::Relaxed);
                            match args.next() {
                                Some("depth") => match args.next().and_then(|d| d.parse().ok()) {
                                    Some(depth) => {
                                        if let Err(e) = cmd_sender.send(EngineCommand::Start {
                                            limits: EngineLimits { depth: Some(depth) },
                                        }) {
                                            println!("{e}");
                                        }
                                    }
                                    None => println!("Depth not specified"),
                                },
                                _ => {
                                    println!("Unsure what you mean. Type 'uci' to get started.")
                                }
                            }
                        }
                        Some("stop") => stop.store(true, Ordering::Relaxed),
                        Some("setoption") => {}
                        Some(_) => println!("Unsure what you mean. Type 'uci' to get started."),
                        None => println!("No command received. Type 'uci' to get started."),
                    }
                }
                UCIMessage::Event(event) => match event {
                    EngineEvent::IterationInfo(info) => println!(
                        "info depth {} seldepth {} multipv 1 score {} nodes {} nps {} pv {}",
                        info.depth,
                        info.seldepth,
                        match info.score {
                            Score::CP(score) => format!("cp {score}"),
                            Score::Mate(score) => format!("mate {score}"),
                        },
                        info.nodes,
                        info.nps,
                        info.best_line
                            .unwrap_or_default()
                            .into_iter()
                            .map(|m| m.to_algebraic())
                            .collect::<Vec<_>>()
                            .join(" ")
                    ),
                    EngineEvent::SearchFinished(m) | EngineEvent::SearchStopped(m) => {
                        println!("bestmove {}", m.to_algebraic())
                    }
                },
            },
            Err(e) => println!("{e}"),
        }
    }
}
