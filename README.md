# GoldFish

A UCI-compatible chess engine written in Rust.

## Features

- **Search**: Iterative deepening with negamax alpha-beta, quiescence search, and Late Move Reductions (LMR)
- **Evaluation**: Material, piece-square tables, mobility, king safety, pawn structure (passed/isolated/doubled pawns), and bishop pair bonus
- **Move Ordering**: Principal variation search, transposition table, MVV-LVA captures, and history heuristics
- **Transposition Table**: Depth-preferred replacement with exact/lower/upper bound entries
- **Opening Book**: Polyglot `.bin` format support with weighted random selection
- **Zobrist Hashing**: Polyglot constants for incremental hash updates
- **Clock Management**: Time allocation based on remaining time, increments, and move budgets
- **UCI Protocol**: Full UCI support including `ponder`, `setoption`, `perft`, and `debug` commands
- **Perft**: Move generation correctness testing with divide output

## Build

```sh
cargo build --release
```

The release profile uses fat LTO, single codegen unit, and `panic = "abort"` for maximum performance.

## Usage

### UCI Mode (default)

```sh
cargo run --release
```

Then interact via UCI commands:

```
uci
isready
position startpos
go depth 20
go wtime 60000 btime 60000 winc 1000 binc 1000
stop
quit
```

### Supported UCI Options

| Option    | Type   | Default | Description                        |
|-----------|--------|---------|------------------------------------|
| Hash      | spin   | 64      | Transposition table size in MB     |
| Ponder    | check  | true    | Enable pondering                   |
| OwnBook   | check  | false   | Use the opening book               |
| BookFile  | string |         | Path to Polyglot `.bin` book file  |

### Search Analysis Binary

```sh
cargo run --release --bin run_search
```

Runs an infinite-depth search on a hardcoded position and prints detailed per-iteration statistics (nodes, NPS, EBF, TT hit rate, LMR stats, etc.).

## Project Structure

```
src/
├── main.rs                  # UCI loop and command dispatch
├── lib.rs                   # EngineWorker, EngineCommand, EngineEvent
├── engine.rs                # GoldFish engine: search, perft, divide, debug
├── board.rs                 # 64-square board with FEN parsing
├── position.rs              # Position analysis (checkmate, stalemate, draws)
├── notation.rs              # Algebraic notation parsing and PGN formatting
├── types.rs                 # Color, PieceType, Square, Coordinate, Move
├── clock.rs                 # Time management and budget allocation
├── history.rs               # History heuristic for move ordering
├── transposition.rs         # Transposition table (probe, store, stats)
├── bin/
│   └── run_search.rs        # Standalone search analysis tool
├── game/
│   ├── mod.rs               # GameState, FEN, ray tracing, pins, checks, LVA
│   ├── move_gen.rs          # Legal move generation (pawns, knights, sliders, king, castling)
│   ├── make_unmake.rs       # Make/unmake move with incremental updates
│   ├── pst.rs               # Piece-square tables
│   ├── mobility.rs          # Mobility evaluation and affected-piece tracking
│   ├── tests.rs             # Move generation and make/unmake tests
│   └── eval/
│       ├── mod.rs           # Evaluation function combining all terms
│       ├── king_safety.rs   # Pawn shield and king safety scoring
│       └── pawn_structure.rs # Passed, isolated, and doubled pawn detection
├── search/
│   ├── mod.rs               # Iterative deepening loop
│   ├── negamax.rs           # Negamax with alpha-beta, LMR, TT, and PV tracking
│   ├── quiescence.rs        # Quiescence search (captures + check evasions)
│   ├── ordermoves.rs        # Move ordering (PV, TT, MVV-LVA, history)
│   └── search_types.rs      # SearchContext, PVTable, EngineLimits, stats
├── opening_book/
│   ├── mod.rs               # Polyglot book lookup and weighted move selection
│   └── parse_book.rs        # Binary book file parser
└── zobrist/
    ├── mod.rs               # Zobrist hash computation and incremental updates
    └── consts.rs            # Polyglot random number constants
```

## Testing

```sh
cargo test
```

Tests cover:
- Perft on standard positions (startpos, Kiwipete, etc.)
- FEN parsing and output
- Legal move generation
- Make/unmake move correctness (captures, castling, en passant, promotions)
- Zobrist hash verification against known Polyglot values
- Evaluation sanity checks
- Algebraic notation disambiguation

## Profiling

```sh
# CPU profiling with samply
./profile.sh

# Memory profiling with heaptrack
./memory.profile.sh
```

## Dependencies

- `rand` - Opening book weighted random selection
- `thiserror` - Ergonomic error types
