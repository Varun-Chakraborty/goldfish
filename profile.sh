# echo '-1' | sudo tee /proc/sys/kernel/perf_event_paranoid
# perf record -g cargo run --release --bin run_search

echo '-1' | sudo tee /proc/sys/kernel/perf_event_paranoid
cargo build --release
samply record cargo run --release --bin run_search
