# Run with debug logging (scoped to gosuto by default)
# For advanced control: just debug "debug,matrix_sdk=trace"
debug level="debug":
    GOSUTO_LOG={{level}} cargo run

# Run in demo mode (no server needed)
demo:
    cargo run -- --demo

# Tail the log file
logs:
    tail -f ~/.local/share/gosuto/logs/gosuto.log*

# Remove all log files
clean-logs:
    rm -f ~/.local/share/gosuto/logs/gosuto.log*
