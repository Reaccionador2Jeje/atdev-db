FROM rust:1.85-slim AS builder
WORKDIR /app
COPY Cargo.toml ./
COPY src ./src
RUN cargo build --release

FROM debian:bookworm-slim
RUN useradd --system --uid 10001 atdev && mkdir -p /data && chown -R atdev:atdev /data
COPY --from=builder /app/target/release/atdev-db /usr/local/bin/atdev-db
USER atdev
ENV ATDEV_DB_PATH=/data
ENV ATDEV_DB_BIND=0.0.0.0:8788
EXPOSE 8788
ENTRYPOINT ["atdev-db", "--server"]
