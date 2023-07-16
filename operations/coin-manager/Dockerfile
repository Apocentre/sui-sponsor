FROM rust:1.71 as builder

RUN apt-get update \
    && apt-get install -y ca-certificates libclang-dev \
    && rm -rf /var/lib/apt/lists/*

RUN USER=root cargo new --bin coin-manager-dist
COPY ./coin-manager ./coin-manager-dist
COPY ./Cargo.lock ./coin-manager-dist
COPY ./common ./common
COPY ./Cargo.lock ./common

WORKDIR ./coin-manager-dist
RUN cargo build --release

FROM debian:bookworm-slim
ARG APP=/usr/src/app

EXPOSE 8000

ENV TZ=Etc/UTC

RUN apt-get update \
    && apt-get install -y ca-certificates tzdata libpq5 \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /coin-manager-dist/target/release/sui-sponsor-coin-manager ${APP}/coin-manager

USER $APP_USER
WORKDIR ${APP}

ENV ENV=production
CMD ["./coin-manager"]
