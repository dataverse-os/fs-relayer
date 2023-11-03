FROM rust:bullseye AS builder-rust

RUN apt update && apt install -y protobuf-compiler libclang-dev

COPY ./node /build/node
COPY ./crates /build/crates
WORKDIR /build/node
RUN cargo build --release --no-default-features -p dataverse-file-relayer

FROM debian:bullseye
RUN apt update && apt install -y ca-certificates

COPY --from=builder-rust /build/node/target/release/dataverse-file-relayer /usr/bin/file-relayer

ENTRYPOINT ["/usr/bin/file-relayer"]