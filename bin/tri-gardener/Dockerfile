# tri-gardener — autonomous orchestrator service.
#
# Image runs `tri-gardener once --review` (or `--live` when the
# operator flips GARDENER_LIVE=true in Railway service variables).
# Cron is provided by Railway's scheduled-restart on the `gardener`
# service in Acc1 IGLA, ticking every hour at :15 UTC.
#
# Anchor: phi^2 + phi^-2 = 3.

FROM rust:1.95-slim AS builder
WORKDIR /build
COPY . .
RUN cargo build --release --bin tri-gardener --locked

FROM debian:bookworm-slim
RUN apt-get update \
 && apt-get install -y --no-install-recommends ca-certificates \
 && rm -rf /var/lib/apt/lists/*
COPY --from=builder /build/target/release/tri-gardener /usr/local/bin/tri-gardener
COPY bin/tri-gardener/queue.toml /etc/tri-gardener/queue.toml
COPY bin/tri-railway/plan21-manifest.toml /etc/tri-gardener/plan21-manifest.toml

# Default mode is the review window; operator promotes via env vars.
ENV GARDENER_MODE=review \
    RUST_LOG=info \
    TZ=UTC

# `tri-gardener once` performs a single tick and exits with code 0,
# letting Railway's restart policy supply the cron behaviour.
CMD ["/usr/local/bin/tri-gardener", "once", "--review"]
