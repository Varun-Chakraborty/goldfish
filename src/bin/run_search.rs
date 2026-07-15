use std::{
    sync::{Arc, atomic::AtomicBool, mpsc},
    thread,
};

use goldfish::{
    EngineCommand,
    EngineEvent::{CurrentMove, Debug, IterationInfo, SearchFinished},
    EngineLimits, EngineWorker, GameState,
    UCIMessage::{Command, Event},
    fmt_pgn_moves,
};

fn fmt_numbers(n: u64) -> String {
    match n {
        0..=99999 => format!("{:.2}k", n as f64 / 1000.0),
        n => format!("{:.2}M", n as f64 / 1_000_000.0),
    }
}

fn fmt_delta(n: i64) -> String {
    match n {
        0..=99999 => format!("{:+.2}k", n as f64 / 1000.0),
        n => format!("{:+.2}M", n as f64 / 1_000_000.0),
    }
}

fn stat(name: &str, value: impl std::fmt::Display) {
    println!("{:<18} : {}", name, value);
}

fn main() {
    let (cmd_sender, cmd_receiver) = mpsc::channel();
    let (event_sender, event_receiver) = mpsc::channel();
    let stop = Arc::new(AtomicBool::new(false));
    let ponderhit = Arc::new(AtomicBool::new(false));

    let mut worker = EngineWorker::new(cmd_receiver, event_sender, stop.clone(), ponderhit.clone());

    thread::spawn(move || worker.run());

    // let fen = "3nr1R1/5kpp/2K5/7P/1R4p1/6P1/8/8 w - - 0 1"; // tactical position
    let fen = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1"; // starting position
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
                        println!("d{} score: {}", info.depth, info.raw_score);
                        println!("PV: {}", best_line);
                        println!();
                        println!("Search:");
                        stat(
                            "Nodes",
                            format!(
                                "{}n ({}n) @ {:.2}Mn/s",
                                fmt_numbers(info.search_stats.search_counters.nodes),
                                fmt_delta(info.search_stats.delta),
                                info.search_stats.search_counters.nodes as f64
                                    / info.time.as_secs_f64()
                                    / 1_000_000.0,
                            ),
                        );
                        stat(
                            "QNodes",
                            format!(
                                "{}n @ {:.2}Mn/s",
                                fmt_numbers(info.qsearch_stats.nodes),
                                info.qsearch_stats.nodes as f64
                                    / info.time.as_secs_f64()
                                    / 1_000_000.0,
                            ),
                        );
                        stat(
                            "TT Hit Rate",
                            format!(
                                "{:.2}%",
                                info.tt_stats.hits as f64 / info.tt_stats.probes.max(1) as f64
                                    * 100.0,
                            ),
                        );
                        println!();
                        println!("Tree:");
                        stat(
                            "Leaf %",
                            format!(
                                "{:.2}",
                                info.search_stats.search_counters.leaf_nodes as f64
                                    / info.search_stats.search_counters.nodes as f64
                                    * 100.0,
                            ),
                        );
                        stat("EBF", format!("{:.2}", info.search_stats.branching_factor));
                        stat(
                            "MPC",
                            format!(
                                "{:.2}",
                                info.search_stats.search_counters.examined_moves as f64
                                    / (info.search_stats.search_counters.cutoffs).max(1) as f64,
                            ),
                        );
                        stat(
                            "1st Cutoff",
                            format!(
                                "{}",
                                fmt_numbers(info.search_stats.search_counters.first_move_cutoffs),
                            ),
                        );
                        stat(
                            "Moves Examined",
                            format!(
                                "{:.2}",
                                info.search_stats.search_counters.examined_moves as f64
                                    / info.search_stats.search_counters.available_moves as f64,
                            ),
                        );
                        println!();
                        println!("LMR:");
                        stat(
                            "Reduced",
                            format!(
                                "{:>6}",
                                fmt_numbers(info.search_stats.search_counters.reduced_searches),
                            ),
                        );
                        stat(
                            "Fail Low",
                            format!(
                                "{:>6} ({:>6.2}%)",
                                fmt_numbers(info.search_stats.search_counters.reduced_fail_low),
                                info.search_stats.search_counters.reduced_fail_low as f64
                                    / info.search_stats.search_counters.reduced_searches.max(1)
                                        as f64
                                    * 100.0
                            ),
                        );
                        stat(
                            "Fail High",
                            format!(
                                "{:>6} ({:>6.2}%)",
                                fmt_numbers(info.search_stats.search_counters.reduced_fail_high),
                                info.search_stats.search_counters.reduced_fail_high as f64
                                    / info.search_stats.search_counters.reduced_searches.max(1)
                                        as f64
                                    * 100.0
                            ),
                        );
                        stat(
                            "Verify Low",
                            format!(
                                "{:>6} ({:>6.2}%)",
                                fmt_numbers(info.search_stats.search_counters.verified_fail_low),
                                info.search_stats.search_counters.verified_fail_low as f64
                                    / info.search_stats.search_counters.reduced_fail_high.max(1)
                                        as f64
                                    * 100.0
                            ),
                        );
                        stat(
                            "Verify High",
                            format!(
                                "{:>6} ({:>6.2}%)",
                                fmt_numbers(info.search_stats.search_counters.verified_fail_high),
                                info.search_stats.search_counters.verified_fail_high as f64
                                    / info.search_stats.search_counters.reduced_fail_high.max(1)
                                        as f64
                                    * 100.0
                            ),
                        );
                        println!();
                    }
                    SearchFinished { .. } => {
                        cmd_sender.send(EngineCommand::Quit).unwrap();
                        break;
                    }
                    Debug { .. } | CurrentMove { .. } => {}
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
