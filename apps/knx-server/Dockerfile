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
ENV KNX_PORT=8080
VOLUME /data
EXPOSE 8080
# `POST /api/bus/discover` sends KNXnet/IP multicast. Docker's default bridge
# does not carry that request onto the LAN, so run this image with
# `--network host` on Linux when gateway discovery is needed. Project editing
# and manually entered unicast endpoints still work through the bridge. See
# docs/KNOWN_LIMITATIONS.md #79.
ENTRYPOINT ["/usr/local/bin/knx-server"]
