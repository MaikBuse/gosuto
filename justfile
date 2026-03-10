# Run with debug logging (scoped to gosuto by default)
# For advanced control: just debug "debug,matrix_sdk=trace"
debug level="debug":
    GOSUTO_LOG={{level}} cargo run

# Run with a named profile (separate data instance)
test profile="test":
    cargo run -- --profile {{profile}}

# Run with a named profile and debug logging
test-debug profile="test" level="debug":
    GOSUTO_LOG={{level}} cargo run -- --profile {{profile}}

# Tail the log file
logs:
    tail -f ~/.local/share/gosuto/logs/gosuto.log*

# Remove all log files
clean-logs:
    rm -f ~/.local/share/gosuto/logs/gosuto.log*
    rm -f ~/.local/share/gosuto-test/logs/gosuto.log*
