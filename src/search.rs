use crate::{GameState, eval::eval, game::MoveGenMode, types::Move};
const MATE: i32 = 32000;

#[derive(Debug, Clone)]
struct SearchResult {
    score: i32,
    best_move: Option<Move>,
}

fn negamax(
    gs: &mut GameState,
    depth: u32,
    ply: u32,
    mut alpha: i32,
    beta: i32,
) -> SearchResult {
    if depth == 0 {
        return SearchResult { score: eval(gs).0, best_move: None };
    }

    let mut search_result = SearchResult {
        score: -MATE,
        best_move: None,
    };

    let mut legal = gs.legal_moves(MoveGenMode::All);

    legal.sort_by_key(|m| m.captured.is_none());

    for m in legal {
        let undo = gs.make_move(m);
        let mut result = negamax(
            gs,
            depth - 1,
            ply + 1,
            -beta,
            -alpha,
        );
        gs.unmake_move(undo);

        result.score = -result.score;

        if result.score > search_result.score {
            search_result.score = result.score;
            search_result.best_move = Some(m);
        }
        alpha = alpha.max(search_result.score);
        if alpha >= beta {
            break;
        }
    }

    search_result
}
