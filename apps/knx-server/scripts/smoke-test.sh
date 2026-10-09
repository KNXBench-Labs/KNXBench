#!/usr/bin/env bash
# apps/knx-server/scripts/smoke-test.sh
# Builds the image (or takes a given one), runs it, and confirms it answers
# /healthz over HTTPS, refuses an unauthenticated API call, and can save,
# reopen and import a project mounted into /data once logged in. Run from the
# repository root.
#
#   KNXBENCH_IMAGE              test this image instead of building one
#                               (the Docker release workflow passes its build)
#   KNXBENCH_SMOKE_PORT         host port to publish (default 18080)
#   KNXBENCH_REFERENCE_PROJECT  optional .knxproj to import as a last step
set -euo pipefail

reference_project="${KNXBENCH_REFERENCE_PROJECT:-}"
image="${KNXBENCH_IMAGE:-}"
port="${KNXBENCH_SMOKE_PORT:-18080}"
base="https://127.0.0.1:${port}"

if [[ -n "$reference_project" && ! -f "$reference_project" ]]; then
  echo "reference project not found: $reference_project" >&2
  exit 1
fi

if [[ -z "$image" ]]; then
  image=knxbench-server
  docker build -t "$image" -f apps/knx-server/Dockerfile . \
    --build-arg KNX_BUILD_SHA="$(git rev-parse --short HEAD 2>/dev/null || true)"
fi

version="$(docker run --rm "$image" --version)"
test -n "$version" || { echo "--version printed nothing" >&2; exit 1; }
echo "image reports: $version"

# ADR-0026: without a password the server binds 127.0.0.1 inside its own
# network namespace, so -p would publish a port nothing listens on. The
# hash is produced by the image itself, which also smoke-tests
# --hash-password; the password never reaches an argument vector.
smoke_password="smoke test password"
auth_hash="$(printf '%s\n' "$smoke_password" \
  | docker run --rm -i "$image" --hash-password)"
# shellcheck disable=SC2016 # a literal PHC prefix, nothing to expand
case "$auth_hash" in
  '$pbkdf2-sha256$i=600000$'*) ;;
  *) echo "--hash-password produced something unexpected: $auth_hash" >&2; exit 1 ;;
esac

# The container writes /data as root (projects, and the generated TLS key in
# /data/.knxbench-tls), so the cleanup asks the image to hand the directory
# back before removing it.
data_dir="$(mktemp -d)"
jar="$(mktemp)"
cid=""
cleanup() {
  if [[ -n "$cid" ]]; then docker rm -f "$cid" >/dev/null 2>&1 || true; fi
  docker run --rm --entrypoint /bin/chown -v "$data_dir:/data" "$image" \
    -R "$(id -u):$(id -g)" /data >/dev/null 2>&1 || true
  rm -rf "$data_dir" "$jar"
}
trap cleanup EXIT

if [[ -n "$reference_project" ]]; then
  reference_name="$(basename "$reference_project")"
  cp "$reference_project" "$data_dir/$reference_name"
fi

cid=$(docker run -d -p "127.0.0.1:${port}:8080" \
  -e KNX_AUTH_PASSWORD_HASH="$auth_hash" \
  -v "$data_dir:/data" "$image")

# ADR-0088: a password means HTTPS with a self-signed certificate, hence -k.
healthy=""
for _ in $(seq 1 30); do
  if curl -skf "$base/healthz" >/dev/null; then healthy=1; break; fi
  sleep 1
done
if [[ -z "$healthy" ]]; then
  echo "no HTTPS /healthz answer within 30 s" >&2
  docker logs "$cid" >&2 || true
  exit 1
fi
# /healthz stays open on purpose: a liveness probe holds no session.
curl -skf "$base/healthz"
echo

# The guard, seen from outside: no session, no project.
status=$(curl -sk -o /dev/null -w '%{http_code}' "$base/api/version")
test "$status" = "401" || { echo "an unauthenticated /api/version answered $status, not 401"; exit 1; }

curl -skf -c "$jar" -X POST "$base/api/auth/login" \
  -H 'Content-Type: application/json' \
  -d "{\"password\":\"$smoke_password\"}" >/dev/null
grep -q knx_session "$jar" || { echo "login set no session cookie"; exit 1; }

status=$(curl -sk -b "$jar" -o /dev/null -w '%{http_code}' "$base/api/version")
test "$status" = "200" || { echo "an authenticated /api/version answered $status, not 200"; exit 1; }

created=$(curl -skf -b "$jar" -X POST "$base/api/project/new" \
  -H 'Content-Type: application/json' \
  -d '{"name":"Docker smoke","installationName":"Smoke installation","language":"en","groupAddressStyle":"ThreeLevel","discardChanges":false}')
echo "$created" | grep -q '"errors":0' || { echo "new project did not report 0 errors: $created"; exit 1; }

curl -skf -b "$jar" -X POST "$base/api/project/save-as" \
  -H 'Content-Type: application/json' \
  -d '{"path":"/data/smoke.knxdb"}' >/dev/null
test -s "$data_dir/smoke.knxdb"

curl -skf -b "$jar" -X POST "$base/api/project/new" \
  -H 'Content-Type: application/json' \
  -d '{"name":"Replacement","installationName":"Replacement","language":"en","groupAddressStyle":"Free","discardChanges":false}' >/dev/null

reopened=$(curl -skf -b "$jar" -X POST "$base/api/project/open" \
  -H 'Content-Type: application/json' \
  -d '{"path":"/data/smoke.knxdb"}')
echo "$reopened" | grep -q '"errors":0' || { echo "reopened project did not report 0 errors: $reopened"; exit 1; }
echo "$reopened" | grep -q '"name":"Smoke installation"' || { echo "reopened project lost its installation: $reopened"; exit 1; }

if [[ -n "$reference_project" ]]; then
  imported=$(curl -skf -b "$jar" -X POST "$base/api/project/import" \
    -H 'Content-Type: application/json' \
    -d "{\"path\":\"/data/$reference_name\"}")
  echo "$imported" | grep -q '"errors":0' || { echo "import did not report 0 errors: $imported"; exit 1; }
fi

echo "smoke test passed"
