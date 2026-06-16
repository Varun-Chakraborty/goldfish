use std::{
    sync::{Arc, atomic::AtomicBool, mpsc},
    thread,
};

use goldfish::{
    EngineCommand,
    EngineEvent::{IterationInfo, SearchFinished},
    EngineLimits, EngineWorker, GameState,
    UCIMessage::{Command, Event},
    fmt_pgn_moves,
};

fn main() {
    let (cmd_sender, cmd_receiver) = mpsc::channel();
    let (event_sender, event_receiver) = mpsc::channel();
    let stop = Arc::new(AtomicBool::new(false));

    let mut worker = EngineWorker::new(cmd_receiver, event_sender, stop.clone());

    thread::spawn(move || worker.run());

    // let fen = "3nr1R1/3K1kpp/8/7P/1R4p1/6P1/8/8 b - - 0 1";
    let fen = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w - - 0 1";
    let gs = GameState::from_fen(fen).unwrap();
    if let Err(e) = cmd_sender.send(EngineCommand::Init {
        fen: fen.to_string(),
        moves: None,
    }) {
        eprintln!("Failed to send Init: {e}");
        return;
    };

    if let Err(e) = cmd_sender.send(EngineCommand::Start {
        limits: EngineLimits { depth: Some(19) },
    }) {
        eprintln!("Failed to send Start: {e}");
        return;
    };

    loop {
        match event_receiver.recv() {
            Ok(msg) => match msg {
                Event(event) => match event {
                    IterationInfo(info) => {
                        let best_line =
                            fmt_pgn_moves(&gs, info.best_line.as_ref().expect("No best moves"))
                                .join(", ");
                        let search_stats = info.search_stats.expect("No search stats");
                        println!(
                            "\nd{} score={} {}n ({}n) {:.2}Mn/s leaf={:.2}% qnodes/leaf={:.2} ebf={:.2} mpc={:.2} me={:.2} hit={:.2}%\npv: {}",
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
                            search_stats.search_counters.examined_moves as f64
                                / search_stats.search_counters.available_moves as f64,
                            search_stats.tt_stats.hits as f64
                                / search_stats.tt_stats.probes.max(1) as f64
                                * 100.0,
                            best_line,
                        )
                    }
                    SearchFinished(_) => {
                        cmd_sender.send(EngineCommand::Quit).unwrap();
                        break;
                    }
                    _ => eprintln!("Unexpected event"),
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
