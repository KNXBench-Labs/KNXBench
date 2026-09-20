#!/usr/bin/env bash
# apps/knx-server/scripts/smoke-test.sh
# Builds the image, runs it, and confirms it answers /healthz, refuses an
# unauthenticated API call, and can import a project mounted into /data
# once logged in. Run from the repository root.
set -euo pipefail

reference_project="${KNXBENCH_REFERENCE_PROJECT:-}"

if [[ -n "$reference_project" && ! -f "$reference_project" ]]; then
  echo "reference project not found: $reference_project" >&2
  exit 1
fi

docker build -t knxbench-server -f apps/knx-server/Dockerfile .

# ADR-0026: without a password the server binds 127.0.0.1 inside its own
# network namespace, so -p would publish a port nothing listens on. The
# hash is produced by the image itself, which also smoke-tests
# --hash-password; the password never reaches an argument vector.
smoke_password="smoke test password"
auth_hash="$(printf '%s\n' "$smoke_password" \
  | docker run --rm -i knxbench-server --hash-password)"
case "$auth_hash" in
  '$pbkdf2-sha256$i=600000$'*) ;;
  *) echo "--hash-password produced something unexpected: $auth_hash" >&2; exit 1 ;;
esac

mkdir -p .smoke-data
if [[ -n "$reference_project" ]]; then
  reference_name="$(basename "$reference_project")"
  cp "$reference_project" ".smoke-data/$reference_name"
fi

cid=$(docker run -d -p 18080:8080 \
  -e KNX_AUTH_PASSWORD_HASH="$auth_hash" \
  -v "$(pwd)/.smoke-data:/data" knxbench-server)
jar=$(mktemp)
trap 'docker rm -f "$cid" >/dev/null; rm -rf .smoke-data "$jar"' EXIT

for _ in $(seq 1 30); do
  if curl -sf http://127.0.0.1:18080/healthz >/dev/null; then break; fi
  sleep 1
done
# /healthz stays open on purpose: a liveness probe holds no session.
curl -sf http://127.0.0.1:18080/healthz
echo

# The guard, seen from outside: no session, no project.
status=$(curl -s -o /dev/null -w '%{http_code}' http://127.0.0.1:18080/api/version)
test "$status" = "401" || { echo "an unauthenticated /api/version answered $status, not 401"; exit 1; }

curl -sf -c "$jar" -X POST http://127.0.0.1:18080/api/auth/login \
  -H 'Content-Type: application/json' \
  -d "{\"password\":\"$smoke_password\"}" >/dev/null
grep -q knx_session "$jar" || { echo "login set no session cookie"; exit 1; }

status=$(curl -s -b "$jar" -o /dev/null -w '%{http_code}' http://127.0.0.1:18080/api/version)
test "$status" = "200" || { echo "an authenticated /api/version answered $status, not 200"; exit 1; }

created=$(curl -sf -b "$jar" -X POST http://127.0.0.1:18080/api/project/new \
  -H 'Content-Type: application/json' \
  -d '{"name":"Docker smoke","installationName":"Smoke installation","language":"en","groupAddressStyle":"ThreeLevel","discardChanges":false}')
echo "$created" | grep -q '"errors":0' || { echo "new project did not report 0 errors: $created"; exit 1; }

curl -sf -b "$jar" -X POST http://127.0.0.1:18080/api/project/save-as \
  -H 'Content-Type: application/json' \
  -d '{"path":"/data/smoke.knxdb"}' >/dev/null
test -s .smoke-data/smoke.knxdb

curl -sf -b "$jar" -X POST http://127.0.0.1:18080/api/project/new \
  -H 'Content-Type: application/json' \
  -d '{"name":"Replacement","installationName":"Replacement","language":"en","groupAddressStyle":"Free","discardChanges":false}' >/dev/null

reopened=$(curl -sf -b "$jar" -X POST http://127.0.0.1:18080/api/project/open \
  -H 'Content-Type: application/json' \
  -d '{"path":"/data/smoke.knxdb"}')
echo "$reopened" | grep -q '"errors":0' || { echo "reopened project did not report 0 errors: $reopened"; exit 1; }
echo "$reopened" | grep -q '"name":"Smoke installation"' || { echo "reopened project lost its installation: $reopened"; exit 1; }

if [[ -n "$reference_project" ]]; then
  imported=$(curl -sf -b "$jar" -X POST http://127.0.0.1:18080/api/project/import \
    -H 'Content-Type: application/json' \
    -d "{\"path\":\"/data/$reference_name\"}")
  echo "$imported" | grep -q '"errors":0' || { echo "import did not report 0 errors: $imported"; exit 1; }
fi

echo "smoke test passed"
