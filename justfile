# Run with debug logging
debug level="debug":
    GOSUTO_LOG={{level}} cargo run

# Run in demo mode (no server needed)
demo:
    cargo run -- --demo

# Tail the log file
logs:
    tail -f ~/.local/share/gosuto/logs/gosuto.log*
