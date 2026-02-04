#!/bin/bash
set -e

# ---------------------------
# Config
# ---------------------------
REDIS_IMAGE="redis:8-alpine"
NODES=6
START_PORT=7000
START_BUS_PORT=17000

# Data directories (optional: change as needed)
DATA_DIR="./redis-data"

echo "Stopping previous Redis containers..."
for i in $(seq 0 $((NODES - 1))); do
  docker rm -f redis-node-$i 2> /dev/null || true
done

echo "Removing old data directories..."
for i in $(seq 0 $((NODES - 1))); do
  rm -rf "$DATA_DIR/node-$i"
done

echo "Creating data directories..."
for i in $(seq 0 $((NODES - 1))); do
  mkdir -p "$DATA_DIR/node-$i"
done

echo "Starting Redis nodes..."
for i in $(seq 0 $((NODES - 1))); do
  HOST_PORT=$((START_PORT + i))
  BUS_PORT=$((START_BUS_PORT + i))
  docker run -d \
    --name redis-node-$i \
    -p $HOST_PORT:6379 \
    -p $BUS_PORT:$BUS_PORT \
    -v "$(pwd)/$DATA_DIR/node-$i:/data" \
    $REDIS_IMAGE \
    redis-server \
      --port 6379 \
      --bind 0.0.0.0 \
      --protected-mode no \
      --cluster-enabled yes \
      --cluster-config-file nodes.conf \
      --cluster-node-timeout 5000 \
      --cluster-announce-ip 127.0.0.1 \
      --cluster-announce-port 6379 \
      --cluster-announce-bus-port $BUS_PORT \
      --appendonly yes
done

echo "Waiting 15 seconds for nodes to start..."
sleep 15

echo "Creating cluster..."
NODE_ADDRS=""
for i in $(seq 0 $((NODES - 1))); do
  NODE_ADDRS="$NODE_ADDRS 127.0.0.1:$((START_PORT + i))"
done

echo yes | docker run --rm $REDIS_IMAGE redis-cli --cluster create $NODE_ADDRS --cluster-replicas 1

echo "✅ Redis cluster is ready!"
