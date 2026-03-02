# Run with debug logging
debug level="debug":
    GOSUTO_LOG={{level}} cargo run

# Tail the log file
logs:
    tail -f ~/.local/share/gosuto/logs/gosuto.log*
