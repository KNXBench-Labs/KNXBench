# syntax=docker/dockerfile:1

FROM node:22-alpine AS frontend
# Mirror the repository layout: the web build's typecheck reaches the shared
# Playwright temp helper in tools/ (tools/tests/test_docker_frontend_context.py).
WORKDIR /src/apps/knx-web
COPY apps/knx-web/package.json apps/knx-web/package-lock.json ./
RUN npm ci
COPY tools/browser_tmp.cjs tools/browser_tmp.d.cts /src/tools/
# The strict catalogue client imports the authoritative protocol as build input.
COPY docs/contracts/catalogue-jobs/wp043-protocol.schema.json docs/contracts/catalogue-jobs/wp043-artifact-records.schema.json docs/contracts/catalogue-jobs/wp043-protocol-policy.json /src/docs/contracts/catalogue-jobs/
COPY apps/knx-web/ ./
RUN npm run build

FROM rust:1.98-slim AS backend
# .dockerignore drops .git, so `knx-server --version` cannot ask git which
# commit this is. Pass it in: --build-arg KNX_BUILD_SHA=$(git rev-parse --short HEAD)
ARG KNX_BUILD_SHA=
ENV KNX_BUILD_SHA=$KNX_BUILD_SHA
WORKDIR /src
COPY . .
RUN cargo build --release -p knx-server

FROM debian:bookworm-slim AS runtime
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*
COPY --from=backend /src/target/release/knx-server /usr/local/bin/knx-server
COPY --from=frontend /src/apps/knx-web/dist /app/frontend
ENV KNX_STATIC_DIR=/app/frontend
ENV KNX_DATA_DIR=/data
# The product database (application programs, ~/.local/share/knx/products.sqlite
# outside a container) lives on the volume too, so a rebuilt or updated
# container keeps it (GitHub #21, ADR-0128). Saved projects also carry their own
# manufacturer files since then and reinstall them on open.
ENV XDG_DATA_HOME=/data/.knxbench-share
ENV KNX_PORT=8484
VOLUME /data
EXPOSE 8484
# Linux default run recipe: docker run --network host ... (no -p).
# Network mode is selected at container creation, not by the Dockerfile.
# `POST /api/bus/discover` sends KNXnet/IP multicast, which Docker's bridge
# does not carry onto the LAN. Through the bridge a tunnel gets
# no answer either, because the gateway replies to the private container
# address, unless KNX_TUNNEL_ROUTE_BACK=1 asks it to answer the packet's
# source. See docs/KNOWN_LIMITATIONS.md #79 and #155.
# With a password set the server speaks HTTPS on KNX_PORT, with a self-signed
# certificate kept in /data/.knxbench-tls (ADR-0088). KNX_TLS_CERT/KNX_TLS_KEY
# name your own PEM files instead; KNX_TLS=off returns to plain HTTP.
# knx-server runs as PID 1 and handles SIGTERM itself: `docker stop` refuses new
# bus jobs, lets open requests finish (5 s), disconnects a bus monitor's tunnel
# and exits with 0, well inside Docker's 10 s timeout. Only while a device
# download or address programming is running does it wait longer, up to
# KNX_STOP_WRITE_WAIT_SECS (default 120): stop with `docker stop -t 130` then.
# No init wrapper is needed.
ENTRYPOINT ["/usr/local/bin/knx-server"]
