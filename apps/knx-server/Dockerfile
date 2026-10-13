# syntax=docker/dockerfile:1

FROM node:22-alpine AS frontend
WORKDIR /src
COPY apps/knx-web/package.json apps/knx-web/package-lock.json ./
RUN npm ci
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
COPY --from=frontend /src/dist /app/frontend
ENV KNX_STATIC_DIR=/app/frontend
ENV KNX_DATA_DIR=/data
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
