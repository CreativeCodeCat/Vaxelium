#!/usr/bin/env bash
# Local development: asks before each step, then updates the common crate, builds the auth, direct message and chat server
# services and the desktop client, starts the services in order, then starts the client. The services
# keep running until the client closes or you press Ctrl+C. Their logs are in target/dev-logs/.
# The client and the services use this folder's common crate, so pulling common is all the update needs.
set -euo pipefail
cd "$(dirname "$0")/.."

logs=target/dev-logs
mkdir -p "$logs"
pids=()
stop() {
    for pid in "${pids[@]}"; do
        kill "$pid" 2>/dev/null || true
    done
}
trap stop EXIT INT TERM

# Asks before a step. Anything but y stops the script, as later steps need the earlier ones. Services
# already started are stopped on the way out.
confirm() {
    local answer
    read -r -p "$1 [y/N] " answer || answer=""
    case "$answer" in
        y|Y|yes|YES) ;;
        *) echo "Stopped. Nothing else was started."; exit 1 ;;
    esac
}

# Waits until something accepts connections on a port, or stops the script if it never does.
wait_for() {
    local name=$1 port=$2
    for _ in $(seq 1 100); do
        if (exec 3<>"/dev/tcp/127.0.0.1/$port") 2>/dev/null; then
            return 0
        fi
        sleep 0.1
    done
    echo "$name did not start on port $port. Its log: $logs/$name.log" >&2
    exit 1
}

confirm "Update common (git pull in common/)?"
echo "Updating common..."
git -C common pull --ff-only origin main

confirm "Build auth, dm and server?"
echo "Building the services..."
cargo build -p auth -p dm -p server

confirm "Build the client?"
echo "Building the client..."
(cd client && cargo build)

confirm "Start auth on 127.0.0.1:7900?"
echo "Starting auth on 127.0.0.1:7900..."
target/debug/auth > "$logs/auth.log" 2>&1 &
pids+=($!)
wait_for auth 7900

confirm "Start the direct message service on 127.0.0.1:7910?"
echo "Starting the direct message service on 127.0.0.1:7910..."
target/debug/dm > "$logs/dm.log" 2>&1 &
pids+=($!)
wait_for dm 7910

confirm "Start the chat server on 127.0.0.1:7878?"
echo "Starting the chat server on 127.0.0.1:7878..."
target/debug/server > "$logs/server.log" 2>&1 &
pids+=($!)
wait_for server 7878

confirm "Start the client?"
echo "Starting the client. Add the server 127.0.0.1:7878 to connect."
(cd client && cargo run)
