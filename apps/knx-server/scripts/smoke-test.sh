#!/usr/bin/env bash
# apps/knx-server/scripts/smoke-test.sh
# Builds the image, runs it, and confirms it answers /healthz and can
# import a project mounted into /data. Run from the repository root.
set -euo pipefail

docker build -t knxbench-server -f apps/knx-server/Dockerfile .

mkdir -p .smoke-data
cp "Unser Zuhause ets4 - 2025-12-15.knxproj" .smoke-data/

cid=$(docker run -d -p 18080:8080 -v "$(pwd)/.smoke-data:/data" knxbench-server)
trap 'docker rm -f "$cid" >/dev/null; rm -rf .smoke-data' EXIT

for _ in $(seq 1 30); do
  if curl -sf http://127.0.0.1:18080/healthz >/dev/null; then break; fi
  sleep 1
done
curl -sf http://127.0.0.1:18080/healthz
echo

response=$(curl -sf -X POST http://127.0.0.1:18080/api/project/import \
  -H 'Content-Type: application/json' \
  -d '{"path": "/data/Unser Zuhause ets4 - 2025-12-15.knxproj"}')
echo "$response" | grep -q '"errors":0' || { echo "import did not report 0 errors: $response"; exit 1; }

echo "smoke test passed"
