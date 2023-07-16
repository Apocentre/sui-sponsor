FROM rust:1.71 as builder

RUN apt-get update \
    && apt-get install -y clang \
    && rm -rf /var/lib/apt/lists/*

RUN USER=root cargo new --bin api-dist
COPY ./api ./api-dist
COPY ./Cargo.lock ./api-dist
COPY ./common ./common
WORKDIR ./api-dist
RUN cargo build --release

FROM debian:bullseye-slim
ARG APP=/usr/src/app

EXPOSE 8000

ENV TZ=Etc/UTC

RUN apt-get update \
    && apt-get install -y ca-certificates tzdata libpq5 \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /api-dist/target/release/sui-sponsor-api ${APP}/api

USER $APP_USER
WORKDIR ${APP}

ENV ENV=production
CMD ["./api"]
