#!/usr/bin/env bash
# apps/knx-server/scripts/smoke-test.sh
# Builds the image, runs it, and confirms it answers /healthz and can
# import a project mounted into /data. Run from the repository root.
set -euo pipefail

reference_project="${KNXBENCH_REFERENCE_PROJECT:-}"

if [[ -n "$reference_project" && ! -f "$reference_project" ]]; then
  echo "reference project not found: $reference_project" >&2
  exit 1
fi

docker build -t knxbench-server -f apps/knx-server/Dockerfile .

mkdir -p .smoke-data
if [[ -n "$reference_project" ]]; then
  reference_name="$(basename "$reference_project")"
  cp "$reference_project" ".smoke-data/$reference_name"
fi

cid=$(docker run -d -p 18080:8080 -v "$(pwd)/.smoke-data:/data" knxbench-server)
trap 'docker rm -f "$cid" >/dev/null; rm -rf .smoke-data' EXIT

for _ in $(seq 1 30); do
  if curl -sf http://127.0.0.1:18080/healthz >/dev/null; then break; fi
  sleep 1
done
curl -sf http://127.0.0.1:18080/healthz
echo

created=$(curl -sf -X POST http://127.0.0.1:18080/api/project/new \
  -H 'Content-Type: application/json' \
  -d '{"name":"Docker smoke","installationName":"Smoke installation","language":"en","groupAddressStyle":"ThreeLevel","discardChanges":false}')
echo "$created" | grep -q '"errors":0' || { echo "new project did not report 0 errors: $created"; exit 1; }

curl -sf -X POST http://127.0.0.1:18080/api/project/save-as \
  -H 'Content-Type: application/json' \
  -d '{"path":"/data/smoke.knxdb"}' >/dev/null
test -s .smoke-data/smoke.knxdb

curl -sf -X POST http://127.0.0.1:18080/api/project/new \
  -H 'Content-Type: application/json' \
  -d '{"name":"Replacement","installationName":"Replacement","language":"en","groupAddressStyle":"Free","discardChanges":false}' >/dev/null

reopened=$(curl -sf -X POST http://127.0.0.1:18080/api/project/open \
  -H 'Content-Type: application/json' \
  -d '{"path":"/data/smoke.knxdb"}')
echo "$reopened" | grep -q '"errors":0' || { echo "reopened project did not report 0 errors: $reopened"; exit 1; }
echo "$reopened" | grep -q '"name":"Smoke installation"' || { echo "reopened project lost its installation: $reopened"; exit 1; }

if [[ -n "$reference_project" ]]; then
  imported=$(curl -sf -X POST http://127.0.0.1:18080/api/project/import \
    -H 'Content-Type: application/json' \
    -d "{\"path\":\"/data/$reference_name\"}")
  echo "$imported" | grep -q '"errors":0' || { echo "import did not report 0 errors: $imported"; exit 1; }
fi

echo "smoke test passed"
