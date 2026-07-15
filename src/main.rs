use std::{
    io::{self, BufRead},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread,
};

use goldfish::{
    EngineCommand,
    EngineEvent::{Debug, IterationInfo, SearchFinished},
    EngineLimits, EngineWorker, Score, UCIMessage,
};

fn main() {
    let (cmd_sender, cmd_receiver) = mpsc::channel();
    let (ucimsg_sender, ucimsg_receiver) = mpsc::channel();
    let stop = Arc::new(AtomicBool::new(false));
    let ponderhit = Arc::new(AtomicBool::new(false));

    let mut worker = EngineWorker::new(
        cmd_receiver,
        ucimsg_sender.clone(),
        stop.clone(),
        ponderhit.clone(),
    );

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
                            println!("id name GoldFish");
                            println!("id author Varun");
                            println!();
                            println!("option name Hash type spin default 64 min 0 max 1048576");
                            println!("option name Ponder type check default true");
                            println!("option name OwnBook type check default false");
                            println!("option name BookFile type string default ");
                            println!("uciok");
                        }
                        Some("ucinewgame") => {
                            if let Err(e) = cmd_sender.send(EngineCommand::NewGame) {
                                println!("{e}");
                            }
                        }
                        Some("isready") => println!("readyok"),
                        Some("q") | Some("quit") => {
                            stop.store(true, Ordering::Relaxed);
                            if let Err(e) = cmd_sender.send(EngineCommand::Quit) {
                                println!("{e}");
                            }
                            println!("Goodbye!");
                            break;
                        }
                        Some("position") => {
                            let fen;
                            let mut moves = None;
                            match args.next() {
                                Some("startpos") => {
                                    fen = "startpos".to_string();
                                }
                                Some("fen") => {
                                    fen = args.by_ref().take(6).collect::<Vec<_>>().join(" ");
                                }
                                _ => {
                                    eprintln!("Unsure what you mean. Type 'uci' to get started.");
                                    continue;
                                }
                            }
                            let argument = args.next();
                            if argument == Some("moves") || argument == Some("move") {
                                moves = Some(args.map(|m| m.to_string()).collect());
                            }
                            if let Err(e) = cmd_sender.send(EngineCommand::Init { fen, moves }) {
                                println!("{e}");
                            }
                        }
                        Some("go") => {
                            stop.store(false, Ordering::Relaxed);
                            ponderhit.store(false, Ordering::Relaxed);
                            let mut limits = EngineLimits::default();
                            let mut perft_depth = 0;
                            let mut go_type = "none";
                            while let Some(arg) = args.next() {
                                match arg {
                                    "infinite" => {
                                        limits = EngineLimits::default();
                                        go_type = "search";
                                        break;
                                    }
                                    "depth" => match args.next().and_then(|d| d.parse().ok()) {
                                        Some(depth) => {
                                            limits.depth = Some(depth);
                                            go_type = "search";
                                        }
                                        None => {
                                            println!("Depth not specified");
                                            go_type = "none";
                                            break;
                                        }
                                    },
                                    "wtime" => match args.next().and_then(|d| d.parse().ok()) {
                                        Some(depth) => {
                                            limits.wtime = Some(depth);
                                            go_type = "search";
                                        }
                                        None => {
                                            println!("wtime not specified");
                                            go_type = "none";
                                            break;
                                        }
                                    },
                                    "btime" => match args.next().and_then(|d| d.parse().ok()) {
                                        Some(depth) => {
                                            limits.btime = Some(depth);
                                            go_type = "search";
                                        }
                                        None => {
                                            println!("btime not specified");
                                            go_type = "none";
                                            break;
                                        }
                                    },
                                    "winc" => match args.next().and_then(|d| d.parse().ok()) {
                                        Some(depth) => {
                                            limits.winc = Some(depth);
                                            go_type = "search";
                                        }
                                        None => {
                                            println!("winc not specified");
                                            go_type = "none";
                                            break;
                                        }
                                    },
                                    "binc" => match args.next().and_then(|d| d.parse().ok()) {
                                        Some(depth) => {
                                            limits.binc = Some(depth);
                                            go_type = "search";
                                        }
                                        None => {
                                            println!("binc not specified");
                                            go_type = "none";
                                            break;
                                        }
                                    },
                                    "movestogo" => match args.next().and_then(|d| d.parse().ok()) {
                                        Some(depth) => {
                                            limits.movestogo = Some(depth);
                                            go_type = "search";
                                        }
                                        None => {
                                            println!("movestogo not specified");
                                            go_type = "none";
                                            break;
                                        }
                                    },
                                    "movetime" => match args.next().and_then(|d| d.parse().ok()) {
                                        Some(depth) => {
                                            limits.movetime = Some(depth);
                                            go_type = "search";
                                        }
                                        None => {
                                            println!("movetime not specified");
                                            go_type = "none";
                                            break;
                                        }
                                    },
                                    "nodes" => match args.next().and_then(|d| d.parse().ok()) {
                                        Some(depth) => {
                                            limits.nodes = Some(depth);
                                            go_type = "search";
                                        }
                                        None => {
                                            println!("nodes not specified");
                                            go_type = "none";
                                            break;
                                        }
                                    },
                                    "ponder" => {
                                        limits.ponder = true;
                                        go_type = "search";
                                    }
                                    "perft" => match args.next().and_then(|d| d.parse().ok()) {
                                        Some(depth) => {
                                            perft_depth = depth;
                                            go_type = "perft";
                                        }
                                        None => {
                                            println!("Depth not specified");
                                            go_type = "none";
                                            break;
                                        }
                                    },
                                    _ => {
                                        eprintln!(
                                            "Unsure what you mean. Type 'uci' to get started."
                                        );
                                        break;
                                    }
                                }
                            }
                            match go_type {
                                "search" => {
                                    if let Err(e) = cmd_sender.send(EngineCommand::Start { limits })
                                    {
                                        println!("{e}");
                                    }
                                }
                                "perft" => {
                                    if let Err(e) =
                                        cmd_sender.send(EngineCommand::Perft { depth: perft_depth })
                                    {
                                        println!("{e}");
                                    }
                                }
                                _ => (),
                            }
                        }
                        Some("stop") => stop.store(true, Ordering::Relaxed),
                        Some("ponderhit") => ponderhit.store(true, Ordering::Relaxed),
                        Some("setoption") => {
                            let mut name = vec![];
                            if let Some(arg) = args.next()
                                && arg == "name"
                            {
                                while let Some(arg) = args.next()
                                    && arg != "value"
                                {
                                    name.push(arg);
                                }
                                let value: String = args.collect();
                                let name = name.join(" ");
                                if !name.is_empty()
                                    && !value.is_empty()
                                    && let Err(e) =
                                        cmd_sender.send(EngineCommand::SetOption { name, value })
                                {
                                    println!("{e}");
                                }
                            }
                        }
                        Some("d") => {
                            if let Err(e) = cmd_sender.send(EngineCommand::Debug) {
                                println!("{e}");
                            }
                        }
                        Some(_) => eprintln!("Unsure what you mean. Type 'uci' to get started."),
                        None => eprintln!("No command received. Type 'uci' to get started."),
                    }
                }
                UCIMessage::Event(event) => match event {
                    IterationInfo(info) => println!(
                        "info depth {} seldepth {} multipv 1 score {} nodes {} nps {} hashfull {} time {} pv {}",
                        info.depth,
                        info.seldepth,
                        match info.score {
                            Score::CP(score) => format!("cp {score}"),
                            Score::Mate(score) => format!("mate {score}"),
                        },
                        info.search_stats.search_counters.nodes + info.qsearch_stats.nodes,
                        ((info.search_stats.search_counters.nodes + info.qsearch_stats.nodes)
                            as f64
                            / info.time.as_secs_f64())
                        .ceil() as u64,
                        info.hashfull,
                        info.time.as_millis(),
                        info.best_line
                            .unwrap_or_default()
                            .into_iter()
                            .map(|m| m.to_algebraic())
                            .collect::<Vec<_>>()
                            .join(" ")
                    ),
                    SearchFinished { best_move, ponder } => match (best_move, ponder) {
                        (Some(m), Some(p)) => {
                            println!("bestmove {} ponder {}", m.to_algebraic(), p.to_algebraic())
                        }
                        (Some(m), None) => println!("bestmove {}", m.to_algebraic()),
                        _ => println!("bestmove (none)"),
                    },
                    Debug { string } => println!("{string}"),
                },
            },
            Err(e) => println!("{e}"),
        }
    }
}
