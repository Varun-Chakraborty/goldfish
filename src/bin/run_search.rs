use std::{
    sync::{Arc, atomic::AtomicBool, mpsc},
    thread,
};

use goldfish::{
    EngineCommand,
    EngineEvent::{Debug, IterationInfo, SearchFinished},
    EngineLimits, EngineWorker, GameState,
    UCIMessage::{Command, Event},
    fmt_pgn_moves,
};

fn main() {
    let (cmd_sender, cmd_receiver) = mpsc::channel();
    let (event_sender, event_receiver) = mpsc::channel();
    let stop = Arc::new(AtomicBool::new(false));
    let ponderhit = Arc::new(AtomicBool::new(false));

    let mut worker = EngineWorker::new(cmd_receiver, event_sender, stop.clone(), ponderhit.clone());

    thread::spawn(move || worker.run());

    // let fen = "3nr1R1/5kpp/2K5/7P/1R4p1/6P1/8/8 w - - 0 1"; // tactical position
    let fen = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w - - 0 1"; // starting position
    // let fen = "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1"; // kiwipete position
    let gs = GameState::from_fen(fen).unwrap();
    if let Err(e) = cmd_sender.send(EngineCommand::Init {
        fen: fen.to_string(),
        moves: None,
    }) {
        eprintln!("Failed to send Init: {e}");
        return;
    };

    let limits = EngineLimits::default();
    if let Err(e) = cmd_sender.send(EngineCommand::Start { limits }) {
        eprintln!("Failed to send Start: {e}");
        return;
    };

    loop {
        match event_receiver.recv() {
            Ok(msg) => match msg {
                Event(event) => match event {
                    IterationInfo(info) => {
                        let best_line = info.best_line.as_ref().expect("No best moves");
                        let best_line = fmt_pgn_moves(&gs, best_line).join(", ");
                        let search_stats = info.search_stats.expect("No search stats");
                        println!(
                            "d{} score={} {}n ({}n) {:.2}Mn/s leaf={:.2}% qnodes/leaf={:.2} ebf={:.2} mpc={:.2} first_move_cutoffs={} me={:.2} hit={:.2}%\npv: {}",
                            info.depth,
                            info.raw_score,
                            match search_stats.search_counters.nodes {
                                0..=99999 => format!(
                                    "{:.2}k",
                                    search_stats.search_counters.nodes as f64 / 1000.0
                                ),
                                n => format!("{:.2}M", n as f64 / 1_000_000.0),
                            },
                            match search_stats.delta {
                                -99_999..=99_999 =>
                                    format!("{:+.2}k", search_stats.delta as f64 / 1_000.0),
                                n => format!("{:+.2}M", n as f64 / 1_000_000.0),
                            },
                            info.nps as f64 / 1_000_000.0,
                            search_stats.search_counters.leaf_nodes as f64
                                / search_stats.search_counters.nodes as f64
                                * 100.0,
                            search_stats.search_counters.qnodes as f64
                                / search_stats.search_counters.leaf_nodes as f64,
                            search_stats.branching_factor,
                            search_stats.search_counters.examined_moves as f64
                                / (search_stats.search_counters.cutoffs).max(1) as f64,
                            match search_stats.search_counters.first_move_cutoffs {
                                0..=99999 => format!(
                                    "{:.2}k",
                                    search_stats.search_counters.first_move_cutoffs as f64 / 1000.0
                                ),
                                n => format!("{:.2}M", n as f64 / 1_000_000.0),
                            },
                            search_stats.search_counters.examined_moves as f64
                                / search_stats.search_counters.available_moves as f64,
                            search_stats.tt_stats.hits as f64
                                / search_stats.tt_stats.probes.max(1) as f64
                                * 100.0,
                            best_line,
                        )
                    }
                    SearchFinished { .. } => {
                        cmd_sender.send(EngineCommand::Quit).unwrap();
                        break;
                    }
                    Debug { .. } => {}
                },
                Command(line) => println!("{}", line),
            },
            Err(e) => {
                eprintln!("Event channel error: {e}");
                break;
            }
        };
    }
}
