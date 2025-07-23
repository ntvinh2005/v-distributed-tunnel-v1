#!/bin/bash

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
PROJECT_ROOT="$SCRIPT_DIR/../.."

SERVER_BIN="$PROJECT_ROOT/target/release/server"
CLIENT_BIN="$PROJECT_ROOT/target/release/client"
ECHO_BIN="$PROJECT_ROOT/test/simple-go-service-client-test/echo-server"
IPERF3_BIN="iperf3"
RESULTS_DIR="$PROJECT_ROOT/results"
RESULTS_FILE="$RESULTS_DIR/bench-$(date +%Y%m%d-%H%M%S).csv"

mkdir -p "$RESULTS_DIR"

pkill -f "$SERVER_BIN"
pkill -f "$CLIENT_BIN"
pkill -f "$ECHO_BIN"
pkill -f "iperf3 -s"

echo "Starting Go echo server on port 8080..."
$ECHO_BIN &
ECHO_PID=$!
sleep 1

echo "Starting tunnel server..."
$SERVER_BIN &
SERVER_PID=$!
sleep 1

echo "Starting tunnel client..."
$CLIENT_BIN &
CLIENT_PID=$!
sleep 1

echo "Starting iperf3 server..."
$IPERF3_BIN -s -p 8080 &
IPERF3_SRV_PID=$!
sleep 1

echo "Streams,Throughput_Mbps,Transfer_MBytes" > "$RESULTS_FILE"

for streams in 1 5 10 20 50
do
    echo "Running iperf3 with $streams parallel streams..."
    OUTPUT=$($IPERF3_BIN -c 127.0.0.1 -p 8080 -P $streams)
    LINE=$(echo "$OUTPUT" | grep SUM | tail -1)
    THROUGHPUT=$(echo "$LINE" | awk '{print $(NF-1)}')
    UNIT=$(echo "$LINE" | awk '{print $NF}')
    TRANSFER=$(echo "$LINE" | awk '{print $(NF-4)}')
    if [[ "$UNIT" == "Mbits/sec" ]]; then
        echo "$streams,$THROUGHPUT,$TRANSFER" >> "$RESULTS_FILE"
    else
        echo "$streams,ERROR,ERROR" >> "$RESULTS_FILE"
    fi
    sleep 2
done

echo "Stopping all servers..."
kill $ECHO_PID $SERVER_PID $CLIENT_PID $IPERF3_SRV_PID 2>/dev/null

echo "Benchmark complete! Results saved in $RESULTS_FILE"
cat "$RESULTS_FILE"
