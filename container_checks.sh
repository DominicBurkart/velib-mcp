#!/usr/bin/env bash
set -euo pipefail

STATUS=0
CONTAINER_ID=""

find_available_port() {
  local port=8080
  while true; do
    if ss -tuln | grep -q ":$port "; then
      ((port++))
      if [ $port -gt 8180 ]; then
        echo "No available ports found in range 8080-8180" >&2
        exit 1
      fi
    else
      echo $port
      return
    fi
  done
}

cleanup() {
  if [ -n "$CONTAINER_ID" ]; then
    echo "Cleaning up container..."
    podman stop "$CONTAINER_ID" &> /dev/null || true
    podman rm "$CONTAINER_ID" &> /dev/null || true
  fi
}

trap cleanup EXIT

SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )
cd "$SCRIPT_DIR"

PORT=$(find_available_port)
echo "Using port $PORT for container checks"

CONTAINER_ID=$(podman run -d -p $PORT:8080 velib-mcp)
echo "Container ID: $CONTAINER_ID"

echo "Waiting for HTTP server to be ready..."
MAX_RETRIES=30
RETRY_COUNT=0

while [ $RETRY_COUNT -lt $MAX_RETRIES ]; do
  if curl -4 -s -f http://localhost:$PORT/health > /dev/null 2>&1; then
    echo "HTTP server is ready"
    break
  fi

  if ! podman ps --filter "id=$CONTAINER_ID" --format "{{.ID}}" | grep -q "$CONTAINER_ID"; then
    echo "ERROR: Container exited prematurely"
    podman logs "$CONTAINER_ID"
    STATUS=1
    exit $STATUS
  fi

  RETRY_COUNT=$((RETRY_COUNT + 1))
  sleep 1
done

if [ $RETRY_COUNT -eq $MAX_RETRIES ]; then
  echo "ERROR: HTTP server failed to start within ${MAX_RETRIES} seconds"
  podman logs "$CONTAINER_ID"
  STATUS=1
  exit $STATUS
fi

echo "Testing HTTP endpoints..."
ENDPOINTS_TESTED=0
ENDPOINTS_PASSED=0
while IFS= read -r page; do
  ENDPOINTS_TESTED=$((ENDPOINTS_TESTED + 1))
  if curl -4 -s -f "http://localhost:$PORT$page" > /dev/null 2>&1; then
    echo "  OK $page"
    ENDPOINTS_PASSED=$((ENDPOINTS_PASSED + 1))
  else
    echo "  FAIL $page"
    STATUS=1
  fi
done < "$SCRIPT_DIR/CHECKS"

echo "Tested $ENDPOINTS_TESTED endpoints, $ENDPOINTS_PASSED passed"

if [ $STATUS -eq 0 ]; then
  echo "All container checks passed"
else
  echo "Some container checks failed"
fi

exit $STATUS
